import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

/**
 * 主窗口最小化后收进托盘（依赖后端 `is_minimized`）。
 * 在 GNOME/Wayland 上最小化状态常不可靠，关闭按钮会由 Rust 原生事件拦截为收到托盘。
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
