//! 数据持久化模块

use crate::models::StockPosition;
use crate::config::ThresholdType;
use anyhow::Result;
use chrono::{NaiveDateTime, Local};
use sqlx::{sqlite::SqlitePool, Row};
use std::path::PathBuf;
use log::{debug, info};
use serde_json;

/// 数据库存储管理器
pub struct Storage {
    pool: SqlitePool,
}

impl Storage {
    /// 创建或打开数据库
    pub async fn new(db_path: &PathBuf) -> Result<Self> {
        let url = format!("sqlite:{}?mode=rwc", db_path.display());
        let pool = SqlitePool::connect(&url).await?;


        // 创建表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS stock_positions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                code TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                buy_price REAL NOT NULL,
                current_price REAL,
                highest_price_since_buy REAL,
                profit_threshold_level1_alerted INTEGER NOT NULL DEFAULT 0,
                profit_threshold_level2_alerted INTEGER NOT NULL DEFAULT 0,
                loss_threshold_alerted INTEGER NOT NULL DEFAULT 0,
                profit_drawdown_half_alerted INTEGER NOT NULL DEFAULT 0,
                profit_threshold1 TEXT NOT NULL DEFAULT '{"type":"Ratio","value":10}',
                profit_threshold2 TEXT NOT NULL DEFAULT '{"type":"Ratio","value":20}',
                loss_threshold TEXT NOT NULL DEFAULT '{"type":"Ratio","value":10}',
                created_at TEXT,
                updated_at TEXT
            )
            "#,
        )
        .execute(&pool)
        .await?;

        // 迁移旧数据：如果表已存在但没有新字段，添加新字段
        // 先尝试删除旧的 profit_threshold_alerted 字段（如果存在）
        let _ = sqlx::query(
            r#"
            ALTER TABLE stock_positions 
            ADD COLUMN profit_threshold_level1_alerted INTEGER NOT NULL DEFAULT 0
            "#,
        )
        .execute(&pool)
        .await;
        
        let _ = sqlx::query(
            r#"
            ALTER TABLE stock_positions 
            ADD COLUMN profit_threshold_level2_alerted INTEGER NOT NULL DEFAULT 0
            "#,
        )
        .execute(&pool)
        .await;
        
        let _ = sqlx::query(
            r#"
            ALTER TABLE stock_positions 
            ADD COLUMN loss_threshold_alerted INTEGER NOT NULL DEFAULT 0
            "#,
        )
        .execute(&pool)
        .await;
        
        let _ = sqlx::query(
            r#"
            ALTER TABLE stock_positions 
            ADD COLUMN profit_drawdown_half_alerted INTEGER NOT NULL DEFAULT 0
            "#,
        )
        .execute(&pool)
        .await;
        
        // 添加阈值字段（如果不存在）
        // SQLite 的 ALTER TABLE ADD COLUMN 不支持 NOT NULL DEFAULT，需要分两步
        let _ = sqlx::query(
            r#"
            ALTER TABLE stock_positions 
            ADD COLUMN profit_threshold1 TEXT
            "#,
        )
        .execute(&pool)
        .await;
        
        let _ = sqlx::query(
            r#"
            ALTER TABLE stock_positions 
            ADD COLUMN profit_threshold2 TEXT
            "#,
        )
        .execute(&pool)
        .await;
        
        let _ = sqlx::query(
            r#"
            ALTER TABLE stock_positions 
            ADD COLUMN loss_threshold TEXT
            "#,
        )
        .execute(&pool)
        .await;
        
        // 为现有记录设置默认值
        let _ = sqlx::query(
            r#"
            UPDATE stock_positions 
            SET profit_threshold1 = '{"type":"Ratio","value":10}'
            WHERE profit_threshold1 IS NULL
            "#,
        )
        .execute(&pool)
        .await;
        
        let _ = sqlx::query(
            r#"
            UPDATE stock_positions 
            SET profit_threshold2 = '{"type":"Ratio","value":20}'
            WHERE profit_threshold2 IS NULL
            "#,
        )
        .execute(&pool)
        .await;
        
        let _ = sqlx::query(
            r#"
            UPDATE stock_positions 
            SET loss_threshold = '{"type":"Ratio","value":10}'
            WHERE loss_threshold IS NULL
            "#,
        )
        .execute(&pool)
        .await;

        info!("数据库初始化完成: {}", db_path.display());
        Ok(Self { pool })
    }

    /// 获取所有股票持仓
    pub async fn list_positions(&self) -> Result<Vec<StockPosition>> {
        let rows = sqlx::query(
            r#"
            SELECT id, code, name, buy_price, current_price, 
                   highest_price_since_buy, profit_threshold_level1_alerted, 
                   profit_threshold_level2_alerted, loss_threshold_alerted, 
                   profit_drawdown_half_alerted, profit_threshold1, profit_threshold2, 
                   loss_threshold, created_at, updated_at
            FROM stock_positions
            ORDER BY code
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut positions = Vec::new();
        for row in rows {
            // 使用 try_get 处理可能不存在的列（旧数据）
            let profit_threshold1_str: String = row.try_get("profit_threshold1")
                .unwrap_or_else(|_| r#"{"type":"Ratio","value":10}"#.to_string());
            let profit_threshold2_str: String = row.try_get("profit_threshold2")
                .unwrap_or_else(|_| r#"{"type":"Ratio","value":20}"#.to_string());
            let loss_threshold_str: String = row.try_get("loss_threshold")
                .unwrap_or_else(|_| r#"{"type":"Ratio","value":10}"#.to_string());
            
            let profit_threshold1: ThresholdType = serde_json::from_str(&profit_threshold1_str)
                .unwrap_or(ThresholdType::Ratio(10));
            let profit_threshold2: ThresholdType = serde_json::from_str(&profit_threshold2_str)
                .unwrap_or(ThresholdType::Ratio(20));
            let loss_threshold: ThresholdType = serde_json::from_str(&loss_threshold_str)
                .unwrap_or(ThresholdType::Ratio(10));
            
            positions.push(StockPosition {
                id: row.get::<Option<i64>, _>("id"),
                code: row.get("code"),
                name: row.get("name"),
                buy_price: row.get("buy_price"),
                current_price: row.get("current_price"),
                highest_price_since_buy: row.get("highest_price_since_buy"),
                profit_threshold_level1_alerted: row.get::<i64, _>("profit_threshold_level1_alerted") != 0,
                profit_threshold_level2_alerted: row.get::<i64, _>("profit_threshold_level2_alerted") != 0,
                loss_threshold_alerted: row.get::<i64, _>("loss_threshold_alerted") != 0,
                profit_drawdown_half_alerted: row.get::<i64, _>("profit_drawdown_half_alerted") != 0,
                profit_threshold1: profit_threshold1,
                profit_threshold2: profit_threshold2,
                loss_threshold: loss_threshold,
                created_at: row
                    .get::<Option<String>, _>("created_at")
                    .and_then(|s| NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S").ok()),
                updated_at: row
                    .get::<Option<String>, _>("updated_at")
                    .and_then(|s| NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S").ok()),
            });
        }

        Ok(positions)
    }

    /// 添加股票持仓
    pub async fn add_position(&self, position: &StockPosition) -> Result<i64> {
        let now = Local::now().naive_local();
        let result = sqlx::query(
            r#"
            INSERT INTO stock_positions 
            (code, name, buy_price, current_price, highest_price_since_buy, 
             profit_threshold_level1_alerted, profit_threshold_level2_alerted, 
             loss_threshold_alerted, profit_drawdown_half_alerted, 
             profit_threshold1, profit_threshold2, loss_threshold,
             created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&position.code)
        .bind(&position.name)
        .bind(position.buy_price)
        .bind(position.current_price)
        .bind(position.highest_price_since_buy)
        .bind(if position.profit_threshold_level1_alerted { 1 } else { 0 })
        .bind(if position.profit_threshold_level2_alerted { 1 } else { 0 })
        .bind(if position.loss_threshold_alerted { 1 } else { 0 })
        .bind(if position.profit_drawdown_half_alerted { 1 } else { 0 })
        .bind(serde_json::to_string(&position.profit_threshold1).unwrap_or_default())
        .bind(serde_json::to_string(&position.profit_threshold2).unwrap_or_default())
        .bind(serde_json::to_string(&position.loss_threshold).unwrap_or_default())
        .bind(now.format("%Y-%m-%d %H:%M:%S").to_string())
        .bind(now.format("%Y-%m-%d %H:%M:%S").to_string())
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// 删除股票持仓
    pub async fn delete_position(&self, code: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM stock_positions WHERE code = ?")
            .bind(code)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 更新股票持仓（买入价）
    pub async fn update_position(&self, code: &str, buy_price: f64) -> Result<bool> {
        let now = Local::now().naive_local();
        let result = sqlx::query(
            r#"
            UPDATE stock_positions 
            SET buy_price = ?, updated_at = ?
            WHERE code = ?
            "#,
        )
        .bind(buy_price)
        .bind(now.format("%Y-%m-%d %H:%M:%S").to_string())
        .bind(code)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 更新股票持仓信息（包括阈值）
    pub async fn update_position_full(
        &self,
        code: &str,
        buy_price: f64,
        profit_threshold1: &ThresholdType,
        profit_threshold2: &ThresholdType,
        loss_threshold: &ThresholdType,
    ) -> Result<bool> {
        let now = Local::now().naive_local();
        let result = sqlx::query(
            r#"
            UPDATE stock_positions 
            SET buy_price = ?, profit_threshold1 = ?, profit_threshold2 = ?, 
                loss_threshold = ?, profit_threshold_level1_alerted = 0,
                profit_threshold_level2_alerted = 0, loss_threshold_alerted = 0,
                updated_at = ?
            WHERE code = ?
            "#,
        )
        .bind(buy_price)
        .bind(serde_json::to_string(profit_threshold1).unwrap_or_default())
        .bind(serde_json::to_string(profit_threshold2).unwrap_or_default())
        .bind(serde_json::to_string(loss_threshold).unwrap_or_default())
        .bind(now.format("%Y-%m-%d %H:%M:%S").to_string())
        .bind(code)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 全量更新股票持仓信息（批量）
    pub async fn update_positions(&self, positions: &[StockPosition]) -> Result<()> {
        let now = Local::now().naive_local();
        let mut tx = self.pool.begin().await?;

        for position in positions {
            sqlx::query(
                r#"
                UPDATE stock_positions 
                SET current_price = ?, highest_price_since_buy = ?, 
                    profit_threshold_level1_alerted = ?, profit_threshold_level2_alerted = ?, 
                    loss_threshold_alerted = ?, profit_drawdown_half_alerted = ?,
                    profit_threshold1 = ?, profit_threshold2 = ?, loss_threshold = ?,
                    updated_at = ?
                WHERE code = ?
                "#,
            )
            .bind(position.current_price)
            .bind(position.highest_price_since_buy)
            .bind(if position.profit_threshold_level1_alerted { 1 } else { 0 })
            .bind(if position.profit_threshold_level2_alerted { 1 } else { 0 })
            .bind(if position.loss_threshold_alerted { 1 } else { 0 })
            .bind(if position.profit_drawdown_half_alerted { 1 } else { 0 })
            .bind(serde_json::to_string(&position.profit_threshold1).unwrap_or_default())
            .bind(serde_json::to_string(&position.profit_threshold2).unwrap_or_default())
            .bind(serde_json::to_string(&position.loss_threshold).unwrap_or_default())
            .bind(now.format("%Y-%m-%d %H:%M:%S").to_string())
            .bind(&position.code)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        debug!("批量全量更新了 {} 只股票的持仓信息", positions.len());
        Ok(())
    }

    /// 获取单个股票持仓
    pub async fn get_position(&self, code: &str) -> Result<Option<StockPosition>> {
        let row = sqlx::query(
            r#"
            SELECT id, code, name, buy_price, current_price, 
                   highest_price_since_buy, profit_threshold_level1_alerted, 
                   profit_threshold_level2_alerted, loss_threshold_alerted, 
                   profit_drawdown_half_alerted, profit_threshold1, profit_threshold2, 
                   loss_threshold, created_at, updated_at
            FROM stock_positions
            WHERE code = ?
            "#,
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            // 使用 try_get 处理可能不存在的列（旧数据）
            let profit_threshold1_str: String = row.try_get("profit_threshold1")
                .unwrap_or_else(|_| r#"{"type":"Ratio","value":10}"#.to_string());
            let profit_threshold2_str: String = row.try_get("profit_threshold2")
                .unwrap_or_else(|_| r#"{"type":"Ratio","value":20}"#.to_string());
            let loss_threshold_str: String = row.try_get("loss_threshold")
                .unwrap_or_else(|_| r#"{"type":"Ratio","value":10}"#.to_string());
            
            let profit_threshold1: ThresholdType = serde_json::from_str(&profit_threshold1_str)
                .unwrap_or(ThresholdType::Ratio(10));
            let profit_threshold2: ThresholdType = serde_json::from_str(&profit_threshold2_str)
                .unwrap_or(ThresholdType::Ratio(20));
            let loss_threshold: ThresholdType = serde_json::from_str(&loss_threshold_str)
                .unwrap_or(ThresholdType::Ratio(10));
            
            Ok(Some(StockPosition {
                id: row.get::<Option<i64>, _>("id"),
                code: row.get("code"),
                name: row.get("name"),
                buy_price: row.get("buy_price"),
                current_price: row.get("current_price"),
                highest_price_since_buy: row.get("highest_price_since_buy"),
                profit_threshold_level1_alerted: row.get::<i64, _>("profit_threshold_level1_alerted") != 0,
                profit_threshold_level2_alerted: row.get::<i64, _>("profit_threshold_level2_alerted") != 0,
                loss_threshold_alerted: row.get::<i64, _>("loss_threshold_alerted") != 0,
                profit_drawdown_half_alerted: row.get::<i64, _>("profit_drawdown_half_alerted") != 0,
                profit_threshold1: profit_threshold1,
                profit_threshold2: profit_threshold2,
                loss_threshold: loss_threshold,
                created_at: row
                    .get::<Option<String>, _>("created_at")
                    .and_then(|s| NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S").ok()),
                updated_at: row
                    .get::<Option<String>, _>("updated_at")
                    .and_then(|s| NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S").ok()),
            }))
        } else {
            Ok(None)
        }
    }
}

