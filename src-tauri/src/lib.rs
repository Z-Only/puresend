//! PureSend 文件传输应用
//!
//! 提供本地网络和云盘文件传输功能

mod discovery;
mod error;
mod models;
mod share;
mod transfer;
mod web_upload;

use discovery::DiscoveryState;
use share::ShareManagerState;
use transfer::TransferState;
use web_upload::WebUploadManagerState;
use tauri::Manager;

#[cfg(target_os = "macos")]
use tauri::Emitter;

#[cfg(target_os = "macos")]
use tauri::menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder};

/// 菜单文本的中英文映射
#[cfg(target_os = "macos")]
struct MenuTexts {
    // PureSend 菜单
    about: &'static str,
    quit: &'static str,
    // 文件菜单
    file: &'static str,
    send_file: &'static str,
    // 编辑菜单
    edit: &'static str,
    // 视图菜单
    view: &'static str,
    toggle_fullscreen: &'static str,
    // 窗口菜单
    window: &'static str,
    minimize: &'static str,
    zoom: &'static str,
    // 帮助菜单
    help: &'static str,
    github: &'static str,
    docs: &'static str,
}

#[cfg(target_os = "macos")]
const MENU_TEXTS_ZH: MenuTexts = MenuTexts {
    about: "关于 PureSend",
    quit: "退出",
    file: "文件",
    send_file: "发送文件",
    edit: "编辑",
    view: "视图",
    toggle_fullscreen: "切换全屏",
    window: "窗口",
    minimize: "最小化",
    zoom: "缩放",
    help: "帮助",
    github: "GitHub 仓库",
    docs: "在线文档",
};

#[cfg(target_os = "macos")]
const MENU_TEXTS_EN: MenuTexts = MenuTexts {
    about: "About PureSend",
    quit: "Quit",
    file: "File",
    send_file: "Send File",
    edit: "Edit",
    view: "View",
    toggle_fullscreen: "Toggle Fullscreen",
    window: "Window",
    minimize: "Minimize",
    zoom: "Zoom",
    help: "Help",
    github: "GitHub Repository",
    docs: "Online Documentation",
};

/// 根据语言获取菜单文本
#[cfg(target_os = "macos")]
fn get_menu_texts(lang: &str) -> &'static MenuTexts {
    if lang.starts_with("zh") {
        &MENU_TEXTS_ZH
    } else {
        &MENU_TEXTS_EN
    }
}

/// 构建 macOS 系统菜单栏
#[cfg(target_os = "macos")]
fn build_menu(
    app: &tauri::AppHandle,
    lang: &str,
) -> Result<tauri::menu::Menu<tauri::Wry>, tauri::Error> {
    let texts = get_menu_texts(lang);

    // PureSend 菜单
    let about_item = MenuItemBuilder::with_id("about", texts.about).build(app)?;
    let app_submenu = SubmenuBuilder::new(app, "PureSend")
        .item(&about_item)
        .separator()
        .item(&PredefinedMenuItem::quit(app, Some(texts.quit))?)
        .build()?;

    // 文件菜单
    let send_file_item = MenuItemBuilder::with_id("send_file", texts.send_file)
        .accelerator("CmdOrCtrl+O")
        .build(app)?;
    let file_submenu = SubmenuBuilder::new(app, texts.file)
        .item(&send_file_item)
        .build()?;

    // 编辑菜单
    let edit_submenu = SubmenuBuilder::new(app, texts.edit)
        .item(&PredefinedMenuItem::undo(app, None)?)
        .item(&PredefinedMenuItem::redo(app, None)?)
        .separator()
        .item(&PredefinedMenuItem::cut(app, None)?)
        .item(&PredefinedMenuItem::copy(app, None)?)
        .item(&PredefinedMenuItem::paste(app, None)?)
        .item(&PredefinedMenuItem::select_all(app, None)?)
        .build()?;

    // 视图菜单
    let fullscreen_item = MenuItemBuilder::with_id("toggle_fullscreen", texts.toggle_fullscreen)
        .accelerator("CmdOrCtrl+F")
        .build(app)?;
    let view_submenu = SubmenuBuilder::new(app, texts.view)
        .item(&fullscreen_item)
        .build()?;

    // 窗口菜单
    let window_submenu = SubmenuBuilder::new(app, texts.window)
        .item(&PredefinedMenuItem::minimize(app, Some(texts.minimize))?)
        .item(&PredefinedMenuItem::maximize(app, Some(texts.zoom))?)
        .build()?;

    // 帮助菜单
    let github_item = MenuItemBuilder::with_id("open_github", texts.github).build(app)?;
    let docs_item = MenuItemBuilder::with_id("open_docs", texts.docs).build(app)?;
    let help_submenu = SubmenuBuilder::new(app, texts.help)
        .item(&github_item)
        .item(&docs_item)
        .build()?;

    MenuBuilder::new(app)
        .item(&app_submenu)
        .item(&file_submenu)
        .item(&edit_submenu)
        .item(&view_submenu)
        .item(&window_submenu)
        .item(&help_submenu)
        .build()
}

