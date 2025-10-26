mod popup_window;
mod selection_monitor;
mod translator;

use global_hotkey::GlobalHotKeyEvent;
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyManager,
};
use popup_window::PopupWindow;
use selection_monitor::SelectionMonitor;
use std::sync::mpsc;
use tokio::time::{sleep, Duration};
use tracing::info;
use tracing_subscriber;
use translator::Translator;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::ConnectionExt;
use x11rb::rust_connection::RustConnection;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_file(true) // 显示文件名
        .with_line_number(true) // 显示行号
        .with_thread_ids(true) // 显示线程ID
        .with_target(true) // 显示目标
        .init();

    info!("Application starting");

    let (tx, rx) = mpsc::channel::<String>();
    let selection_manager = SelectionMonitor::new(rx)?;
    let translator_manager = Translator::new(std::env::var("DASHSCOPE_API_KEY")?);
    let popup_manager = PopupWindow::new(tx);
    let hot_key_manager = GlobalHotKeyManager::new().unwrap();

    // construct the hotkey
    let hotkey = HotKey::new(Some(Modifiers::SHIFT), Code::KeyD);

    // register it
    hot_key_manager.register(hotkey)?;

    loop {
        if let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            println!("{:?}", event);
            let selection = match selection_manager.get_selection().await {
                Some(text) => text,
                None => continue,
            };
            let target_lang = if is_chinese(&selection) {
                "English"
            } else {
                "Chinese"
            };
            info!("Translating to: {}", target_lang);
            match translator_manager.translate(&selection, target_lang).await {
                Ok(translation) => {
                    if let Some((x, y)) = get_mouse_position() {
                        info!("Showing popup at ({}, {})", x, y);

                        popup_manager.show_at_mouse(&translation, x + 10, y + 10);
                    }
                }
                Err(e) => eprintln!("Translation error: {}", e),
            }
        }

        while gtk::events_pending() {
            gtk::main_iteration_do(false);
        }
        sleep(Duration::from_millis(300)).await;
    }
}

fn is_chinese(text: &str) -> bool {
    text.chars()
        .any(|c| c as u32 >= 0x4e00 && c as u32 <= 0x9fff)
}

fn get_mouse_position() -> Option<(i32, i32)> {
    match get_mouse_position_impl() {
        Ok(pos) => pos,
        Err(e) => {
            tracing::warn!("Failed to get mouse position: {}", e);
            None
        }
    }
}

fn get_mouse_position_impl() -> Result<Option<(i32, i32)>, Box<dyn std::error::Error>> {
    // 创建 X11 连接
    let (conn, screen_num) = RustConnection::connect(None)?;
    let screen = &conn.setup().roots[screen_num];

    // 查询鼠标指针位置
    let reply = conn.query_pointer(screen.root)?.reply()?;

    // 返回根窗口中的鼠标坐标
    Ok(Some((reply.root_x as i32, reply.root_y as i32)))
}
//  yum install openssl-devel
//  sudo yum install pango-devel
// sudo yum install cairo-devel
//  export PKG_CONFIG_PATH="/usr/lib/x86_64-linux-gnu/pkgconfig"
//  yum install glib2-devel cmake gcc-c++  cairo-gobject-devel  sudo dnf install gtk3-devel pango-devel atk-devel cairo-devel gdk-pixbuf2-devel glib2-devel

// find /usr -name "gdk-3.0.pc" 2>/dev/null
// export PKG_CONFIG_PATH="/usr/lib/x86_64-linux-gnu/pkgconfig:/usr/lib64/pkgconfig/gdk-3.0.pc:/usr/lib64/pkgconfig/atk.pc"
