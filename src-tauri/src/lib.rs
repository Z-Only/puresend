//! PureSend 文件传输应用
//!
//! 提供本地网络和云盘文件传输功能

mod discovery;
mod error;
mod models;
mod share;
mod transfer;

use discovery::DiscoveryState;
use transfer::TransferState;
use share::ShareManagerState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(TransferState::default())
        .manage(DiscoveryState::default())
        .manage(ShareManagerState::default())
        .invoke_handler(tauri::generate_handler![
            // Device commands
            crate::discovery::get_device_name,
            // Discovery commands
            crate::discovery::init_discovery,
            crate::discovery::stop_discovery,
            crate::discovery::get_peers,
            crate::discovery::get_peer,
            crate::discovery::add_peer_manual,
            crate::discovery::is_peer_online,
            crate::discovery::get_online_count,
            // Transfer commands
            crate::transfer::init_transfer,
            crate::transfer::get_transfer_port,
            crate::transfer::prepare_file_transfer,
            crate::transfer::get_file_metadata,
            crate::transfer::get_files_in_folder,
            crate::transfer::get_network_info,
            crate::transfer::send_file,
            crate::transfer::send_file_async,
            crate::transfer::cancel_transfer,
            crate::transfer::get_transfer_progress,
            crate::transfer::get_active_tasks,
            crate::transfer::verify_file_integrity,
            crate::transfer::cleanup_completed_tasks,
            // Share commands
            crate::share::start_share,
            crate::share::stop_share,
            crate::share::get_share_info,
            crate::share::get_access_requests,
            crate::share::accept_access_request,
            crate::share::reject_access_request,
            crate::share::update_share_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}