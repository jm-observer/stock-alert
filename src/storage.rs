//! 数据持久化模块

use crate::models::StockPosition;
use anyhow::Result;
use chrono::{NaiveDate, NaiveDateTime, Utc};
use sqlx::{sqlite::SqlitePool, Row};
use std::path::PathBuf;
use log::{debug, info};

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
                buy_date TEXT NOT NULL,
                current_price REAL,
                highest_price_since_buy REAL,
                profit_threshold_level1_alerted INTEGER NOT NULL DEFAULT 0,
                profit_threshold_level2_alerted INTEGER NOT NULL DEFAULT 0,
                loss_threshold_alerted INTEGER NOT NULL DEFAULT 0,
                profit_drawdown_half_alerted INTEGER NOT NULL DEFAULT 0,
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

        info!("数据库初始化完成: {}", db_path.display());
        Ok(Self { pool })
    }

    /// 获取所有股票持仓
    pub async fn list_positions(&self) -> Result<Vec<StockPosition>> {
        let rows = sqlx::query(
            r#"
            SELECT id, code, name, buy_price, buy_date, current_price, 
                   highest_price_since_buy, enabled, profit_threshold_level1_alerted, 
                   profit_threshold_level2_alerted, loss_threshold_alerted, 
                   profit_drawdown_half_alerted, created_at, updated_at
            FROM stock_positions
            ORDER BY code
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut positions = Vec::new();
        for row in rows {
            positions.push(StockPosition {
                id: row.get::<Option<i64>, _>("id"),
                code: row.get("code"),
                name: row.get("name"),
                buy_price: row.get("buy_price"),
                buy_date: NaiveDate::parse_from_str(&row.get::<String, _>("buy_date"), "%Y-%m-%d")?,
                current_price: row.get("current_price"),
                highest_price_since_buy: row.get("highest_price_since_buy"),
                profit_threshold_level1_alerted: row.get::<i64, _>("profit_threshold_level1_alerted") != 0,
                profit_threshold_level2_alerted: row.get::<i64, _>("profit_threshold_level2_alerted") != 0,
                loss_threshold_alerted: row.get::<i64, _>("loss_threshold_alerted") != 0,
                profit_drawdown_half_alerted: row.get::<i64, _>("profit_drawdown_half_alerted") != 0,
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
        let now = Utc::now().naive_utc();
        let result = sqlx::query(
            r#"
            INSERT INTO stock_positions 
            (code, name, buy_price, buy_date, current_price, highest_price_since_buy, 
             profit_threshold_level1_alerted, profit_threshold_level2_alerted, 
             loss_threshold_alerted, profit_drawdown_half_alerted, 
             created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&position.code)
        .bind(&position.name)
        .bind(position.buy_price)
        .bind(position.buy_date.format("%Y-%m-%d").to_string())
        .bind(position.current_price)
        .bind(position.highest_price_since_buy)
        .bind(if position.profit_threshold_level1_alerted { 1 } else { 0 })
        .bind(if position.profit_threshold_level2_alerted { 1 } else { 0 })
        .bind(if position.loss_threshold_alerted { 1 } else { 0 })
        .bind(if position.profit_drawdown_half_alerted { 1 } else { 0 })
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

    /// 更新股票持仓（买入价、买入日期）
    pub async fn update_position(&self, code: &str, buy_price: f64, buy_date: NaiveDate) -> Result<bool> {
        let now = Utc::now().naive_utc();
        let result = sqlx::query(
            r#"
            UPDATE stock_positions 
            SET buy_price = ?, buy_date = ?, updated_at = ?
            WHERE code = ?
            "#,
        )
        .bind(buy_price)
        .bind(buy_date.format("%Y-%m-%d").to_string())
        .bind(now.format("%Y-%m-%d %H:%M:%S").to_string())
        .bind(code)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 全量更新股票持仓信息（批量）
    pub async fn update_positions(&self, positions: &[StockPosition]) -> Result<()> {
        let now = Utc::now().naive_utc();
        let mut tx = self.pool.begin().await?;

        for position in positions {
            sqlx::query(
                r#"
                UPDATE stock_positions 
                SET current_price = ?, highest_price_since_buy = ?, 
                    profit_threshold_level1_alerted = ?, profit_threshold_level2_alerted = ?, 
                    loss_threshold_alerted = ?, profit_drawdown_half_alerted = ?, updated_at = ?
                WHERE code = ?
                "#,
            )
            .bind(position.current_price)
            .bind(position.highest_price_since_buy)
            .bind(if position.profit_threshold_level1_alerted { 1 } else { 0 })
            .bind(if position.profit_threshold_level2_alerted { 1 } else { 0 })
            .bind(if position.loss_threshold_alerted { 1 } else { 0 })
            .bind(if position.profit_drawdown_half_alerted { 1 } else { 0 })
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
            SELECT id, code, name, buy_price, buy_date, current_price, 
                   highest_price_since_buy, enabled, profit_threshold_level1_alerted, 
                   profit_threshold_level2_alerted, loss_threshold_alerted, 
                   profit_drawdown_half_alerted, created_at, updated_at
            FROM stock_positions
            WHERE code = ?
            "#,
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(StockPosition {
                id: row.get::<Option<i64>, _>("id"),
                code: row.get("code"),
                name: row.get("name"),
                buy_price: row.get("buy_price"),
                buy_date: NaiveDate::parse_from_str(&row.get::<String, _>("buy_date"), "%Y-%m-%d")?,
                current_price: row.get("current_price"),
                highest_price_since_buy: row.get("highest_price_since_buy"),
                profit_threshold_level1_alerted: row.get::<i64, _>("profit_threshold_level1_alerted") != 0,
                profit_threshold_level2_alerted: row.get::<i64, _>("profit_threshold_level2_alerted") != 0,
                loss_threshold_alerted: row.get::<i64, _>("loss_threshold_alerted") != 0,
                profit_drawdown_half_alerted: row.get::<i64, _>("profit_drawdown_half_alerted") != 0,
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

