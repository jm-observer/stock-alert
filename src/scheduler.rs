//! 交易时段调度模块

use crate::config::TradingHoursConfig;
use chrono::{Datelike, Local, NaiveTime, TimeZone, Weekday};
use chrono_tz::Tz;
use std::time::Duration;
use tokio::time::sleep;
use log::{debug, info};



#[derive(Clone)]
/// 交易时段调度器
pub struct TradingScheduler {
    config: TradingHoursConfig,
    timezone: Tz,
}

impl TradingScheduler {
    pub fn new(config: TradingHoursConfig, timezone_str: &str) -> anyhow::Result<Self> {
        let timezone: Tz = timezone_str.parse()?;
        Ok(Self {
            config,
            timezone,
        })
    }

    /// 解析时间字符串（HH:MM格式）
    fn parse_time(&self, time_str: &str) -> anyhow::Result<NaiveTime> {
        NaiveTime::parse_from_str(time_str, "%H:%M")
            .map_err(|e| anyhow::anyhow!("解析时间失败: {} - {}", time_str, e))
    }

    /// 判断当前是否在交易时段内
    pub fn is_trading_hours(&self) -> anyhow::Result<bool> {
        let now = Local::now().with_timezone(&self.timezone);
        let weekday = now.weekday();

        // 检查是否是工作日（周一到周五）
        if weekday == Weekday::Sat || weekday == Weekday::Sun {
            return Ok(false);
        }

        let current_time = now.time();
        let morning_start = self.parse_time(&self.config.morning_start)?;
        let morning_end = self.parse_time(&self.config.morning_end)?;
        let afternoon_start = self.parse_time(&self.config.afternoon_start)?;
        let afternoon_end = self.parse_time(&self.config.afternoon_end)?;

        let in_morning = current_time >= morning_start && current_time <= morning_end;
        let in_afternoon = current_time >= afternoon_start && current_time <= afternoon_end;

        Ok(in_morning || in_afternoon)
    }

    /// 等待到下一个交易时段开始
    pub async fn wait_until_trading_hours(&self) -> anyhow::Result<()> {
        loop {
            if self.is_trading_hours()? {
                info!("进入交易时段");
                return Ok(());
            }

            // 计算到下一个交易时段的等待时间
            let wait_seconds = self.calculate_wait_seconds()?;
            debug!("等待 {} 秒后进入交易时段", wait_seconds);
            sleep(Duration::from_secs(wait_seconds)).await;
        }
    }

    /// 计算到下一个交易时段的秒数
    fn calculate_wait_seconds(&self) -> anyhow::Result<u64> {
        let now = Local::now().with_timezone(&self.timezone);
        let weekday = now.weekday();
        let current_time = now.time();

        let morning_start = self.parse_time(&self.config.morning_start)?;
        let morning_end = self.parse_time(&self.config.morning_end)?;
        let afternoon_start = self.parse_time(&self.config.afternoon_start)?;
        let afternoon_end = self.parse_time(&self.config.afternoon_end)?;

        // 如果当前在交易时段内，返回0
        if self.is_trading_hours()? {
            return Ok(0);
        }

        // 计算到下一个交易时段的等待时间
        let mut wait_seconds = 0u64;

        // 如果当前是周末，等待到下周一
        if weekday == Weekday::Sat {
            // 计算到下周一上午开盘的时间
            let days_until_monday = 2;
            let next_monday = now.date_naive() + chrono::Duration::days(days_until_monday as i64);
            let next_monday_naive = next_monday.and_time(morning_start);
            let next_monday_dt = self.timezone.from_local_datetime(&next_monday_naive).single().unwrap();
            let diff = (next_monday_dt - now).num_seconds() as u64;
            return Ok(diff);
        } else if weekday == Weekday::Sun {
            // 计算到下周一上午开盘的时间
            let days_until_monday = 1;
            let next_monday = now.date_naive() + chrono::Duration::days(days_until_monday as i64);
            let next_monday_naive = next_monday.and_time(morning_start);
            let next_monday_dt = self.timezone.from_local_datetime(&next_monday_naive).single().unwrap();
            let diff = (next_monday_dt - now).num_seconds() as u64;
            return Ok(diff);
        }

        // 工作日：计算到下一个交易时段的等待时间
        if current_time < morning_start {
            // 等待到上午开盘
            let today_morning = now.date_naive().and_time(morning_start);
            let today_morning_dt = self.timezone.from_local_datetime(&today_morning).single().unwrap();
            wait_seconds = (today_morning_dt - now).num_seconds() as u64;
        } else if current_time > morning_end && current_time < afternoon_start {
            // 等待到下午开盘
            let today_afternoon = now.date_naive().and_time(afternoon_start);
            let today_afternoon_dt = self.timezone.from_local_datetime(&today_afternoon).single().unwrap();
            wait_seconds = (today_afternoon_dt - now).num_seconds() as u64;
        } else if current_time > afternoon_end {
            // 等待到明天上午开盘
            let tomorrow = now.date_naive() + chrono::Duration::days(1);
            let tomorrow_morning = tomorrow.and_time(morning_start);
            let tomorrow_morning_dt = self.timezone.from_local_datetime(&tomorrow_morning).single().unwrap();
            wait_seconds = (tomorrow_morning_dt - now).num_seconds() as u64;
        }

        Ok(wait_seconds.max(1)) // 至少等待1秒
    }

    /// 等待交易时段结束
    pub async fn wait_until_trading_hours_end(&self) -> anyhow::Result<()> {
        loop {
            if !self.is_trading_hours()? {
                info!("交易时段结束");
                return Ok(());
            }

            sleep(Duration::from_secs(60)).await; // 每分钟检查一次
        }
    }
}

