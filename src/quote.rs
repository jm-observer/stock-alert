//! 行情数据源模块

use crate::config::{QuoteSourceConfig, QuoteSseSourceConfig};
use crate::models::Quote;
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use chrono_tz::Asia::Shanghai;
use eventsource_client as es;
use eventsource_client::Client;
use futures::TryStreamExt;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::sleep;

/// 行情数据源 trait
#[async_trait]
pub trait QuoteSource: Send + Sync {
    /// 订阅股票行情，通过通道发送行情数据
    async fn subscribe(&self, code: String, tx: mpsc::UnboundedSender<Result<Quote>>) -> Result<()>;
}

/// 测试数据记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestQuoteRecord {
    /// 股票代码
    pub code: String,
    /// 价格
    pub price: f64,
    /// 时间戳（ISO 8601格式）
    pub timestamp: String,
}

/// 测试数据文件格式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestDataFile {
    /// 记录列表
    pub records: Vec<TestQuoteRecord>,
}

/// SSE 行情源
#[derive(Clone)]
pub struct SSESource {
    config: QuoteSseSourceConfig,
}

impl SSESource {
    pub fn new(config: QuoteSseSourceConfig) -> Self {
        Self { config }
    }

    /// 构建SSE URL
    fn build_sse_url(&self, code: &str) -> String {
        // 确定市场代码：6开头是上海(1)，其他是深圳(0)
        let market_code = if code.starts_with('6') { 1 } else { 0 };
        let secid = format!("{}.{}", market_code, code);

        format!(
            "{}?fields={}&mpi={}&invt=2&fltt=1&secid={}&ut={}&dect=1&wbp2u=|0|0|0|web",
            self.config.sse_url_template,
            self.config.sse_fields,
            self.config.sse_mpi,
            secid,
            self.config.sse_ut
        )}

    /// 解析单条detail记录
    /// 格式：时间,价格,数量,?,?
    /// 时间格式：HH:MM:SS
    /// 返回 (价格, 时间戳, 交易量)
    

    /// 解析SSE消息中的数据（价格、时间、交易量）
    /// SSE消息格式：{"rc":0,"rt":12,"svr":183124488,"lt":1,"full":0,"dlmkts":"","data":{"details":["10:22:45,16.37,52,3,1","10:22:48,16.37,54,2,1"]}}
    /// details数组中的格式：时间,价格,数量,?,?
    /// 时间格式：HH:MM:SS
    /// 返回所有记录，按时间顺序排序
    fn parse_quote_from_sse(&self, data: &str, code: &str) -> Vec<(f64, chrono::DateTime<Utc>, f64)> {
        parse_quote_from_sse(data, code)
    }
}

