//! 定时任务到期时通过系统通知提醒（应用运行期间轮询，约 30s）。

use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::todo::TodoStore;

pub fn spawn_schedule_notification_loop(app: AppHandle) {
    let app_h = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(30));
        loop {
            ticker.tick().await;
            if let Err(e) = poll_due_schedules(&app_h) {
                eprintln!("[WordWing] todo schedule notification poll: {}", e);
            }
        }
    });
}

fn poll_due_schedules(app: &AppHandle) -> Result<(), String> {
    let Some(todo) = app.try_state::<TodoStore>() else {
        return Ok(());
    };
    let now = chrono::Utc::now();
    let due = todo.list_due_unsent(&now)?;
    let mut any_marked = false;
    for sch in due {
        let show = app
            .notification()
            .builder()
            .title("WordWing · 定时")
            .body(&sch.title)
            .show();
        match show {
            Ok(_) => {
                if let Err(e) = todo.mark_schedule_notification_sent(&sch.id) {
                    eprintln!("[WordWing] mark_schedule_notification_sent {}: {}", sch.id, e);
                } else {
                    any_marked = true;
                }
            }
            Err(e) => eprintln!(
                "[WordWing] system notification failed for schedule {}: {}",
                sch.id, e
            ),
        }
    }
    if any_marked {
        let _ = app.emit("todo-schedules-changed", ());
    }
    Ok(())
}
