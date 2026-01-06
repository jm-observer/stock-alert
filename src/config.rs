//! 配置管理

use log::debug;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 应用配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// 数据库路径
    pub db_path: PathBuf,
    /// 交易时段配置
    pub trading_hours: TradingHoursConfig,
    /// 时区
    pub timezone: String,
    /// 行情源配置
    pub quote_source: QuoteSourceConfig,
    /// 告警配置
    pub alert: AlertConfig,
    /// 通知配置
    pub notify: NotifyConfig,
}

/// 交易时段配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingHoursConfig {
    /// 上午开始时间（HH:MM格式）
    pub morning_start: String,
    /// 上午结束时间
    pub morning_end: String,
    /// 下午开始时间
    pub afternoon_start: String,
    /// 下午结束时间
    pub afternoon_end: String,
}

impl Default for TradingHoursConfig {
    fn default() -> Self {
        Self {
            morning_start: "09:30".to_string(),
            morning_end: "11:30".to_string(),
            afternoon_start: "13:00".to_string(),
            afternoon_end: "15:00".to_string(),
        }
    }
}

/// 行情源配置类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum QuoteSourceConfig {
    /// SSE行情源配置
    Sse(QuoteSseSourceConfig),
    /// 测试行情源配置
    Test {
        /// 测试数据文件路径
        test_data_file: PathBuf,
        /// 价格更新间隔（秒）
        #[serde(default = "default_test_interval")]
        update_interval: u64,
        /// 是否循环播放
        #[serde(default = "default_true")]
        loop_playback: bool,
    },
}

impl QuoteSourceConfig {
    pub fn is_test(&self) -> bool {
        matches!(self, QuoteSourceConfig::Test { .. })
    }
}

/// 行情源配置类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteSseSourceConfig {
    /// SSE行情源配置
    /// SSE端点URL模板
    #[serde(default = "default_sse_url_template")]
    pub sse_url_template: String,
    /// SSE字段列表
    #[serde(default = "default_sse_fields")]
    pub sse_fields: String,
    /// SSE推送间隔（毫秒）
    #[serde(default = "default_sse_mpi")]
    pub sse_mpi: u64,
    /// SSE token
    #[serde(default = "default_sse_ut")]
    pub sse_ut: String,
    /// 是否启用轮询兜底
    #[serde(default = "default_true")]
    pub enable_polling_fallback: bool,
    /// 轮询间隔（秒）
    #[serde(default = "default_polling_interval")]
    pub polling_interval: u64,
    /// 轮询API URL模板
    #[serde(default = "default_polling_url_template")]
    pub polling_url_template: String,
    /// 最大重连次数（0表示无限）
    #[serde(default)]
    pub max_reconnect_attempts: u32,
    /// 重连初始延迟（秒）
    #[serde(default = "default_reconnect_initial_delay")]
    pub reconnect_initial_delay: u64,
    /// 重连最大延迟（秒）
    #[serde(default = "default_reconnect_max_delay")]
    pub reconnect_max_delay: u64,
}

fn default_sse_url_template() -> String {
    "https://22.push2.eastmoney.com/api/qt/stock/details/sse".to_string()
}

fn default_sse_fields() -> String {
    "f1,f2,f3,f4".to_string()
}

fn default_sse_mpi() -> u64 {
    1000
}

fn default_sse_ut() -> String {
    "bd1d9ddb04089700cf9c27f6f7426281".to_string()
}

fn default_polling_interval() -> u64 {
    2
}

fn default_polling_url_template() -> String {
    "https://push2.eastmoney.com/api/qt/stock/get".to_string()
}

fn default_reconnect_initial_delay() -> u64 {
    1
}

fn default_reconnect_max_delay() -> u64 {
    60
}

fn default_test_interval() -> u64 {
    1
}

fn default_true() -> bool {
    true
}

impl Default for QuoteSourceConfig {
    fn default() -> Self {
        Self::Sse(QuoteSseSourceConfig {
            sse_url_template: default_sse_url_template(),
            sse_fields: default_sse_fields(),
            sse_mpi: default_sse_mpi(),
            sse_ut: default_sse_ut(),
            enable_polling_fallback: true,
            polling_interval: default_polling_interval(),
            polling_url_template: default_polling_url_template(),
            max_reconnect_attempts: 0,
            reconnect_initial_delay: default_reconnect_initial_delay(),
            reconnect_max_delay: default_reconnect_max_delay(),
        })
    }
}

/// 告警配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    /// 盈利阈值列表（整数百分比，如10表示10%），第一个为1级，第二个为2级
    pub profit_thresholds: [i32; 2],
    /// 亏损阈值（整数百分比，如10表示10%）
    pub loss_threshold: i32,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            profit_thresholds: [10, 20], // 10%, 20%
            loss_threshold: 10, // 默认10%
        }
    }
}

/// 通知配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotifyConfig {
    /// 声音文件路径（可选）
    pub sound_file: Option<PathBuf>,
    /// 声音播放最小间隔（秒）
    pub sound_min_interval: u64,
    /// 日志文件路径（可选）
    pub log_file: Option<PathBuf>,
}

impl Default for NotifyConfig {
    fn default() -> Self {
        Self {
            sound_file: None,
            sound_min_interval: 1,
            log_file: Some(PathBuf::from("alerts.log")),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            db_path: PathBuf::from("stock_alert.db"),
            trading_hours: TradingHoursConfig::default(),
            timezone: "Asia/Shanghai".to_string(),
            quote_source: QuoteSourceConfig::default(),
            alert: AlertConfig::default(),
            notify: NotifyConfig::default(),
        }
    }
}

impl Config {
    /// 从文件加载配置，如果文件不存在则使用默认配置
    pub fn load_or_default(path: &PathBuf) -> anyhow::Result<Self> {
        debug!("加载配置文件: {}", path.display());
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            Ok(toml::from_str(&content)?)
        } else {
            Ok(Self::default())
        }
    }

    /// 保存配置到文件
    pub fn save(&self, path: &PathBuf) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
