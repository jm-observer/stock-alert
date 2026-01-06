//! 数据模型定义

use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 股票持仓信息
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct StockPosition {
    pub id: Option<i64>,
    pub code: String,
    pub name: String,
    pub buy_price: f64,
    pub buy_date: NaiveDate,
    pub current_price: f64,
    pub highest_price_since_buy: f64,
    /// 盈利阈值1级报警开关（已报警则不再报警）
    pub profit_threshold_level1_alerted: bool,
    /// 盈利阈值2级报警开关（已报警则不再报警）
    pub profit_threshold_level2_alerted: bool,
    /// 亏损阈值报警开关（已报警则不再报警）
    pub loss_threshold_alerted: bool,
    /// 盈利回撤过半报警开关（已报警则不再报警）
    pub profit_drawdown_half_alerted: bool,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

impl StockPosition {
    /// 计算盈亏比例
    pub fn pnl_ratio(&self) -> f64 {
        (self.current_price - self.buy_price) / self.buy_price
    }

    /// 计算最高盈利比例
    pub fn max_profit_ratio(&self) -> f64 {
        (self.highest_price_since_buy - self.buy_price) / self.buy_price
    }

    /// 更新当前价格，同时更新最高价
    pub fn update_price(&mut self, price: f64) {
        self.current_price = price;
        if price > self.highest_price_since_buy {
            self.highest_price_since_buy = price;
        }
    }
}

/// 行情数据
#[derive(Debug, Clone)]
pub struct Quote {
    pub code: String,
    pub price: f64,
    pub volume: Option<f64>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// 告警规则类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlertRule {
    /// 盈利阈值提醒（整数百分比，如10表示10%）
    ProfitThreshold(i32),
    /// 亏损阈值提醒（整数百分比，如10表示10%）
    LossThreshold(i32),
    /// 盈利回撤过半提醒
    ProfitDrawdownHalf,
}

/// 告警事件
#[derive(Debug, Clone)]
pub struct AlertEvent {
    pub code: String,
    pub name: String,
    pub rule: AlertRule,
    pub current_price: f64,
    pub pnl_ratio: f64,
    pub max_profit_ratio: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

