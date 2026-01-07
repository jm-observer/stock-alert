// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app_tauri;
mod commands;
mod config;
mod engine;
mod models;
mod notify;
mod quote;
mod scheduler;
mod storage;

use app_tauri::TauriApp;
use commands::*;
use log::{error, info};
use tauri::Manager;

fn main() -> anyhow::Result<()> {
    // 初始化日志
    let _logger = custom_utils::logger::logger_feature(
        "stock-alert",
        "debug,sqlx=info,reqwest=info,hyper_util=info,hyper=info,eventsource_client=info,rustls=info,symphonia_core=info,symphonia_bundle_mp3=info",
        log::LevelFilter::Info,
        true,
    )
    .build();

    info!("股票价格提醒程序启动（Tauri模式）");


    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // 启动监控循环
            let app_handle = app.handle().clone();

            let app_handle_clone = app_handle.clone();
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            let tauri_app = tauri::async_runtime::block_on(async move {
                TauriApp::new(app_handle_clone, tx).await
            }).unwrap();
            let tauri_app_clone = tauri_app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = tauri_app.run(rx).await {
                    error!("监控循环运行出错: {}", e);
                }
            });
            app_handle.manage(tauri_app_clone);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            add_stock,
            delete_stock,
            list_stocks,
            get_config,
            save_config,
            update_alert_thresholds,
            update_sound_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    Ok(())
}

