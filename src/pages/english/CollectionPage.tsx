import { useCallback, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { Pencil, Search, Star, Trash2 } from "lucide-react";
import { PronounceButton } from "../../components/PronounceButton";

type VocabItem = {
  id: string;
  source_text: string;
  translation: string;
  target_lang: string;
  created_at: string;
  starred?: boolean;
  review_correct?: number;
  review_miss?: number;
  review_interval_days?: number;
  next_review_at?: string | null;
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

export function CollectionPage() {
  const [items, setItems] = useState<VocabItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editSource, setEditSource] = useState("");
  const [editTranslation, setEditTranslation] = useState("");
  const [editTarget, setEditTarget] = useState("");
  const [saving, setSaving] = useState(false);

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
    if (!window.confirm("确定删除该收藏？复习记录也会一并删除。")) return;
    try {
      await invoke("delete_vocabulary_item", { id });
    } catch (e) {
      setErr(String(e));
    }
  };

  const startEdit = (item: VocabItem) => {
    setEditingId(item.id);
    setEditSource(item.source_text);
    setEditTranslation(item.translation);
    setEditTarget(item.target_lang);
    setErr(null);
  };

  const saveEdit = async () => {
    if (!editingId) return;
    setSaving(true);
    setErr(null);
    try {
      await invoke("update_vocabulary_item", {
        id: editingId,
        sourceText: editSource,
        translation: editTranslation,
        targetLang: editTarget,
      });
      setEditingId(null);
    } catch (e) {
      setErr(String(e));
    } finally {
      setSaving(false);
    }
  };

  const toggleStar = async (id: string, starred: boolean) => {
    try {
      await invoke("set_vocabulary_starred", { id, starred });
    } catch (e) {
      setErr(String(e));
    }
  };

  const normalizedQuery = query.trim().toLocaleLowerCase();
  const starred = items.filter((x) => {
    if (!x.starred) return false;
    if (!normalizedQuery) return true;
    return [x.source_text, x.translation, x.target_lang].some((value) =>
      value.toLocaleLowerCase().includes(normalizedQuery),
    );
  });

  return (
    <>
      <h2 className="page-title">收藏</h2>
      <p className="page-lead">
        星标条目会出现在此页，并在 <Link to="/english/review">复习</Link> 中略更常抽到。可从{" "}
        <Link to="/english/vocabulary">生词</Link> 最近翻译里点「收藏」加入，或在翻译浮层点「收藏」。
      </p>

      {err && (
        <p className="page-lead" style={{ color: "var(--error)" }}>
          {err}
        </p>
      )}

      <label className="collection-search card" htmlFor="collection-search">
        <Search size={18} aria-hidden />
        <input
          id="collection-search"
          className="todo-input"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder="搜索原文、译文或目标语言"
        />
      </label>

      {loading ? (
        <p className="page-lead">加载中…</p>
      ) : starred.length === 0 ? (
        <div className="card">
          <p>
            {normalizedQuery ? (
              <>没有匹配“{query.trim()}”的收藏。</>
            ) : (
              <>
                暂无收藏。打开 <Link to="/english/vocabulary">生词</Link>{" "}
                使用翻译浮层，在最近列表中点击「收藏」即可。
              </>
            )}
          </p>
        </div>
      ) : (
        <div className="data-table-wrap">
          <table className="data-table">
            <thead>
              <tr>
                <th aria-label="收藏" />
                <th>原文</th>
                <th>译文</th>
                <th>目标</th>
                <th>复习</th>
                <th>时间</th>
                <th aria-label="操作" />
              </tr>
            </thead>
            <tbody>
              {starred.map((row) => (
                <tr key={row.id}>
                  <td>
                    <button
                      type="button"
                      className="btn-icon"
                      aria-label={row.starred ? "取消收藏星标" : "标为收藏"}
                      onClick={() => void toggleStar(row.id, !row.starred)}
                    >
                      <Star
                        size={18}
                        strokeWidth={2}
                        fill={row.starred ? "var(--warning)" : "none"}
                        color={row.starred ? "var(--warning)" : "var(--muted)"}
                      />
                    </button>
                  </td>
                  <td className="cell-word">
                    {editingId === row.id ? (
                      <input className="todo-input" value={editSource} onChange={(e) => setEditSource(e.target.value)} />
                    ) : (
                      <span className="text-with-pronunciation">
                        <span>{row.source_text}</span>
                        <PronounceButton text={row.source_text} />
                      </span>
                    )}
                  </td>
                  <td>
                    {editingId === row.id ? (
                      <input className="todo-input" value={editTranslation} onChange={(e) => setEditTranslation(e.target.value)} />
                    ) : (
                      <span className="text-with-pronunciation">
                        <span>{row.translation}</span>
                        <PronounceButton text={row.translation} />
                      </span>
                    )}
                  </td>
                  <td className="cell-muted">
                    {editingId === row.id ? (
                      <input className="todo-input" value={editTarget} onChange={(e) => setEditTarget(e.target.value)} />
                    ) : row.target_lang}
                  </td>
                  <td className="cell-muted" style={{ fontSize: 13 }}>
                    ✓{row.review_correct ?? 0} · ×{row.review_miss ?? 0}
                  </td>
                  <td className="cell-muted">{formatTime(row.created_at)}</td>
                  <td>
                    <div className="todo-row__actions">
                      {editingId === row.id ? (
                        <>
                          <button type="button" className="btn-secondary btn-small" disabled={saving} onClick={() => void saveEdit()}>
                            保存
                          </button>
                          <button type="button" className="btn-secondary btn-small" onClick={() => setEditingId(null)}>
                            取消
                          </button>
                        </>
                      ) : (
                        <button type="button" className="btn-icon" aria-label="编辑" onClick={() => startEdit(row)}>
                          <Pencil size={17} strokeWidth={2} />
                        </button>
                      )}
                      <button type="button" className="btn-icon" aria-label="删除" onClick={() => void remove(row.id)}>
                        <Trash2 size={18} strokeWidth={2} />
                      </button>
                    </div>
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
