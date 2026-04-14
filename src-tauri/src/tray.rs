//! 系统托盘（Ubuntu / Linux 桌面等）：最小化收进托盘，托盘菜单打开主窗口与划词翻译。

use tauri::menu::{MenuBuilder, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};

#[cfg(any(target_os = "android", target_os = "ios"))]
pub fn setup_tray(_app: &AppHandle) -> tauri::Result<()> {
    Ok(())
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let show_item = MenuItem::with_id(
        app,
        "tray_show_main",
        "打开主窗口",
        true,
        None::<&str>,
    )?;
    let translate_item = MenuItem::with_id(
        app,
        "tray_translate",
        "划词翻译",
        true,
        None::<&str>,
    )?;
    let menu = MenuBuilder::new(app)
        .items(&[&show_item, &translate_item])
        .build()?;

    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| tauri::Error::AssetNotFound("default window icon".into()))?;

    TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| {
            match event.id.as_ref() {
                "tray_show_main" => show_main_window(app),
                "tray_translate" => {
                    let h = app.clone();
                    tauri::async_runtime::spawn(async move {
                        crate::translate_flow_selection_first(h).await;
                    });
                }
                _ => {}
            }
        })
        .build(app)?;

    Ok(())
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn show_main_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}
