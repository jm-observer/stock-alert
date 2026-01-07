//! 告警引擎模块

use crate::config::AlertConfig;
use crate::models::{AlertEvent, AlertRule, StockPosition};

/// 告警引擎
pub struct AlertEngine {
    config: AlertConfig,
    // states: HashMap<String, AlertState>,
}

impl AlertEngine {
    pub fn new(config: AlertConfig) -> Self {
        Self {
            config,
            // states: HashMap::new(),
        }
    }

    /// 初始化股票状态
    // pub fn init_stock(&mut self, code: &str) {
    //     self.states.entry(code.to_string()).or_insert_with(AlertState::new);
    // }

    /// 处理价格更新，返回触发的告警事件列表
    pub fn process_update(
        &mut self,
        position: &mut StockPosition
    ) -> Vec<AlertEvent> {
        let code = &position.code;
        // let state = self.states.entry(code.clone()).or_insert_with(AlertState::new);

        let pnl_ratio = position.pnl_ratio();
        let max_profit_ratio = position.max_profit_ratio();
        let current_price = position.current_price;

        let mut alerts = Vec::new();

        let profit_threshold_1 = self.config.profit_thresholds[1] as f64 / 100.0;
        let profit_threshold_0 = self.config.profit_thresholds[0] as f64 / 100.0;
        let loss_threshold = self.config.loss_threshold as f64 / 100.0;

        if pnl_ratio >= profit_threshold_1 && !position.profit_threshold_level2_alerted {
            alerts.push(AlertEvent {
                code: code.clone(),
                name: position.name.clone(),
                rule: AlertRule::ProfitThreshold(self.config.profit_thresholds[1]),
                current_price,
                pnl_ratio: pnl_ratio,
                max_profit_ratio,
                timestamp: chrono::Utc::now(),
            });
            position.profit_threshold_level2_alerted = true;
        } else if pnl_ratio >= profit_threshold_0 && !position.profit_threshold_level1_alerted {
            alerts.push(AlertEvent {
                code: code.clone(),
                name: position.name.clone(),
                rule: AlertRule::ProfitThreshold(self.config.profit_thresholds[0]),
                current_price,
                pnl_ratio: pnl_ratio,
                max_profit_ratio,
                timestamp: chrono::Utc::now(),
            });
            position.profit_threshold_level1_alerted = true;
        } else if (-pnl_ratio) >= loss_threshold && !position.loss_threshold_alerted {
            alerts.push(AlertEvent {
                code: code.clone(),
                name: position.name.clone(),
                rule: AlertRule::LossThreshold(self.config.loss_threshold),
                current_price,
                pnl_ratio: -pnl_ratio,
                max_profit_ratio,
                timestamp: chrono::Utc::now(),
            });
            position.loss_threshold_alerted = true;
        }

        // 处理盈利回撤过半提醒
        if position.profit_threshold_level2_alerted && !position.profit_drawdown_half_alerted {
            let max_profit = max_profit_ratio;
            let current_pnl = pnl_ratio;
                let half_max_profit = max_profit * 0.5;
                let current_above_half = current_pnl > half_max_profit;

                // 穿越触发：从上方穿越到下方
                if !current_above_half {
                    alerts.push(AlertEvent {
                        code: code.clone(),
                        name: position.name.clone(),
                        rule: AlertRule::ProfitDrawdownHalf,
                        current_price,
                        pnl_ratio: current_pnl,
                        max_profit_ratio: max_profit,
                        timestamp: chrono::Utc::now(),
                    });
                    position.profit_drawdown_half_alerted = true;
                    log::info!("触发盈利回撤过半提醒: {} - 当前: {:.2}%, 最高: {:.2}%", 
                        code, current_pnl * 100.0, max_profit * 100.0);
                }
        } else {
            log::debug!("盈利回撤过半提醒已触发过，跳过: {}", code);
        }

        alerts
    }
}

