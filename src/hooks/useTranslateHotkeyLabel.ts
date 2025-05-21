import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

/** 与设置页「翻译快捷键」同步；默认文案与后端默认预设一致。 */
export function useTranslateHotkeyLabel() {
  const [label, setLabel] = useState("Super + Shift + T（推荐）");

  useEffect(() => {
    const load = () => {
      void invoke<string>("get_translate_hotkey_display")
        .then(setLabel)
        .catch(() => {
          /* 非 Tauri 环境（如纯 vite） */
        });
    };
    void load();
    let unlisten: (() => void) | undefined;
    void listen("translate-hotkey-changed", () => {
      load();
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
    };
  }, []);

  return label;
}
