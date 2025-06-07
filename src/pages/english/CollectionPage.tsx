import { useCallback, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { Star, Trash2 } from "lucide-react";

type VocabItem = {
  id: string;
  source_text: string;
  translation: string;
  target_lang: string;
  created_at: string;
  starred?: boolean;
  review_correct?: number;
  review_miss?: number;
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

  const toggleStar = async (id: string, starred: boolean) => {
    try {
      await invoke("set_vocabulary_starred", { id, starred });
    } catch (e) {
      setErr(String(e));
    }
  };

  const starred = items.filter((x) => !!x.starred);

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

      {loading ? (
        <p className="page-lead">加载中…</p>
      ) : starred.length === 0 ? (
        <div className="card">
          <p>
            暂无收藏。打开 <Link to="/english/vocabulary">生词</Link>{" "}
            使用翻译浮层，在最近列表中点击「收藏」即可。
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
                  <td className="cell-word">{row.source_text}</td>
                  <td>{row.translation}</td>
                  <td className="cell-muted">{row.target_lang}</td>
                  <td className="cell-muted" style={{ fontSize: 13 }}>
                    ✓{row.review_correct ?? 0} · ×{row.review_miss ?? 0}
                  </td>
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
