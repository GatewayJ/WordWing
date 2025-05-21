import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useTranslateHotkeyLabel } from "../hooks/useTranslateHotkeyLabel";
import "./translate-overlay.css";

export type TranslateState =
  | { kind: "idle" }
  | { kind: "empty"; reason: string }
  | { kind: "loading"; source: string; source_truncated?: boolean }
  | { kind: "success"; source: string; translation: string; target_lang: string }
  | { kind: "error"; source?: string; message: string };

function applyStoredTheme() {
  try {
    const t = localStorage.getItem("wordwing-theme");
    if (t === "dark" || t === "light") {
      document.documentElement.setAttribute("data-theme", t);
    }
  } catch {
    /* ignore */
  }
}

export function TranslateOverlay() {
  const hotkeyLabel = useTranslateHotkeyLabel();
  const [state, setState] = useState<TranslateState>({ kind: "idle" });
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    applyStoredTheme();
  }, []);

  useEffect(() => {
    let un: UnlistenFn | undefined;
    void listen<TranslateState>("translate-state", (ev) => {
      setState(ev.payload);
    }).then((fn) => {
      un = fn;
    });
    return () => {
      void un?.();
    };
  }, []);

  const hide = useCallback(() => {
    void getCurrentWebviewWindow().hide();
  }, []);

  const save = useCallback(async () => {
    if (state.kind !== "success") return;
    setSaving(true);
    try {
      await invoke("add_vocabulary_item", {
        source_text: state.source,
        translation: state.translation,
        target_lang: state.target_lang,
      });
    } finally {
      setSaving(false);
    }
  }, [state]);

  const retryClipboard = useCallback(() => {
    void invoke("translate_from_clipboard_only");
  }, []);

  const retryNetwork = useCallback(() => {
    if (state.kind === "error" && state.source) {
      void invoke("retry_translate_with_text", { source: state.source });
    } else if (state.kind === "success") {
      void invoke("retry_translate_with_text", { source: state.source });
    }
  }, [state]);

  return (
    <div className="translate-overlay">
      <header className="translate-overlay__head">
        <span className="translate-overlay__title">翻译</span>
        <button type="button" className="translate-overlay__close" onClick={hide} aria-label="关闭">
          ×
        </button>
      </header>

      <div className="translate-overlay__body">
        {state.kind === "idle" && (
          <p className="translate-overlay__muted">
            按 <strong>{hotkeyLabel}</strong> 划词翻译（可在主窗口设置中更换）。
          </p>
        )}

        {state.kind === "empty" && (
          <div className="translate-overlay__panel">
            <p className="translate-overlay__error">{state.reason}</p>
            <div className="translate-overlay__actions">
              <button type="button" className="btn-ghost" onClick={retryClipboard}>
                用剪贴板再试
              </button>
              <button type="button" className="btn-ghost" onClick={hide}>
                关闭
              </button>
            </div>
          </div>
        )}

        {state.kind === "loading" && (
          <div className="translate-overlay__panel">
            <div className="translate-overlay__skeleton" aria-hidden />
            <div className="translate-overlay__skeleton translate-overlay__skeleton--mid" aria-hidden />
            <div className="translate-overlay__skeleton translate-overlay__skeleton--short" aria-hidden />
            <p className="translate-overlay__source-preview">
              {state.source}
              {state.source_truncated ? "…" : ""}
            </p>
            <p className="translate-overlay__muted">翻译中…</p>
          </div>
        )}

        {state.kind === "success" && (
          <div className="translate-overlay__panel">
            <p className="translate-overlay__source">
              <strong>{state.source}</strong>
            </p>
            <p className="translate-overlay__result">{state.translation}</p>
            <p className="translate-overlay__meta">目标：{state.target_lang}</p>
            <div className="translate-overlay__actions">
              <button type="button" className="btn-primary" disabled={saving} onClick={save}>
                {saving ? "保存中…" : "收藏"}
              </button>
              <button type="button" className="btn-ghost" onClick={retryNetwork}>
                重试
              </button>
              <button type="button" className="btn-ghost" onClick={hide}>
                关闭
              </button>
            </div>
          </div>
        )}

        {state.kind === "error" && (
          <div className="translate-overlay__panel">
            <p className="translate-overlay__error">{state.message}</p>
            {state.source && (
              <p className="translate-overlay__source-preview">{state.source}</p>
            )}
            <div className="translate-overlay__actions">
              <button type="button" className="btn-primary" onClick={retryNetwork}>
                重试
              </button>
              <button type="button" className="btn-ghost" onClick={retryClipboard}>
                用剪贴板再试
              </button>
              <button type="button" className="btn-ghost" onClick={hide}>
                关闭
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