#[async_trait]
impl QuoteSource for SSESource {
    async fn subscribe(&self, code: String, tx: mpsc::UnboundedSender<Result<Quote>>) -> Result<()> {
        let config = self.config.clone();
        let sse_source = Arc::new(self.clone());
        let url = sse_source.build_sse_url(&code);

        // 启动SSE连接任务
        tokio::spawn(async move {
            let mut reconnect_delay = config.reconnect_initial_delay;
            let mut attempt = 0u32;

            loop {
                // 检查重连次数限制
                if config.max_reconnect_attempts > 0
                    && attempt >= config.max_reconnect_attempts
                {
                    error!("达到最大重连次数限制，停止重连: {}", code);
                    break;
                }

                if attempt > 0 {
                    info!("等待 {} 秒后重连: {}", reconnect_delay, code);
                    sleep(Duration::from_secs(reconnect_delay)).await;
                    reconnect_delay =
                        (reconnect_delay * 2).min(config.reconnect_max_delay);
                }

                attempt += 1;
                info!("开始连接SSE: {}，尝试次数: {}", code, attempt);
                debug!("连接SSE: {} -> {}", code, url);

                // 使用eventsource-client创建SSE客户端
                let builder = match es::ClientBuilder::for_url(&url) {
                    Ok(b) => b,
                    Err(e) => {
                        error!("创建SSE客户端失败: {} - {}", code, e);
                        sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                };

                // 配置重连选项
                let reconnect_opts = es::ReconnectOptions::reconnect(true)
                    .retry_initial(false)
                    .delay(Duration::from_secs(config.reconnect_initial_delay))
                    .backoff_factor(2)
                    .delay_max(Duration::from_secs(config.reconnect_max_delay))
                    .build();

                let client = builder.reconnect(reconnect_opts).build();

                debug!("连接SSE: {}  成功", code);

                // 处理事件流
                let mut stream = client.stream();
                loop {
                    match stream.try_next().await {
                        Ok(Some(event)) => {
                            match event {
                                es::SSE::Connected(connection) => {
                                    debug!(
                                        "SSE连接建立: {} - status={}",
                                        code,
                                        connection.response().status()
                                    );
                                }
                                        es::SSE::Event(ev) => {
                                            // 处理消息事件
                                            if ev.event_type == "message"
                                                || ev.event_type.is_empty()
                                            {
                                                // 解析所有记录，按时间顺序排序
                                                let quotes = sse_source.parse_quote_from_sse(&ev.data, &code);
                                                
                                                // 按时间顺序发送每条记录
                                                for (price, timestamp, volume) in quotes {
                                                    // 转换为本地时区显示
                                                    let local_time = timestamp.with_timezone(&Shanghai);
                                                    
                                                    debug!(
                                                        "发送行情: {} 价格={:.2} UTC时间={} 本地时间={} 交易量={:.2}",
                                                        code,
                                                        price,
                                                        timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
                                                        local_time.format("%Y-%m-%d %H:%M:%S %Z"),
                                                        volume
                                                    );
                                                    
                                                    let _ = tx.send(Ok(Quote {
                                                        code: code.clone(),
                                                        price,
                                                        volume: Some(volume),
                                                        timestamp,
                                                    }));
                                                }
                                            }
                                        }
                                es::SSE::Comment(_) => {
                                    // 忽略注释
                                }
                            }
                        }
                        Ok(None) => {
                            debug!("SSE流结束: {}", code);
                            break;
                        }
                        Err(e) => {
                            // 检查错误类型
                            let error_msg = e.to_string();
                            if error_msg.contains("body")
                                || error_msg.contains("stream")
                                || error_msg.contains("timeout")
                                || error_msg.contains("connection")
                            {
                                debug!(
                                    "SSE流读取结束: {} - {} (可能是正常的连接关闭)",
                                    code, e
                                );
                            } else {
                                warn!("读取SSE流错误: {} - {}", code, e);
                            }
                            break;
                        }
                    }
                }

                warn!("SSE连接断开: {}，准备重连", code);
                // eventsource-client会自动重连，这里只需要等待一下
                sleep(Duration::from_secs(1)).await;
            }
        });

        Ok(())
    }
}

/// 测试行情源
pub struct TestSource {
    records: Vec<TestQuoteRecord>,
    update_interval: u64,
    loop_playback: bool,
}

impl TestSource {
    pub fn new(config: QuoteSourceConfig) -> Result<Self> {
        if let QuoteSourceConfig::Test {
            test_data_file,
            update_interval,
            loop_playback,
        } = config
        {
            // 读取测试数据文件
            let content = fs::read_to_string(&test_data_file).context(format!(
                "读取测试数据文件失败: {}",
                test_data_file.display()
            ))?;
            let test_data: TestDataFile = serde_json::from_str(&content)
                .context("解析测试数据文件失败，请确保文件格式正确")?;

            info!(
                "加载测试数据文件: {}，共 {} 条记录",
                test_data_file.display(),
                test_data.records.len()
            );

            Ok(Self {
                records: test_data.records,
                update_interval,
                loop_playback,
            })
        } else {
            Err(anyhow::anyhow!("TestSource只能使用Test配置"))
        }
    }
}

#[async_trait]
impl QuoteSource for TestSource {
    async fn subscribe(&self, code: String, tx: mpsc::UnboundedSender<Result<Quote>>) -> Result<()> {
        let records = self.records.clone();
        let update_interval = self.update_interval;
        let loop_playback = self.loop_playback;

        tokio::spawn(async move {
            // 过滤出订阅的股票代码的记录
            let mut filtered_records: Vec<TestQuoteRecord> = records
                .into_iter()
                .filter(|r| r.code == code)
                .collect();

            // 按时间戳排序
            filtered_records.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

            if filtered_records.is_empty() {
                warn!("测试数据中没有找到订阅的股票代码: {}", code);
                return;
            }

            info!("开始播放测试数据: {}，共 {} 条记录", code, filtered_records.len());

            loop {
                for record in &filtered_records {
                    // 解析时间戳
                    if let Ok(timestamp) = DateTime::parse_from_rfc3339(&record.timestamp) {
                        let quote = Quote {
                            code: record.code.clone(),
                            price: record.price,
                            volume: None, // 测试数据中没有交易量
                            timestamp: timestamp.with_timezone(&Utc),
                        };

                        debug!(
                            "发送行情: {} 价格={:.2}",
                            record.code,
                            record.price,
                        );

                        if let Err(e) = tx.send(Ok(quote)) {
                            warn!("发送测试数据失败: {}", e);
                            return;
                        }

                        sleep(Duration::from_secs(update_interval)).await;
                    } else {
                        warn!("解析时间戳失败: {}", record.timestamp);
                    }
                }

                if !loop_playback {
                    info!("测试数据播放完成: {}", code);
                    break;
                } else {
                    info!("测试数据循环播放: {}", code);
                }
            }
        });

        Ok(())
    }
}

/// 创建行情源
pub fn create_quote_source(config: QuoteSourceConfig) -> Result<Arc<dyn QuoteSource>> {
    match config {
        QuoteSourceConfig::Sse(config) => Ok(Arc::new(SSESource::new(config))),
        QuoteSourceConfig::Test { .. } => {
            let test_source = TestSource::new(config)?;
            Ok(Arc::new(test_source))
        }
    }
}


