//! Tauri模式下的应用主逻辑

use crate::commands::{get_config_path, get_db_path};
use crate::config::Config;
use crate::engine::AlertEngine;
use crate::models::{Quote, StockPosition};
use crate::notify::Notifier;
use crate::quote::{create_quote_source, QuoteSource};
use crate::scheduler::TradingScheduler;
use crate::storage::Storage;
use anyhow::Result;
use log::{debug, error, info, warn};
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc};
use tokio::sync::RwLock;
use std::time::Instant;
use tauri::AppHandle;
use tauri::Emitter;
use tokio::sync::{mpsc, Mutex};

/// 股票状态更新事件
#[derive(Debug, Clone, Serialize)]
pub struct StockUpdateEvent {
    pub position: StockPosition,
}

#[derive(Clone)]
/// Tauri模式下的应用主结构
pub struct TauriApp {
    app_handle: AppHandle,
    config: Arc<RwLock<Config>>,
    storage: Arc<Storage>,
    scheduler: TradingScheduler,
    engine: Arc<Mutex<AlertEngine>>,
    notifier: Arc<Notifier>,
    quote_source: Arc<dyn QuoteSource>,
    is_test: bool,
    tx: mpsc::UnboundedSender<Result<Quote>>,
    positions: Arc<RwLock<HashMap<String, StockPosition>>>,
    config_path: PathBuf,
}

impl TauriApp {
    pub async fn new(
        app_handle: AppHandle,
        tx: mpsc::UnboundedSender<Result<Quote>>,
    ) -> Result<Self> {
        // 获取配置和数据库路径
        let config_path = get_config_path(&app_handle)?;
        let db_path = get_db_path(&app_handle)?;

        // 加载配置
        let config = Config::load_or_default(&config_path)?;

        // 如果配置文件不存在，保存默认配置
        if !config_path.exists() {
            config.save(&config_path)?;
            info!("已创建默认配置文件: {}", config_path.display());
        }

        // 创建存储，使用Tauri应用数据目录
        let mut config_with_tauri_path = config.clone();
        config_with_tauri_path.db_path = db_path.clone();

        let storage = Arc::new(Storage::new(&db_path).await?);
        let scheduler = TradingScheduler::new(config.trading_hours.clone(), &config.timezone)?;
        let engine = Arc::new(Mutex::new(AlertEngine::new(config.alert.clone())));
        let notifier = Arc::new(Notifier::new(config.notify.clone())?);
        let is_test = config.quote_source.is_test();
        let quote_source = create_quote_source(config.quote_source.clone())?;

        
        let positions = storage.list_positions().await?;
        let positions_cache: HashMap<String, StockPosition> =
        positions.into_iter().map(|p| (p.code.clone(), p)).collect();
        Ok(Self {
            tx,
            app_handle,
            config: Arc::new(RwLock::new(config)),
            storage,
            scheduler,
            engine,
            notifier,
            quote_source,
            is_test,
            positions: Arc::new(RwLock::new(positions_cache)),
            config_path,
        })
    }

