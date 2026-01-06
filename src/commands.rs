//! Tauri命令模块，提供前端调用的API

use crate::app_tauri::TauriApp;
use crate::config::Config;
use crate::models::StockPosition;
use crate::storage::Storage;
use anyhow::Result;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, State};

/// 获取应用数据目录路径
pub fn get_app_data_dir(app: &AppHandle) -> Result<PathBuf> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| anyhow::anyhow!("获取应用数据目录失败: {}", e))?;
    std::fs::create_dir_all(&app_data_dir)?;
    Ok(app_data_dir)
}

/// 获取数据库路径
pub fn get_db_path(app: &AppHandle) -> Result<PathBuf> {
    let app_data_dir = get_app_data_dir(app)?;
    Ok(app_data_dir.join("stock_alert.db"))
}

/// 获取配置文件路径
pub fn get_config_path(app: &AppHandle) -> Result<PathBuf> {
    let app_data_dir = get_app_data_dir(app)?;
    Ok(app_data_dir.join("config.toml"))
}

#[tauri::command]
pub async fn add_stock(
    tauri_app: State<'_, TauriApp>,
    code: String,
    name: String,
    buy_price: f64,
    buy_date: String,
) -> Result<i64, String> {
    tauri_app.inner().add_stock(code, name, buy_price, buy_date).await.map_err(|x| x.to_string())
}

/// 删除股票持仓
#[tauri::command]
pub async fn delete_stock(tauri_app: State<'_, TauriApp>, code: String) -> Result<bool, String> {
    tauri_app.inner().delete_stock(code)
        .await
        .map_err(|e| format!("数据库连接失败: {}", e))
}

/// 获取所有股票持仓
#[tauri::command]
pub async fn list_stocks(app: AppHandle) -> Result<Vec<StockPosition>, String> {
    let db_path = get_db_path(&app).map_err(|e| e.to_string())?;
    let storage = Storage::new(&db_path)
        .await
        .map_err(|e| format!("数据库连接失败: {}", e))?;

    storage
        .list_positions()
        .await
        .map_err(|e| format!("获取股票列表失败: {}", e))
}

/// 获取配置
#[tauri::command]
pub async fn get_config(app: AppHandle) -> Result<Config, String> {
    let config_path = get_config_path(&app).map_err(|e| e.to_string())?;
    Config::load_or_default(&config_path)
        .map_err(|e| format!("加载配置失败: {}", e))
}

/// 保存配置
#[tauri::command]
pub async fn save_config(app: AppHandle, config: Config) -> Result<(), String> {
    let config_path = get_config_path(&app).map_err(|e| e.to_string())?;
    config
        .save(&config_path)
        .map_err(|e| format!("保存配置失败: {}", e))
}

/// 更新告警阈值
#[tauri::command]
pub async fn update_alert_thresholds(
    tauri_app: State<'_, TauriApp>,
    profit_thresholds: [i32; 2],
    loss_threshold: i32,
) -> Result<(), String> {
    tauri_app
        .inner()
        .update_alert_thresholds(profit_thresholds, loss_threshold)
        .await
        .map_err(|e| e.to_string())
}

/// 更新声音文件配置
#[tauri::command]
pub async fn update_sound_file(
    tauri_app: State<'_, TauriApp>,
    sound_file: Option<String>,
) -> Result<(), String> {
    tauri_app.inner().update_sound_file(sound_file)
        .await
        .map_err(|e| e.to_string())
}

