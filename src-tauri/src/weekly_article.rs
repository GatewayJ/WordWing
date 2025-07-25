use chrono::{DateTime, Datelike, Local, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Mutex;

use crate::translate::dashscope_api_key_configured;
use crate::vocabulary::{VocabItem, VocabStore};

const MAX_PHRASES: usize = 36;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WeeklyFileState {
    #[serde(default)]
    pub last_generated_iso_week_year: Option<i32>,
    #[serde(default)]
    pub last_generated_iso_week: Option<u32>,
    /// 保留字段；当前版本组稿范围改为「本自然周内收藏」，不再用截断时间筛选。
    #[serde(default)]
    pub last_generation_cutoff_rfc3339: Option<String>,
    #[serde(default)]
    pub article: Option<SavedArticle>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedArticle {
    pub title: String,
    #[serde(alias = "generatedAtRfc3339")]
    pub generated_at_rfc3339: String,
    #[serde(alias = "weekLabelZh")]
    pub week_label_zh: String,
    pub segments: Vec<ArticleSegment>,
    /// 参与组稿的原文短语（生词 source_text）
    #[serde(default, alias = "sourcePhrases")]
    pub source_phrases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ArticleSegment {
    Text { c: String },
    Vocab { c: String },
}

/// 通过 Tauri 返回前端：保持 snake_case 字段名，与前端 TypeScript 一致（勿用 camelCase，否则 `dashscope_configured` 等会读不到）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklyStatusDto {
    pub can_generate_this_week: bool,
    pub week_label_zh: String,
    pub article: Option<SavedArticle>,
    pub new_phrase_count: usize,
    pub block_reason: Option<String>,
    /// 已配置 DASHSCOPE_API_KEY（与翻译浮层相同）；前端据此决定是否展示「需配置 Key」说明。
    pub dashscope_configured: bool,
}

pub struct WeeklyArticleStore {
    tree: sled::Tree,
    inner: Mutex<WeeklyFileState>,
}

impl WeeklyArticleStore {
    pub fn open(db: &sled::Db) -> Result<Self, String> {
        let tree = db.open_tree("weekly").map_err(|e| e.to_string())?;
        let state = match tree.get(b"state").map_err(|e| e.to_string())? {
            None => WeeklyFileState::default(),
            Some(v) => {
                if v.is_empty() {
                    WeeklyFileState::default()
                } else {
                    serde_json::from_slice(&v).map_err(|e| e.to_string())?
                }
            }
        };
        Ok(Self {
            tree,
            inner: Mutex::new(state),
        })
    }

    fn persist_locked(state: &WeeklyFileState, tree: &sled::Tree) -> Result<(), String> {
        let raw = serde_json::to_vec(state).map_err(|e| e.to_string())?;
        tree.insert(b"state", raw).map_err(|e| e.to_string())?;
        tree.flush().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn week_label_zh() -> String {
        let d = Local::now().date_naive();
        let iw = d.iso_week();
        format!("{} 年第 {} 周", iw.year(), iw.week())
    }

    pub fn get_status(&self, vocab: &VocabStore) -> Result<WeeklyStatusDto, String> {
        let g = self.inner.lock().map_err(|e| e.to_string())?;
        let new_phrases = collect_phrases_for_generation(vocab)?;
        Ok(WeeklyStatusDto {
            // 暂时关闭「每自然周仅一次」限制，可随时再次生成。
            can_generate_this_week: true,
            week_label_zh: Self::week_label_zh(),
            article: g.article.clone(),
            new_phrase_count: new_phrases.len(),
            block_reason: None,
            dashscope_configured: dashscope_api_key_configured(),
        })
    }

    /// 取出本自然周（ISO 周，本地时区）内收藏的短语，供 LLM 组稿。
    pub fn take_phrases_for_llm(&self, vocab: &VocabStore) -> Result<Vec<String>, String> {
        let phrases = collect_phrases_for_generation(vocab)?;
        if phrases.is_empty() {
            return Err(
                "本自然周内（按 ISO 周）还没有可用于组稿的收藏词条。请先在「收藏」中加入词条。"
                    .to_string(),
            );
        }
        Ok(phrases)
    }

    /// LLM 返回后落盘（不更新「每周一次」相关字段，该限制已暂时关闭）。
    pub fn finish_generation(&self, raw_llm: &str, phrases: &[String]) -> Result<SavedArticle, String> {
        let mut g = self.inner.lock().map_err(|e| e.to_string())?;
        let parsed = parse_llm_segments_json(raw_llm, phrases)?;
        let now = Utc::now().to_rfc3339();
        let article = SavedArticle {
            title: parsed.title,
            generated_at_rfc3339: now.clone(),
            week_label_zh: Self::week_label_zh(),
            segments: parsed.segments,
            source_phrases: phrases.to_vec(),
        };
        g.article = Some(article.clone());
        Self::persist_locked(&g, &self.tree)?;
        Ok(article)
    }
}

/// 收藏创建时间落在「当前本地日期所在 ISO 周」内（与 `week_label_zh` 一致）。
fn created_in_current_iso_week(created_at_rfc3339: &str) -> bool {
    let Ok(dt) = DateTime::parse_from_rfc3339(created_at_rfc3339) else {
        return false;
    };
    let local = dt.with_timezone(&Local);
    let d = local.date_naive();
    let today = Local::now().date_naive();
    d.iso_week() == today.iso_week()
}

fn collect_phrases_for_generation(vocab: &VocabStore) -> Result<Vec<String>, String> {
    let list = vocab.list()?;
    let mut v: Vec<VocabItem> = list
        .into_iter()
        .filter(|x| x.starred)
        .filter(|x| created_in_current_iso_week(&x.created_at))
        .collect();
    v.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    v.truncate(MAX_PHRASES);
    Ok(v.into_iter().map(|x| x.source_text).collect())
}

#[derive(Deserialize)]
struct LlmEnvelope {
    title: String,
    segments: Vec<LlmSeg>,
}

#[derive(Deserialize)]
struct LlmSeg {
    #[serde(rename = "kind")]
    kind: String,
    #[serde(default)]
    c: String,
    #[serde(default)]
    text: String,
}

struct ParsedArticle {
    title: String,
    segments: Vec<ArticleSegment>,
}

fn parse_llm_segments_json(raw: &str, allowed: &[String]) -> Result<ParsedArticle, String> {
    let mut s = raw.trim().to_string();
    if let Some(rest) = s.strip_prefix("```json") {
        s = rest.to_string();
    } else if let Some(rest) = s.strip_prefix("```") {
        s = rest.to_string();
    }
    if let Some(idx) = s.rfind("```") {
        s.truncate(idx);
    }
    let s = s.trim();

    let env: LlmEnvelope = serde_json::from_str(s).map_err(|e| format!("模型返回非预期 JSON：{}", e))?;

    let set: HashSet<&str> = allowed.iter().map(|x| x.as_str()).collect();
    let mut out: Vec<ArticleSegment> = Vec::new();

    for seg in env.segments {
        let kind = seg.kind.to_lowercase();
        let content = if !seg.c.is_empty() {
            seg.c
        } else {
            seg.text
        };
        match kind.as_str() {
            "vocab" | "v" => {
                if set.contains(content.trim()) {
                    out.push(ArticleSegment::Vocab {
                        c: content.trim().to_string(),
                    });
                } else {
                    // 模型略偏离时降级为正文，避免破坏阅读
                    out.push(ArticleSegment::Text { c: content });
                }
            }
            "text" | "plain" | "t" => {
                if !content.is_empty() {
                    out.push(ArticleSegment::Text { c: content });
                }
            }
            _ => {
                if !content.is_empty() {
                    out.push(ArticleSegment::Text { c: content });
                }
            }
        }
    }

    if out.is_empty() {
        return Err("模型未返回有效正文段落。".to_string());
    }

    let title = env.title.trim();
    let title = if title.is_empty() {
        "Short reading".to_string()
    } else {
        title.to_string()
    };

    let merged = merge_adjacent_text(out);
    let segments = strip_trailing_vocab_after_sentence_end(merged);

    Ok(ParsedArticle { title, segments })
}

/// 模型常把未嵌入正文的多余 vocab 段堆在全文最后一个句号之后；按用户要求整段丢弃。
fn strip_trailing_vocab_after_sentence_end(segs: Vec<ArticleSegment>) -> Vec<ArticleSegment> {
    let mut segs = segs;
    let mut i = segs.len();
    while i > 0 && matches!(segs[i - 1], ArticleSegment::Vocab { .. }) {
        i -= 1;
    }
    if i == segs.len() {
        return segs;
    }
    match segs.get(i.saturating_sub(1)) {
        Some(ArticleSegment::Text { c }) if text_ends_sentence_terminator(c) => {
            segs.truncate(i);
        }
        _ => {}
    }
    segs
}

fn text_ends_sentence_terminator(text: &str) -> bool {
    let t = text.trim_end();
    if t.is_empty() {
        return false;
    }
    matches!(t.chars().last(), Some('.') | Some('?') | Some('!') | Some('…'))
}

fn merge_adjacent_text(segs: Vec<ArticleSegment>) -> Vec<ArticleSegment> {
    let mut merged: Vec<ArticleSegment> = Vec::new();
    for seg in segs {
        match (merged.last_mut(), seg) {
            (Some(ArticleSegment::Text { c: prev }), ArticleSegment::Text { c: next }) => {
                prev.push_str(&next);
            }
            (_, s) => merged.push(s),
        }
    }
    merged
}
