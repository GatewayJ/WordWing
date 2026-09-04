import { useCallback, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { Clock3, Sparkles } from "lucide-react";

type ArticleSegment =
  | { kind: "text"; c: string }
  | { kind: "vocab"; c: string };

type SavedArticle = {
  title: string;
  generated_at_rfc3339: string;
  week_label_zh: string;
  segments: ArticleSegment[];
  source_phrases: string[];
};

type WeeklyStatusDto = {
  can_generate_this_week: boolean;
  week_label_zh: string;
  article: SavedArticle | null;
  history: SavedArticle[];
  new_phrase_count: number;
  block_reason: string | null;
  dashscope_configured: boolean;
};

function formatGeneratedAt(iso: string) {
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

export function WeeklyPage() {
  const [status, setStatus] = useState<WeeklyStatusDto | null>(null);
  const [loading, setLoading] = useState(true);
  const [generating, setGenerating] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [selectedGeneratedAt, setSelectedGeneratedAt] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setErr(null);
    try {
      const s = await invoke<WeeklyStatusDto>("get_weekly_article_status");
      setStatus(s);
    } catch (e) {
      setErr(String(e));
      setStatus(null);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    let un: UnlistenFn | undefined;
    void listen("vocabulary-changed", () => {
      void refresh();
    }).then((fn) => {
      un = fn;
    });
    return () => {
      void un?.();
    };
  }, [refresh]);

  useEffect(() => {
    let un: UnlistenFn | undefined;
    void listen("weekly-article-changed", () => {
      void refresh();
    }).then((fn) => {
      un = fn;
    });
    return () => {
      void un?.();
    };
  }, [refresh]);

  const onGenerate = async () => {
    setGenerating(true);
    setErr(null);
    try {
      await invoke<SavedArticle>("generate_weekly_article");
      setSelectedGeneratedAt(null);
      await refresh();
    } catch (e) {
      setErr(String(e));
    } finally {
      setGenerating(false);
    }
  };

  const canClick =
    (status?.new_phrase_count ?? 0) > 0 &&
    (status?.dashscope_configured ?? false) &&
    !generating &&
    !loading;

  const disabledReason = !status
    ? null
    : !status.dashscope_configured
      ? "需要配置 DASHSCOPE_API_KEY（与翻译浮层相同）后才能生成。"
      : status.new_phrase_count === 0
        ? "本自然周内还没有收藏词条。请先在「收藏」中加入词条后再生成。"
        : null;

  const history = status?.history ?? [];
  const article = selectedGeneratedAt
    ? history.find((item) => item.generated_at_rfc3339 === selectedGeneratedAt) ?? status?.article ?? null
    : status?.article ?? null;

  const translateWord = (word: string) => {
    void invoke("retry_translate_with_text", { source: word }).catch((e) => setErr(String(e)));
  };

  const renderWords = (text: string) => {
    const parts = text.split(/(\p{L}[\p{L}\p{M}'’-]*|\p{N}+)/gu);
    return parts.map((part, index) =>
      /^(\p{L}[\p{L}\p{M}'’-]*|\p{N}+)$/u.test(part) ? (
        <span
          key={index}
          className="weekly-word"
          title="双击翻译并可收藏"
          onDoubleClick={() => translateWord(part)}
        >
          {part}
        </span>
      ) : (
        <span key={index}>{part}</span>
      ),
    );
  };

  return (
    <>
      <h2 className="page-title">周短文</h2>
      <p className="page-lead">
        按<strong>当前自然周</strong>（ISO 周，周一为一周之始）内加入<strong>收藏</strong>的原文短语组稿；每次生成都会保留历史。正文里用下划线标出所用词条，<strong>双击任意单词</strong>可打开独立翻译浮层并收藏。配合{" "}
        <Link to="/english/vocabulary">生词</Link>、<Link to="/english/collection">收藏</Link> 与{" "}
        <Link to="/english/review">复习</Link> 使用。
      </p>

      {err && (
        <p className="page-lead" style={{ color: "var(--error)" }}>
          {err}
        </p>
      )}

      <div className="weekly-hero card">
        <div className="weekly-hero__grid">
          <div className="weekly-hero__intro">
            <span className="weekly-hero__eyebrow">本周</span>
            <p className="weekly-hero__week">
              {loading ? "…" : status?.week_label_zh ?? "—"}
            </p>
            {!loading && status && !status.dashscope_configured ? (
              <p className="weekly-hero__hint">
                需要已配置 <code>DASHSCOPE_API_KEY</code>（与翻译浮层相同）。生成会调用通义文本模型。
              </p>
            ) : null}
          </div>
          <div className="weekly-hero__panel">
            <div className="weekly-hero__stats">
              <span className="weekly-hero__chip">
                可纳入组稿的新收藏{" "}
                <strong>{loading ? "…" : (status?.new_phrase_count ?? 0)}</strong> 条
              </span>
            </div>
            <button
              type="button"
              className="weekly-hero__generate btn-primary"
              disabled={!canClick}
              onClick={() => void onGenerate()}
            >
              <Sparkles size={18} strokeWidth={2} aria-hidden />
              {generating ? "正在生成…" : "生成本周短文"}
            </button>
            {disabledReason && !generating ? (
              <p className="weekly-hero__disabled-msg">{disabledReason}</p>
            ) : null}
          </div>
        </div>
      </div>

      {article ? (
        <article className="weekly-article card weekly-article--generated">
          <header className="weekly-article__head">
            <div>
              <p className="weekly-article__label">{article.week_label_zh}</p>
              <h3 className="weekly-article__title">{article.title}</h3>
            </div>
            <p className="weekly-article__generated">
              生成于 {formatGeneratedAt(article.generated_at_rfc3339)}
            </p>
          </header>
          <div className="weekly-article__body weekly-article__body--segments">
            <p className="weekly-article__para">
              {article.segments.map((seg, i) =>
                seg.kind === "vocab" ? (
                  <span key={i} className="weekly-vocab-underline">
                    {renderWords(seg.c)}
                  </span>
                ) : (
                  <span key={i}>{renderWords(seg.c)}</span>
                ),
              )}
            </p>
          </div>
        </article>
      ) : !loading ? (
        <div className="card weekly-empty">
          <p className="weekly-empty__title">还没有本周短文</p>
          <p className="weekly-empty__text">
            在本自然周内先往「收藏」加入词条，再点击上方生成。每次组稿使用<strong>本周内收藏</strong>的词条（最多 36 条）；未展示短文时也会因尚无生成记录而显示本提示。
          </p>
        </div>
      ) : null}

      {history.length > 0 ? (
        <section className="weekly-history card" aria-label="短文历史">
          <div className="weekly-history__head">
            <Clock3 size={18} aria-hidden />
            <h3>生成历史</h3>
            <span>{history.length} 篇</span>
          </div>
          <div className="weekly-history__list">
            {history.map((item) => {
              const active = article?.generated_at_rfc3339 === item.generated_at_rfc3339;
              return (
                <button
                  type="button"
                  className={active ? "weekly-history__item weekly-history__item--active" : "weekly-history__item"}
                  key={item.generated_at_rfc3339}
                  onClick={() => setSelectedGeneratedAt(item.generated_at_rfc3339)}
                >
                  <strong>{item.title}</strong>
                  <span>{item.week_label_zh} · {formatGeneratedAt(item.generated_at_rfc3339)}</span>
                </button>
              );
            })}
          </div>
        </section>
      ) : null}
    </>
  );
}
