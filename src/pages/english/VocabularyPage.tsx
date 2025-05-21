import { useCallback, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { Trash2 } from "lucide-react";
import { useTranslateHotkeyLabel } from "../../hooks/useTranslateHotkeyLabel";

type VocabItem = {
  id: string;
  source_text: string;
  translation: string;
  target_lang: string;
  created_at: string;
};

function formatTime(iso: string) {
  try {
    const d = new Date(iso);
    return d.toLocaleString("zh-CN", {
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
    });
  } catch {
    return iso;
  }
}

export function VocabularyPage() {
  const hotkeyLabel = useTranslateHotkeyLabel();
  const [items, setItems] = useState<VocabItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setErr(null);
    try {
      const list = await invoke<VocabItem[]>("list_vocabulary");
      setItems(list);
    } catch (e) {
      setErr(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    let un: UnlistenFn | undefined;
    void listen("vocabulary-changed", () => {
      void load();
    }).then((fn) => {
      un = fn;
    });
    return () => {
      void un?.();
    };
  }, [load]);

  const remove = async (id: string) => {
    try {
      await invoke("delete_vocabulary_item", { id });
    } catch (e) {
      setErr(String(e));
    }
  };

  const openTranslate = () => {
    void invoke("trigger_translate_overlay");
  };

  return (
    <>
      <h2 className="page-title">生词</h2>
      <p className="page-lead">
        按 <strong>{hotkeyLabel}</strong> 取选区（PRIMARY）并打开翻译浮层；无选区时回退剪贴板。收藏后本表自动刷新。可在{" "}
        <Link to="/settings">设置</Link> 更换快捷键以减少与浏览器/IDE 冲突。
      </p>

      {err && (
        <p className="page-lead" style={{ color: "var(--error)" }}>
          {err}
        </p>
      )}

      <p className="page-lead" style={{ marginTop: -8 }}>
        <button type="button" className="settings-save-btn" onClick={openTranslate}>
          打开翻译浮层（划词）
        </button>
        <span className="cell-muted" style={{ marginLeft: 12, fontSize: 14 }}>
          若快捷键无反应，可在 Wayland 下使用此按钮；或先复制文本再试「剪贴板」路径。
        </span>
      </p>

      {loading ? (
        <p className="page-lead">加载中…</p>
      ) : items.length === 0 ? (
        <div className="card">
          <p>暂无收录。在翻译浮层点击「收藏」即可加入本表。</p>
        </div>
      ) : (
        <div className="data-table-wrap">
          <table className="data-table">
            <thead>
              <tr>
                <th>原文</th>
                <th>译文</th>
                <th>目标</th>
                <th>时间</th>
                <th aria-label="操作" />
              </tr>
            </thead>
            <tbody>
              {items.map((row) => (
                <tr key={row.id}>
                  <td className="cell-word">{row.source_text}</td>
                  <td>{row.translation}</td>
                  <td className="cell-muted">{row.target_lang}</td>
                  <td className="cell-muted">{formatTime(row.created_at)}</td>
                  <td>
                    <button
                      type="button"
                      className="btn-icon"
                      aria-label="删除"
                      onClick={() => void remove(row.id)}
                    >
                      <Trash2 size={18} strokeWidth={2} />
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </>
  );
}