    /// 解析SSE消息中的数据（价格、时间、交易量）
    /// SSE消息格式：{"rc":0,"rt":12,"svr":183124488,"lt":1,"full":0,"dlmkts":"","data":{"details":["10:22:45,16.37,52,3,1","10:22:48,16.37,54,2,1"]}}
    /// details数组中的格式：时间,价格,数量,?,?
    /// 时间格式：HH:MM:SS
    /// 返回所有记录，按时间顺序排序
    fn parse_quote_from_sse(data: &str, code: &str) -> Vec<(f64, chrono::DateTime<Utc>, f64)> {
        debug!("解析SSE数据 [{}]: {}", code, data);

        let mut results = Vec::new();

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
            if let Some(data_obj) = json.get("data") {
                if let Some(details_array) = data_obj.get("details").and_then(|v| v.as_array()) {
                    if details_array.is_empty() {
                        debug!("details数组为空 [{}]", code);
                        return results;
                    }

                    // 获取当前日期（上海时区）
                    let now_shanghai = chrono::Utc::now().with_timezone(&Shanghai);
                    let date = now_shanghai.date_naive();

                    // 解析所有记录
                    for detail_str in details_array.iter().filter_map(|v| v.as_str()) {
                        if let Some(record) = parse_detail_record(detail_str, code, date) {
                            results.push(record);
                        }
                    }

                    // 按时间顺序排序（时间早的在前面）
                    results.sort_by(|a, b| a.1.cmp(&b.1));

                    debug!("解析结果 [{}]: 共 {} 条记录，按时间排序", code, results.len());
                    for (i, (price, timestamp, volume)) in results.iter().enumerate() {
                        let local_time = timestamp.with_timezone(&Shanghai);
                        debug!(
                            "  记录 {}: 价格={:.2}, UTC时间={}, 本地时间={}, 交易量={:.2}",
                            i + 1,
                            price,
                            timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
                            local_time.format("%Y-%m-%d %H:%M:%S %Z"),
                            volume
                        );
                    }
                } else {
                    debug!("data中没有details字段或不是数组 [{}]", code);
                }
            } else {
                warn!("JSON中没有data字段 [{}]", code);
            }
        } else {
            warn!("解析JSON失败 [{}]: {}", code, data);
        }

        results
    }


