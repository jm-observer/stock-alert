//! 应用主逻辑

use crate::config::Config;
use crate::engine::AlertEngine;
use crate::models::{Quote, StockPosition};
use crate::notify::Notifier;
use crate::quote::{create_quote_source, QuoteSource};
use crate::scheduler::TradingScheduler;
use crate::storage::Storage;
use anyhow::Result;
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{mpsc, Mutex};

/// 应用主结构
#[derive(Clone)]
pub struct App {
    config: Config,
    storage: Arc<Storage>,
    scheduler: TradingScheduler,
    engine: Arc<Mutex<AlertEngine>>,
    notifier: Arc<Notifier>,
    quote_source: Arc<dyn QuoteSource>,
    is_test: bool,
}

impl App {
    pub async fn new(config: Config) -> Result<Self> {
        let storage = Arc::new(Storage::new(&config.db_path).await?);
        let scheduler = TradingScheduler::new(config.trading_hours.clone(), &config.timezone)?;
        let engine = Arc::new(Mutex::new(AlertEngine::new(config.alert.clone())));
        let notifier = Arc::new(Notifier::new(config.notify.clone())?);
        let is_test = config.quote_source.is_test();
        let quote_source = create_quote_source(config.quote_source.clone())?;

        Ok(Self {
            config,
            storage,
            scheduler,
            engine,
            notifier,
            quote_source,
            is_test,
        })
    }

    /// 运行应用主循环
    pub async fn run(&self) -> Result<()> {
        info!("股票价格提醒程序启动");

        loop {
            // 等待交易时段 todo
            // self.scheduler.wait_until_trading_hours().await?;

            // 加载启用的股票列表
            let positions = self.storage.list_positions().await?;
            if positions.is_empty() {
                warn!("没有启用的股票，等待交易时段结束");
                self.scheduler.wait_until_trading_hours_end().await?;
                continue;
            } else {
                info!("监控股票列表:");
                for pos in &positions {
                    // 构建报警状态字符串
                    let mut alert_status = Vec::new();
                    if pos.profit_threshold_level1_alerted {
                        alert_status.push("盈1级✓");
                    } else {
                        alert_status.push("盈1级✗");
                    }
                    if pos.profit_threshold_level2_alerted {
                        alert_status.push("盈2级✓");
                    } else {
                        alert_status.push("盈2级✗");
                    }
                    if pos.loss_threshold_alerted {
                        alert_status.push("亏✓");
                    } else {
                        alert_status.push("亏✗");
                    }
                    if pos.profit_drawdown_half_alerted {
                        alert_status.push("回撤✓");
                    } else {
                        alert_status.push("回撤✗");
                    }
                    let alert_status_str = alert_status.join(" ");
                    
                    info!(
                        "  - {} ({}) | 买入价: {:.2} | 当前价: {:.2} | 最高价: {:.2} | 盈亏比例: {:.2}% | 报警: {}",
                        pos.name,
                        pos.code,
                        pos.buy_price,
                        pos.current_price,
                        pos.highest_price_since_buy,
                        pos.pnl_ratio() * 100.0,
                        alert_status_str
                    );
                }
            }

            info!("开始监控 {} 只股票", positions.len());
            // 获取股票代码列表
            let codes: Vec<String> = positions.iter().map(|p| p.code.clone()).collect();


            let mut is_trading_hours = true;
            loop {
                if self.scheduler.is_trading_hours()? || self.is_test {
                    if !is_trading_hours {
                        // is_trading_hours = true;
                        info!("交易时段开始，继续监控");
                    }
                    break;
                }
                if is_trading_hours {
                    is_trading_hours = false;
                    info!("停止监控，等待交易时段开始");
                }
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            }
            
            // 创建通道用于接收行情数据
            let (tx, mut rx) = mpsc::unbounded_channel();
            
            // 为每只股票创建订阅
            for code in &codes {
                let quote_source = self.quote_source.clone();
                let code_clone = code.clone();
                let tx_clone = tx.clone();
                
                tokio::spawn(async move {
                    if let Err(e) = quote_source.subscribe(code_clone.clone(), tx_clone).await {
                        error!("订阅股票 {} 失败: {}", code_clone, e);
                    }
                });
            }
            
            // 维护股票持仓的本地缓存
            let mut positions_cache: HashMap<String, StockPosition> =
                positions.into_iter().map(|p| (p.code.clone(), p)).collect();

            // 处理行情更新
            let mut last_update_time = Instant::now();
            let update_interval = std::time::Duration::from_secs(5); // 每5秒批量更新一次数据库

            loop {
                tokio::select! {
                    // 接收行情数据
                    quote_result = rx.recv() => {
                        match quote_result {
                            Some(Ok(quote)) => {
                                self.handle_quote(quote, &mut positions_cache, &mut last_update_time, update_interval).await?;
                            }
                            Some(Err(e)) => {
                                error!("接收行情数据错误: {}", e);
                            }
                            None => {
                                warn!("行情通道关闭，准备重连");
                                break;
                            }
                        }
                    }
                    // 超时检查交易时段
                    _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {
                        // 每分钟检查一次交易时段
                        if !self.scheduler.is_trading_hours()? && !self.is_test {
                            info!("交易时段结束，停止监控");
                            break;
                        }
                    }
                }
            }

            // 监控循环结束前，全量更新所有持仓数据到数据库
            let updates: Vec<StockPosition> = positions_cache
                .values()
                .cloned()
                .collect();

            if !updates.is_empty() {
                info!("监控循环结束，全量更新数据库: {} 条记录", updates.len());
                if let Err(e) = self.storage.update_positions(&updates).await {
                    error!("全量更新数据库失败: {}", e);
                } else {
                    info!("全量更新数据库成功: {} 条记录", updates.len());
                }
            }

            // 交易时段结束，等待下一个交易时段
            self.scheduler.wait_until_trading_hours_end().await?;
        }
    }

