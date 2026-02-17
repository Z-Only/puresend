//! PureSend 文件传输应用
//!
//! 提供本地网络和云盘文件传输功能

// 模块声明
pub mod commands;
pub mod discovery;
pub mod error;
pub mod models;
pub mod transfer;

use commands::{DiscoveryState, TransferState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(TransferState::new())
        .manage(DiscoveryState::new())
        .invoke_handler(tauri::generate_handler![
            // 传输命令
            commands::init_transfer,
            commands::get_transfer_port,
            commands::prepare_file_transfer,
            commands::get_file_metadata,
            commands::send_file,
            commands::cancel_transfer,
            commands::get_transfer_progress,
            commands::get_active_tasks,
            commands::verify_file_integrity,
            commands::cleanup_completed_tasks,
            commands::get_network_info,
            commands::start_receiving,
            commands::stop_receiving,
            commands::get_receive_directory,
            commands::set_receive_directory,
            // 设备发现命令
            commands::init_discovery,
            commands::stop_discovery,
            commands::get_peers,
            commands::get_peer,
            commands::add_peer_manual,
            commands::is_peer_online,
            commands::get_online_count,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