fn parse_detail_record(
    detail_str: &str,
    code: &str,
    date: chrono::NaiveDate,
) -> Option<(f64, chrono::DateTime<Utc>, f64)> {
    // 解析格式：时间,价格,数量,?,?
    let parts: Vec<&str> = detail_str.split(',').collect();
    if parts.len() < 3 {
        warn!("detail格式不正确 [{}]: {}", code, detail_str);
        return None;
    }

    // 解析价格
    let price = match parts[1].parse::<f64>() {
        Ok(p) => p,
        Err(_) => {
            warn!("解析价格失败 [{}]: {}", code, parts[1]);
            return None;
        }
    };

    // 解析时间 HH:MM:SS，如果失败则使用当前时间
    let time_str = parts[0];
    let timestamp = if let Ok(time) = chrono::NaiveTime::parse_from_str(time_str, "%H:%M:%S") {
        // 组合日期和时间
        let datetime = date.and_time(time);

        // 转换为UTC时间
        // 使用 earliest() 来处理可能的夏令时转换
        let local_dt = Shanghai
            .from_local_datetime(&datetime)
            .earliest()
            .unwrap_or_else(|| {
                // 如果本地时间转换失败，使用UTC时间
                Shanghai.from_utc_datetime(&datetime)
            });
        local_dt.with_timezone(&chrono::Utc)
    } else {
        warn!("解析时间失败 [{}]: {}，使用当前时间", code, time_str);
        chrono::Utc::now()
    };

    // 解析数量（股数），如果失败则使用 0.0
    let volume = parts[2].parse::<f64>().unwrap_or_else(|_| {
        warn!("解析交易量失败 [{}]: {}，使用 0.0", code, parts[2]);
        0.0
    });

    Some((price, timestamp, volume))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    #[test]
    fn test_parse_quote_from_sse() {
        let code = "000001";

        // 测试正常情况
        let json_data = r#"{"rc":0,"rt":12,"svr":183124488,"lt":1,"full":0,"dlmkts":"","data":{"details":["10:22:45,16.37,52,3,1","10:22:48,16.37,54,2,1","10:22:51,16.37,27,3,1"]}}"#;
        let results = parse_quote_from_sse(json_data, code);
        
        assert_eq!(results.len(), 3);
        
        // 验证第一条记录
        let (price1, timestamp1, volume1) = &results[0];
        assert_eq!(*price1, 16.37);
        assert_eq!(*volume1, 52.0);
        
        // 验证第二条记录
        let (price2, timestamp2, volume2) = &results[1];
        assert_eq!(*price2, 16.37);
        assert_eq!(*volume2, 54.0);
        
        // 验证第三条记录
        let (price3, timestamp3, volume3) = &results[2];
        assert_eq!(*price3, 16.37);
        assert_eq!(*volume3, 27.0);
        
        // 验证时间顺序（第一条应该早于第二条）
        assert!(timestamp1 <= timestamp2);
        assert!(timestamp2 <= timestamp3);

        // 测试空数组
        let empty_json = r#"{"rc":0,"rt":12,"svr":183124488,"lt":1,"full":0,"dlmkts":"","data":{"details":[]}}"#;
        let results = parse_quote_from_sse(empty_json, code);
        assert_eq!(results.len(), 0);

        // 测试无效JSON
        let invalid_json = r#"{"invalid":"json"}"#;
        let results = parse_quote_from_sse(invalid_json, code);
        assert_eq!(results.len(), 0);

        // 测试缺少data字段
        let no_data_json = r#"{"rc":0,"rt":12}"#;
        let results = parse_quote_from_sse(no_data_json, code);
        assert_eq!(results.len(), 0);

        // 测试缺少details字段
        let no_details_json = r#"{"rc":0,"rt":12,"data":{}}"#;
        let results = parse_quote_from_sse(no_details_json, code);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_parse_quote_from_sse_time_sorting() {
        let code = "000001";

        // 测试乱序的时间，验证排序功能
        let json_data = r#"{"rc":0,"rt":12,"svr":183124488,"lt":1,"full":0,"dlmkts":"","data":{"details":["10:22:51,16.37,27,3,1","10:22:45,16.37,52,3,1","10:22:48,16.37,54,2,1"]}}"#;
        let results = parse_quote_from_sse(json_data, code);
        
        assert_eq!(results.len(), 3);
        
        // 验证已按时间排序
        let (_, ts1, _) = &results[0];
        let (_, ts2, _) = &results[1];
        let (_, ts3, _) = &results[2];
        
        assert!(ts1 <= ts2);
        assert!(ts2 <= ts3);
        
        // 验证第一条是10:22:45
        let local1 = ts1.with_timezone(&Shanghai);
        assert_eq!(local1.hour(), 10);
        assert_eq!(local1.minute(), 22);
        assert_eq!(local1.second(), 45);
        
        // 验证最后一条是10:22:51
        let local3 = ts3.with_timezone(&Shanghai);
        assert_eq!(local3.hour(), 10);
        assert_eq!(local3.minute(), 22);
        assert_eq!(local3.second(), 51);
    }
}