/// 更新菜单栏语言
#[cfg(target_os = "macos")]
#[tauri::command]
fn update_menu_language(app: tauri::AppHandle, lang: String) -> Result<(), String> {
    let menu = build_menu(&app, &lang).map_err(|e| e.to_string())?;
    app.set_menu(menu).map_err(|e| e.to_string())?;
    Ok(())
}

/// 占位命令（非 macOS 平台）
#[cfg(not(target_os = "macos"))]
#[tauri::command]
fn update_menu_language(_lang: String) -> Result<(), String> {
    Ok(())
}

/// 切换 WebView DevTools 开关
#[tauri::command]
fn toggle_devtools(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        if enabled {
            window.open_devtools();
        } else {
            window.close_devtools();
        }
        Ok(())
    } else {
        Err("Main window not found".to_string())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_android_fs::init())
        .manage(TransferState::default())
        .manage(DiscoveryState::default())
        .manage(ShareManagerState::default())
        .manage(WebUploadManagerState::default())
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
            crate::transfer::start_receiving,
            crate::transfer::stop_receiving,
            crate::transfer::get_receive_directory,
            crate::transfer::set_receive_directory,
            crate::transfer::send_file,
            crate::transfer::send_file_async,
            crate::transfer::cancel_transfer,
            crate::transfer::get_transfer_progress,
            crate::transfer::get_active_tasks,
            crate::transfer::verify_file_integrity,
            crate::transfer::cleanup_completed_tasks,
            // Receive settings commands
            crate::transfer::get_receive_settings,
            crate::transfer::set_auto_receive,
            crate::transfer::set_file_overwrite,
            crate::transfer::get_unique_file_path,
            // Share commands
            crate::share::start_share,
            crate::share::stop_share,
            crate::share::get_share_info,
            crate::share::get_access_requests,
            crate::share::accept_access_request,
            crate::share::reject_access_request,
            crate::share::remove_access_request,
            crate::share::clear_access_requests,
            crate::share::update_share_files,
            crate::share::update_share_settings,
            // Web upload commands
            crate::web_upload::start_web_upload,
            crate::web_upload::stop_web_upload,
            crate::web_upload::get_web_upload_requests,
            crate::web_upload::accept_web_upload,
            crate::web_upload::reject_web_upload,
            // Menu commands
            update_menu_language,
            toggle_devtools,
        ]);

    // macOS: 构建自定义菜单栏并处理菜单事件
    #[cfg(target_os = "macos")]
    let builder = builder.setup(|app| {
        let handle = app.handle().clone();
        let menu = build_menu(&handle, "zh-CN")?;
        app.set_menu(menu)?;

        // 处理菜单事件
        app.on_menu_event(move |app_handle, event| {
            match event.id().as_ref() {
                "about" => {
                    // 发送事件到前端
                    let _ = app_handle.emit("menu-event", "about");
                }
                "send_file" => {
                    let _ = app_handle.emit("menu-event", "send_file");
                }
                "toggle_fullscreen" => {
                    if let Some(window) = app_handle.get_webview_window("main") {
                        let is_fullscreen: bool = window.is_fullscreen().unwrap_or(false);
                        let _ = window.set_fullscreen(!is_fullscreen);
                    }
                }
                "open_github" => {
                    let _ = open::that("https://github.com/z-only/puresend");
                }
                "open_docs" => {
                    let _ = open::that("https://z-only.github.io/puresend/");
                }
                _ => {}
            }
        });

        Ok(())
    });

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
