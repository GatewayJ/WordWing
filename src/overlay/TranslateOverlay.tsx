import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useTranslateHotkeyLabel } from "../hooks/useTranslateHotkeyLabel";
import "./translate-overlay.css";

export type TranslateState =
  | { kind: "idle" }
  | { kind: "empty"; reason: string; bilingual_overlay?: boolean; zh_to_en?: boolean }
  | { kind: "loading"; source: string; source_truncated?: boolean; bilingual_overlay?: boolean; zh_to_en?: boolean }
  | {
      kind: "success";
      source: string;
      translation: string;
      target_lang: string;
      bilingual_overlay?: boolean;
      zh_to_en?: boolean;
    }
  | { kind: "error"; source?: string; message: string; bilingual_overlay?: boolean; zh_to_en?: boolean };

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

/** Ctrl+Shift+2 中英通道（兼容旧字段 zh_to_en） */
function isBilingualOverlay(s: TranslateState): boolean {
  if (s.kind === "idle") return false;
  return s.bilingual_overlay === true || s.zh_to_en === true;
}

export function TranslateOverlay() {
  const hotkeyLabel = useTranslateHotkeyLabel();
  const [state, setState] = useState<TranslateState>({ kind: "idle" });
  const [saving, setSaving] = useState(false);
  const [saveMsg, setSaveMsg] = useState<string | null>(null);
  const [copyMsg, setCopyMsg] = useState<string | null>(null);

  useEffect(() => {
    applyStoredTheme();
  }, []);

  useEffect(() => {
    let un: UnlistenFn | undefined;
    void listen<TranslateState>("translate-state", (ev) => {
      setSaveMsg(null);
      setCopyMsg(null);
      setState(ev.payload);
    }).then((fn) => {
      un = fn;
    });
    return () => {
      void un?.();
    };
  }, []);

  const hide = useCallback(() => {
    void getCurrentWebviewWindow()
      .hide()
      .catch((e) => console.error("[WordWing] overlay hide failed:", e));
  }, []);

  const save = useCallback(async () => {
    if (state.kind !== "success") return;
    setSaving(true);
    setSaveMsg(null);
    try {
      await invoke("add_vocabulary_item", {
        sourceText: state.source,
        translation: state.translation,
        targetLang: state.target_lang,
        starred: true,
      });
      setSaveMsg("已加入收藏。主窗口打开「收藏」可查看；「生词」页可浏览最近翻译记录。");
    } catch (e) {
      const msg = String(e);
      setSaveMsg(msg.includes("not allowed") || msg.includes("denied") ? `收藏失败：无权限调用保存（${msg}）` : `收藏失败：${msg}`);
      console.error("[WordWing] add_vocabulary_item:", e);
    } finally {
      setSaving(false);
    }
  }, [state]);

  const copyTranslation = useCallback(async () => {
    if (state.kind !== "success" || !isBilingualOverlay(state)) return;
    setCopyMsg(null);
    try {
      await invoke("write_clipboard_text", { text: state.translation });
      setCopyMsg("已复制译文到剪贴板。");
    } catch (e) {
      setCopyMsg(`复制失败：${String(e)}`);
    }
  }, [state]);

  /** 重新读取标准剪贴板并翻译（不沿用浮层内缓存的原文）。 */
  const retryFromClipboard = useCallback(() => {
    if (isBilingualOverlay(state)) {
      void invoke("translate_from_clipboard_zh_en");
    } else {
      void invoke("translate_from_clipboard_only");
    }
  }, [state]);

  const overlayTitle = isBilingualOverlay(state) ? "中英翻译" : "翻译";

  return (
    <div className="translate-overlay">
      <header className="translate-overlay__head">
        <div className="translate-overlay__drag-region" data-tauri-drag-region>
          <span className="translate-overlay__title">{overlayTitle}</span>
        </div>
        <button type="button" className="translate-overlay__close" onClick={hide} aria-label="关闭">
          ×
        </button>
      </header>

      <div className="translate-overlay__body">
        {state.kind === "idle" && (
          <p className="translate-overlay__muted">
            按 <strong>{hotkeyLabel}</strong> 划词翻译：先选中文字再按快捷键（Linux 使用 PRIMARY 选区；可在设置中更换该键）。
            <br />
            按 <strong>Ctrl+Shift+2</strong> 打开<strong>中英翻译</strong>：同样<strong>先划词再按</strong>（无划词则用剪贴板）；自动判断译成英文或中文，成功后可<strong>复制译文</strong>。
          </p>
        )}

        {state.kind === "empty" && (
          <div className="translate-overlay__panel">
            <p className="translate-overlay__error">{state.reason}</p>
            <div className="translate-overlay__actions">
              <button type="button" className="btn-ghost" onClick={retryFromClipboard}>
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
            <p className="translate-overlay__muted">
              {isBilingualOverlay(state) ? "中英翻译中…" : "翻译中…"}
            </p>
          </div>
        )}

        {state.kind === "success" && (
          <div className="translate-overlay__panel">
            <p className="translate-overlay__source">
              <strong>{state.source}</strong>
            </p>
            <p className="translate-overlay__result">{state.translation}</p>
            <p className="translate-overlay__meta">目标：{state.target_lang}</p>
            {saveMsg && !isBilingualOverlay(state) && (
              <p
                className="translate-overlay__muted"
                style={{
                  marginTop: 10,
                  color: saveMsg.startsWith("收藏失败") ? "var(--error)" : "var(--success)",
                }}
              >
                {saveMsg}
              </p>
            )}
            {copyMsg && (
              <p
                className="translate-overlay__muted"
                style={{
                  marginTop: 8,
                  color: copyMsg.startsWith("复制失败") ? "var(--error)" : "var(--success)",
                }}
              >
                {copyMsg}
              </p>
            )}
            <div className="translate-overlay__actions">
              {isBilingualOverlay(state) && (
                <button type="button" className="btn-primary" onClick={() => void copyTranslation()}>
                  复制译文
                </button>
              )}
              {!isBilingualOverlay(state) && (
                <button type="button" className="btn-primary" disabled={saving} onClick={() => void save()}>
                  {saving ? "保存中…" : "收藏"}
                </button>
              )}
              <button type="button" className="btn-ghost" onClick={retryFromClipboard}>
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
              <button type="button" className="btn-primary" onClick={retryFromClipboard}>
                重试（读剪贴板）
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