    /// 处理行情数据
    async fn handle_quote(
        &self,
        quote: Quote,
        positions_cache: &mut HashMap<String, StockPosition>,
        last_update_time: &mut Instant,
        update_interval: std::time::Duration,
    ) -> Result<()> {
        // 记录收到的行情数据
        debug!(
            "\n收到行情: {} 价格={:.2} 时间={} 交易量={:?}",
            quote.code,
            quote.price,
            quote.timestamp.format("%Y-%m-%d %H:%M:%S"),
            quote.volume
        );

        // 更新缓存中的价格
        if let Some(position) = positions_cache.get_mut(&quote.code) {
            let old_price = position.current_price;
            position.update_price(quote.price);

            info!(
                "更新价格: {} {} -> {:.2} (盈亏比例: {:.2}%)",
                position.name,
                old_price,
                quote.price,
                position.pnl_ratio() * 100.0
            );

            // 检查告警
            let mut engine = self.engine.lock().await;
            let alerts = engine.process_update(position);

            // 处理告警
            if !alerts.is_empty() {
                info!("触发告警: {} 共 {} 条", quote.code, alerts.len());
            }
            for alert in alerts {
                info!(
                    "告警: {} {} 规则={:?} 当前价={:.2} 盈亏比例={:.2}%",
                    alert.code,
                    alert.name,
                    alert.rule,
                    alert.current_price,
                    alert.pnl_ratio * 100.0
                );
                
                // 标记对应的报警开关
                match alert.rule {
                    crate::models::AlertRule::ProfitThreshold(threshold) => {
                        // 根据阈值判断是1级还是2级
                        if let Some(first_threshold) = self.config.alert.profit_thresholds.first() {
                            if threshold == *first_threshold {
                                position.profit_threshold_level1_alerted = true;
                            } else if self.config.alert.profit_thresholds.len() > 1 
                                && threshold == self.config.alert.profit_thresholds[1] {
                                position.profit_threshold_level2_alerted = true;
                            }
                        }
                    }
                    crate::models::AlertRule::LossThreshold(_threshold) => {
                        position.loss_threshold_alerted = true;
                    }
                    crate::models::AlertRule::ProfitDrawdownHalf => {
                        position.profit_drawdown_half_alerted = true;
                    }
                }
                
                if let Err(e) = self.notifier.handle_alert(&alert).await {
                    error!("处理告警失败: {}", e);
                }
            }
        } else {
            debug!("未找到持仓: {} (可能未启用监控)", quote.code);
        }

        // 批量全量更新数据库（避免频繁写入）
        if last_update_time.elapsed() >= update_interval {
            let updates: Vec<StockPosition> = positions_cache
                .values()
                .cloned()
                .collect();

            if !updates.is_empty() {
                debug!("批量全量更新数据库: {} 条记录", updates.len());
                if let Err(e) = self.storage.update_positions(&updates).await {
                    error!("批量全量更新失败: {}", e);
                } else {
                    debug!("批量全量更新数据库成功: {} 条记录", updates.len());
                }
            }

            *last_update_time = Instant::now();
        }

        Ok(())
    }
}

/// 运行应用
pub async fn run() -> Result<()> {
    // 加载配置
    let config_path = std::path::PathBuf::from("config.toml");
    let config = Config::load_or_default(&config_path)?;

    // 如果配置文件不存在，保存默认配置
    if !config_path.exists() {
        config.save(&config_path)?;
        info!("已创建默认配置文件: {}", config_path.display());
    }

    // 创建并运行应用
    let app = App::new(config).await?;
    app.run().await?;

    Ok(())
}
