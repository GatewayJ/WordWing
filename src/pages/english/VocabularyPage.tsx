import { useCallback, useEffect, useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useTranslateHotkeyLabel } from "../../hooks/useTranslateHotkeyLabel";

const PER_PAGE = 10;
const MAX_STORED = 100;

type RecentItem = {
  id: string;
  source_text: string;
  translation: string;
  target_lang: string;
  created_at: string;
};

type RecentPagePayload = {
  items: RecentItem[];
  total: number;
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
  const navigate = useNavigate();
  const hotkeyLabel = useTranslateHotkeyLabel();
  const [page, setPage] = useState(1);
  const [payload, setPayload] = useState<RecentPagePayload | null>(null);
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState<string | null>(null);
  const [favoritingId, setFavoritingId] = useState<string | null>(null);

  const load = useCallback(async (p: number) => {
    setLoading(true);
    setErr(null);
    try {
      const data = await invoke<RecentPagePayload>("list_recent_translations_page", {
        page: p,
        perPage: PER_PAGE,
      });
      setPayload(data);
    } catch (e) {
      setErr(String(e));
      setPayload(null);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load(page);
  }, [load, page]);

  useEffect(() => {
    let un: UnlistenFn | undefined;
    void listen("recent-translations-changed", () => {
      void load(page);
    }).then((fn) => {
      un = fn;
    });
    return () => {
      void un?.();
    };
  }, [load, page]);

  const openTranslate = () => {
    void invoke("trigger_translate_overlay");
  };

  const total = payload?.total ?? 0;
  const totalPages = Math.max(1, Math.ceil(total / PER_PAGE));

  useEffect(() => {
    if (page > totalPages && totalPages >= 1) {
      setPage(totalPages);
    }
  }, [page, totalPages]);

  const favorite = async (row: RecentItem) => {
    setFavoritingId(row.id);
    setErr(null);
    try {
      await invoke("add_vocabulary_item", {
        sourceText: row.source_text,
        translation: row.translation,
        targetLang: row.target_lang,
        starred: true,
      });
      navigate("/english/collection");
    } catch (e) {
      setErr(String(e));
    } finally {
      setFavoritingId(null);
    }
  };

  return (
    <>
      <h2 className="page-title">生词</h2>
      <p className="page-lead">
        翻译成功后会自动记入下方「最近翻译」（最多保留 <strong>{MAX_STORED}</strong> 条）。点某一行的{" "}
        <strong>收藏</strong> 会加入 <Link to="/english/collection">收藏</Link> 页。
      </p>
      <p className="page-lead">
        按 <strong>{hotkeyLabel}</strong> 取选区并打开翻译浮层；无选区时回退剪贴板。可在{" "}
        <Link to="/settings">设置</Link> 更换快捷键。
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
          若快捷键无反应，可点此按钮；或先复制文本再走剪贴板路径。
        </span>
      </p>

      <section className="recent-section card" aria-label="最近翻译">
        <div className="recent-section__head">
          <h3 className="recent-section__title">最近翻译</h3>
          <span className="recent-section__meta">
            共 {total} 条（最多 {MAX_STORED} 条）
          </span>
        </div>

        {loading ? (
          <p className="recent-section__empty">加载中…</p>
        ) : !payload || payload.items.length === 0 ? (
          <p className="recent-section__empty">
            暂无记录。使用上方按钮或快捷键打开浮层，成功翻译后即会出现在此列表。
          </p>
        ) : (
          <>
            <ul className="recent-list">
              {payload.items.map((row) => (
                <li key={row.id} className="recent-list__item">
                  <div className="recent-list__body">
                    <p className="recent-list__source">{row.source_text}</p>
                    <p className="recent-list__translation">{row.translation}</p>
                    <p className="recent-list__meta">
                      <span>{row.target_lang}</span>
                      <span className="recent-list__time">{formatTime(row.created_at)}</span>
                    </p>
                  </div>
                  <button
                    type="button"
                    className="btn-primary recent-list__fav"
                    disabled={favoritingId === row.id}
                    onClick={() => void favorite(row)}
                  >
                    {favoritingId === row.id ? "保存中…" : "收藏"}
                  </button>
                </li>
              ))}
            </ul>

            {totalPages > 1 && (
              <div className="pager">
                <button
                  type="button"
                  className="btn-secondary pager__btn"
                  disabled={page <= 1}
                  onClick={() => setPage((p) => Math.max(1, p - 1))}
                >
                  上一页
                </button>
                <span className="pager__info">
                  第 {page} / {totalPages} 页
                </span>
                <button
                  type="button"
                  className="btn-secondary pager__btn"
                  disabled={page >= totalPages}
                  onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
                >
                  下一页
                </button>
              </div>
            )}
          </>
        )}
      </section>
    </>
  );
}
