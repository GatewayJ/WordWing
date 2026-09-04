import { useCallback, useEffect, useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

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

type ReviewQueue = {
  items: VocabItem[];
  total: number;
  next_due_at: string | null;
};

function pickWeighted(items: VocabItem[]): VocabItem {
  const weights = items.map(
    (it) => 1 + (it.review_miss ?? 0) * 2 + (it.starred ? 2 : 0),
  );
  const sum = weights.reduce((a, b) => a + b, 0);
  let r = Math.random() * sum;
  for (let i = 0; i < items.length; i++) {
    r -= weights[i];
    if (r <= 0) return items[i];
  }
  return items[items.length - 1];
}

export function ReviewPage() {
  const [items, setItems] = useState<VocabItem[]>([]);
  const [queue, setQueue] = useState<ReviewQueue>({ items: [], total: 0, next_due_at: null });
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState<string | null>(null);
  const [current, setCurrent] = useState<VocabItem | null>(null);
  const [showAnswer, setShowAnswer] = useState(false);
  const [answered, setAnswered] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    setErr(null);
    try {
      const [list, due] = await Promise.all([
        invoke<VocabItem[]>("list_vocabulary"),
        invoke<ReviewQueue>("get_review_queue"),
      ]);
      setItems(list);
      setQueue(due);
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

  const pickNext = useCallback((list: VocabItem[], excludeId?: string) => {
    if (list.length === 0) {
      setCurrent(null);
      return;
    }
    const pool = excludeId ? list.filter((x) => x.id !== excludeId) : list;
    const source = pool.length > 0 ? pool : list;
    setCurrent(pickWeighted(source));
    setShowAnswer(false);
    setAnswered(false);
  }, []);

  useEffect(() => {
    if (loading) return;
    if (items.length === 0) {
      setCurrent(null);
      return;
    }
    // 到期队列为空时仍允许用户从收藏列表主动选择词条提前复习。
    if (current && items.some((x) => x.id === current.id)) return;
    if (queue.items.length > 0) {
      pickNext(queue.items);
    } else {
      setCurrent(null);
    }
  }, [loading, items, queue.items, current, pickNext]);

  const stats = useMemo(() => {
    const c = items.reduce((s, i) => s + (i.review_correct ?? 0), 0);
    const m = items.reduce((s, i) => s + (i.review_miss ?? 0), 0);
    return { c, m, n: items.length };
  }, [items]);

  const starredItems = useMemo(
    () => items.filter((i) => !!i.starred),
    [items],
  );

  const focusStarred = useCallback(
    (id: string) => {
      const it = items.find((x) => x.id === id);
      if (!it) return;
      setCurrent(it);
      setShowAnswer(false);
      setAnswered(false);
    },
    [items],
  );

  const onRemembered = async (remembered: boolean) => {
    if (!current || answered) return;
    setAnswered(true);
    try {
      await invoke("record_vocab_review", { id: current.id, remembered });
      const [list, due] = await Promise.all([
        invoke<VocabItem[]>("list_vocabulary"),
        invoke<ReviewQueue>("get_review_queue"),
      ]);
      setItems(list);
      setQueue(due);
      pickNext(due.items, current.id);
    } catch (e) {
      setErr(String(e));
      setAnswered(false);
    }
  };

  const skip = () => {
    if (!current) return;
    // 与答完后下一题一致：优先换一道，避免加权随机又抽到当前这条
    const source = queue.items.length > 0 ? queue.items : items;
    pickNext(source, current.id);
  };

  const nextDueLabel = queue.next_due_at
    ? new Date(queue.next_due_at).toLocaleString("zh-CN", {
        month: "2-digit",
        day: "2-digit",
        hour: "2-digit",
        minute: "2-digit",
      })
    : null;

  return (
    <>
      <h2 className="page-title">复习</h2>
      <p className="page-lead">
        按间隔重复计划抽取<strong>到期词条</strong>；答对后间隔逐步增长，答错后约 10 分钟再次出现。到期队列内优先抽取错题，左侧收藏也可提前复习。
      </p>
      <p className="page-lead" style={{ marginTop: -16 }}>
        待复习 <strong>{queue.items.length}</strong> 条 · 共 <strong>{stats.n}</strong> 条 · 累计答对 <strong>{stats.c}</strong> · 答错{" "}
        <strong>{stats.m}</strong>
        {stats.n === 0 && (
          <>
            {" "}
            · 去 <Link to="/english/vocabulary">生词</Link> 翻译后在列表里点收藏
          </>
        )}
      </p>

      {err && (
        <p className="page-lead" style={{ color: "var(--error)" }}>
          {err}
        </p>
      )}

      {loading ? (
        <p className="page-lead">加载中…</p>
      ) : items.length === 0 ? (
        <div className="card">
          <p>生词本为空，无法复习。</p>
        </div>
      ) : !current ? (
        <div className="card">
          <p><strong>本轮复习已完成。</strong></p>
          <p className="page-lead" style={{ marginBottom: 0 }}>
            {nextDueLabel ? `下一条计划复习时间：${nextDueLabel}` : "暂时没有后续复习计划。"}
          </p>
          {starredItems.length > 0 ? (
            <div className="review-complete__early">
              <span className="cell-muted">提前复习收藏：</span>
              {starredItems.slice(0, 8).map((item) => (
                <button type="button" className="btn-secondary btn-small" key={item.id} onClick={() => focusStarred(item.id)}>
                  {item.source_text}
                </button>
              ))}
            </div>
          ) : null}
        </div>
      ) : current ? (
        <div className="review-layout">
          <aside className="review-starred card" aria-label="收藏单词列表">
            <div className="review-starred__head">
              <h3 className="review-starred__title">收藏单词</h3>
              <span className="review-starred__count">{starredItems.length} 条</span>
            </div>
            {starredItems.length === 0 ? (
              <p className="review-starred__empty">
                暂无星标收藏。在 <Link to="/english/vocabulary">生词</Link>{" "}
                最近翻译里点「收藏」，或在翻译浮层点「收藏」。
              </p>
            ) : (
              <ul className="review-starred__list">
                {starredItems.map((row) => {
                  const active = current.id === row.id;
                  return (
                    <li key={row.id}>
                      <button
                        type="button"
                        className={
                          active
                            ? "review-starred__row review-starred__row--active"
                            : "review-starred__row"
                        }
                        onClick={() => focusStarred(row.id)}
                      >
                        <span className="review-starred__row-main" title={row.source_text}>
                          {row.source_text}
                        </span>
                        <span className="review-starred__row-meta">
                          <span className="review-starred__lang">{row.target_lang}</span>
                          <span className="review-starred__stats" aria-hidden>
                            ✓{row.review_correct ?? 0} · ×{row.review_miss ?? 0}
                          </span>
                        </span>
                      </button>
                    </li>
                  );
                })}
              </ul>
            )}
          </aside>

          <div className="review-card card">
            <p className="review-card__meta">
              目标语言：<span>{current.target_lang}</span>
              {current.starred ? (
                <span className="review-card__star"> ★ 收藏</span>
              ) : null}
              {(current.review_interval_days ?? 0) > 0 ? (
                <span> · 当前间隔 {current.review_interval_days} 天</span>
              ) : null}
            </p>
            <p className="review-card__prompt">{current.source_text}</p>
            {!showAnswer ? (
              <button
                type="button"
                className="btn-primary review-card__reveal"
                onClick={() => setShowAnswer(true)}
              >
                显示译文
              </button>
            ) : (
              <>
                <p className="review-card__answer">{current.translation}</p>
                <div className="review-card__actions">
                  <button
                    type="button"
                    className="btn-primary"
                    disabled={answered}
                    onClick={() => void onRemembered(true)}
                  >
                    记得
                  </button>
                  <button
                    type="button"
                    className="btn-secondary"
                    disabled={answered}
                    onClick={() => void onRemembered(false)}
                  >
                    不熟
                  </button>
                  <button type="button" className="btn-ghost" onClick={skip}>
                    跳过本题
                  </button>
                </div>
              </>
            )}
          </div>
        </div>
      ) : null}
    </>
  );
}
