import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

/**
 * 主窗口最小化后收进托盘。
 * Linux/GTK 上最小化状态晚于事件到达，与后端延迟检测配合使用。
 */
export function attachMainWindowMinimizeToTray(): void {
  const win = getCurrentWindow();
  if (win.label !== "main") {
    return;
  }

  let hideTimer: ReturnType<typeof setTimeout> | undefined;
  const schedule = () => {
    if (hideTimer !== undefined) {
      clearTimeout(hideTimer);
    }
    hideTimer = setTimeout(() => {
      hideTimer = undefined;
      void invoke("hide_main_if_minimized").catch(() => {});
    }, 170);
  };

  void win.onResized(() => {
    schedule();
  });
  void win.onFocusChanged(({ payload: focused }) => {
    if (!focused) {
      schedule();
    }
  });
}