    /// 运行应用主循环
    pub async fn run(&self, mut rx: mpsc::UnboundedReceiver<Result<Quote>>) -> Result<()> {
        info!("股票价格提醒程序启动（Tauri模式）");

        loop {
            let mut is_trading_hours = true;
            loop {
                if self.scheduler.is_trading_hours()? || self.is_test {
                    if !is_trading_hours {
                        is_trading_hours = true;
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
            // 加载启用的股票列表
            let positions = self.positions.read().await;
            if !positions.is_empty() {
                info!("监控股票列表:");
                for pos in positions.values() {
                    info!(
                        "  - {} ({}) | 买入价: {:.2} | 当前价: {} | 最高价: {} | 盈亏: {}",
                        pos.name,
                        pos.code,
                        pos.buy_price,
                        pos.current_price,
                        pos.highest_price_since_buy,
                        pos.pnl_ratio(),
                    );
                }
                info!("开始监控 {} 只股票", positions.len());
                // 获取股票代码列表
                let codes: Vec<String> = positions.keys().map(|p| p.clone()).collect();

                for code in &codes {
                    let quote_source = self.quote_source.clone();
                    let code_clone = code.clone();
                    let tx_clone = self.tx.clone();

                    tokio::spawn(async move {
                        if let Err(e) = quote_source.subscribe(code_clone.clone(), tx_clone).await {
                            error!("订阅股票 {} 失败: {}", code_clone, e);
                        }
                    });
                }
            }
            drop(positions);

            // 创建通道用于接收行情数据
            // let (tx, mut rx) = mpsc::unbounded_channel();

            // 为每只股票创建订阅

            // 维护股票持仓的本地缓存
            

            // 处理行情更新
            let mut last_update_time = Instant::now();

            loop {
                tokio::select! {
                    // 接收行情数据
                    quote_result = rx.recv() => {
                        match quote_result {
                            Some(Ok(quote)) => {
                                let mut positions_cache = self.positions.write().await;
                                self.handle_quote(quote, &mut positions_cache, &mut last_update_time).await?;
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
                        // 监控循环结束前，全量更新所有持仓数据到数据库
                        let updates: Vec<StockPosition> = self.positions.read().await
                            .values()
                            .cloned()
                            .collect();

                        if !updates.is_empty() {
                            let storage_clone = self.storage.clone();
                            tokio::spawn(async move {
                                info!("监控循环结束，全量更新数据库: {} 条记录", updates.len());
                                if let Err(e) = storage_clone.update_positions(&updates).await {
                                    error!("全量更新数据库失败: {}", e);
                                } else {
                                    info!("全量更新数据库成功: {} 条记录", updates.len());
                                }
                            });
                        }
                        // 每分钟检查一次交易时段
                        if !self.scheduler.is_trading_hours()? && !self.is_test {
                            info!("交易时段结束，停止监控");
                            break;
                        }
                    }
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
            *last_update_time = Instant::now();
            info!(
                "更新价格: {} {:.2} -> {:.2} (盈亏比例: {:.2}%)",
                position.name,
                old_price,
                quote.price,
                position.pnl_ratio() * 100.0
            );

            // 发送更新事件到前端
            let update_event = StockUpdateEvent {
                position: position.clone(),
            };
            if let Err(e) = self.app_handle.emit("stock-update", &update_event) {
                error!("发送股票更新事件失败: {}", e);
            }

            // 检查告警
            let mut engine = self.engine.lock().await;
            let alerts = engine.process_update(position);

            // 处理告警
            if !alerts.is_empty() {
                info!("触发告警: {} 共 {} 条", quote.code, alerts.len());
            }
            let config = self.config.read().await.clone();
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
                        if let Some(first_threshold) = config.alert.profit_thresholds.first() {
                            if threshold == *first_threshold {
                                position.profit_threshold_level1_alerted = true;
                            } else if config.alert.profit_thresholds.len() > 1
                                && threshold == config.alert.profit_thresholds[1]
                            {
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
        // if last_update_time.elapsed() >= update_interval {
        //     let updates: Vec<StockPosition> = positions_cache.values().cloned().collect();

        //     if !updates.is_empty() {
        //         debug!("批量全量更新数据库: {} 条记录", updates.len());
        //         if let Err(e) = self.storage.update_positions(&updates).await {
        //             error!("批量全量更新失败: {}", e);
        //         } else {
        //             debug!("批量全量更新数据库成功: {} 条记录", updates.len());
        //         }
        //     }

        //     *last_update_time = Instant::now();
        // }

        Ok(())
    }

    /// 添加股票持仓
    pub async fn add_stock(
        &self,
        code: String,
        name: String,
        buy_price: f64,
        buy_date: String,
    ) -> Result<i64> {
        let buy_date = chrono::NaiveDate::parse_from_str(&buy_date, "%Y-%m-%d")?;

        let mut position = StockPosition {
            id: None,
            code: code.clone(),
            name,
            buy_price,
            buy_date,
            current_price: buy_price,
            highest_price_since_buy: buy_price,
            profit_threshold_level1_alerted: false,
            profit_threshold_level2_alerted: false,
            loss_threshold_alerted: false,
            profit_drawdown_half_alerted: false,
            created_at: None,
            updated_at: None,
        };

        let id = self.storage
            .add_position(&position)
            .await?;

        position.id = Some(id);
        self.quote_source
            .subscribe(code.clone(), self.tx.clone())
            .await?;
        let mut positions_cache = self.positions.write().await;
        positions_cache.insert(code, position);

        Ok(id)
    }

    pub async fn delete_stock(&self, code: String) -> Result<bool> {
        self.storage
            .delete_position(&code)
            .await?;
        self.positions.write().await.remove(&code);
        Ok(true)
    }
    
    /// 更新告警阈值
    pub async fn update_alert_thresholds(
        &self,
        profit_thresholds: [i32; 2],
        loss_threshold: i32,
    ) -> Result<()> {
        let mut config = self.config.write().await;
        config.alert.profit_thresholds = profit_thresholds;
        config.alert.loss_threshold = loss_threshold;
        let config_clone = config.clone();
        drop(config);
        self.save_config(config_clone).await
    }

    /// 更新声音文件配置
    pub async fn update_sound_file(
        &self,
        sound_file: Option<String>,
    ) -> Result<()> {
        let mut config = self.config.write().await;
        config.notify.sound_file = sound_file.map(PathBuf::from);
        let config_clone = config.clone();
        drop(config);
        self.save_config(config_clone).await
    }

    pub async fn save_config(&self, config: Config) -> Result<()> {
        config
            .save(&self.config_path)
    }

}
