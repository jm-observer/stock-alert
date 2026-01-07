//! 通知模块

use crate::config::NotifyConfig;
use crate::models::AlertEvent;
use chrono::Local;
use rodio::{Decoder, OutputStream, Sink};
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::fs;
use log::{error, info, warn};

/// 通知管理器
pub struct Notifier {
    pub config: NotifyConfig,
    last_sound_time: Arc<Mutex<Instant>>,
    log_file: Option<Arc<Mutex<std::fs::File>>>,
}

impl Notifier {
    pub fn new(config: NotifyConfig) -> anyhow::Result<Self> {
        let log_file = if let Some(ref log_path) = config.log_file {
            Some(Arc::new(Mutex::new(
                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(log_path)?,
            )))
        } else {
            None
        };

        Ok(Self {
            config,
            last_sound_time: Arc::new(Mutex::new(Instant::now() - Duration::from_secs(1000))),
            log_file,
        })
    }

    /// 处理告警事件
    pub async fn handle_alert(&self, event: &AlertEvent) -> anyhow::Result<()> {
        // 记录日志
        self.log_alert(event).await?;

        // 播放声音（如果满足间隔要求）
        self.play_sound().await?;

        Ok(())
    }

    /// 记录告警日志
    async fn log_alert(&self, event: &AlertEvent) -> anyhow::Result<()> {
        let rule_str = match event.rule {
            crate::models::AlertRule::ProfitThreshold(t) => format!("盈利阈值 {}%", t),
            crate::models::AlertRule::LossThreshold(t) => format!("亏损阈值 {}%", t),
            crate::models::AlertRule::ProfitDrawdownHalf => "盈利回撤过半".to_string(),
        };

        let log_line = format!(
            "[{}] {} ({}) - {} - 当前价: {:.2}, 盈亏: {:.2}%, 最高盈利: {:.2}%\n",
            Local::now().format("%Y-%m-%d %H:%M:%S"),
            event.name,
            event.code,
            rule_str,
            event.current_price,
            event.pnl_ratio * 100.0,
            event.max_profit_ratio * 100.0
        );

        info!("{}", log_line.trim());

        if let Some(ref file) = self.log_file {
            use std::io::Write;
            let mut file = file.lock().await;
            file.write_all(log_line.as_bytes())?;
            file.flush()?;
        }

        Ok(())
    }

    /// 播放声音
    async fn play_sound(&self) -> anyhow::Result<()> {
        // 检查时间间隔
        let mut last_time = self.last_sound_time.lock().await;
        let elapsed = last_time.elapsed();
        if elapsed.as_secs() < self.config.sound_min_interval {
            info!("声音播放间隔太短，跳过: {}秒", elapsed.as_secs());
            return Ok(()); // 间隔太短，跳过
        }

        *last_time = Instant::now();

        // 播放声音
        if let Some(ref path) = self.config.sound_file {
            Self::play_sound_file(path).await?;
        } else {
            // 使用系统提示音（Windows）- 铃声效果
            tokio::task::spawn_blocking(|| {
                #[cfg(windows)]
                {
                    use winapi::um::utilapiset::Beep;
                    unsafe {
                        // 播放铃声效果：连续播放不同频率的音调
                        Beep(800, 2000);  // 800Hz, 200ms
                        std::thread::sleep(std::time::Duration::from_millis(50));
                        Beep(1000, 2000); // 1000Hz, 200ms
                        std::thread::sleep(std::time::Duration::from_millis(50));
                        Beep(1200, 3000); // 1200Hz, 300ms
                    }
                }
                #[cfg(not(windows))]
                {
                    // 其他平台可以使用系统命令
                    let _ = std::process::Command::new("paplay")
                        .arg("/usr/share/sounds/freedesktop/stereo/message.ogg")
                        .output();
                }
            })
            .await?;
        }

        Ok(())
    }

    /// 播放声音文件（异步）
    async fn play_sound_file(path: &PathBuf) -> anyhow::Result<()> {
        // 异步检查文件是否存在
        match fs::metadata(path).await {
            Ok(_) => {
                // 文件存在，继续处理
            }
            Err(_) => {
                warn!("声音文件不存在: {}", path.display());
                return Ok(());
            }
        }

        // 异步打开文件
        let tokio_file = match fs::File::open(path).await {
            Ok(file) => file,
            Err(e) => {
                error!("打开声音文件失败: {} - {}", path.display(), e);
                return Err(anyhow::anyhow!("打开声音文件失败: {}", e));
            }
        };

        // 将 tokio::fs::File 转换为 std::fs::File（rodio 需要）
        let std_file = tokio_file.into_std().await;

        // 在阻塞任务中执行音频播放（rodio 是同步库）
        let path_clone = path.clone();
        tokio::task::spawn_blocking(move || {
            let source = match Decoder::new(BufReader::new(std_file)) {
                Ok(source) => source,
                Err(e) => {
                    error!("解码声音文件失败: {} - {}", path_clone.display(), e);
                    return;
                }
            };

            let (_stream, stream_handle) = match OutputStream::try_default() {
                Ok(s) => s,
                Err(e) => {
                    error!("创建音频输出流失败: {}", e);
                    return;
                }
            };

            let sink = match Sink::try_new(&stream_handle) {
                Ok(s) => s,
                Err(e) => {
                    error!("创建音频接收器失败: {}", e);
                    return;
                }
            };

            sink.append(source);
            sink.sleep_until_end();
        })
        .await?;

        Ok(())
    }
}


