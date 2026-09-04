//! 单词/短语发音：DashScope Qwen3-TTS 生成、立即下载并缓存在应用数据目录。

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use reqwest::{header::CONTENT_TYPE, Client, Url};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};
use tokio::sync::Mutex;
use uuid::Uuid;

const MODEL: &str = "qwen3-tts-flash";
const ENDPOINT: &str =
    "https://dashscope.aliyuncs.com/api/v1/services/aigc/multimodal-generation/generation";
const MAX_TEXT_CHARS: usize = 200;
const MAX_AUDIO_BYTES: usize = 8 * 1024 * 1024;
const MAX_CACHE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_CACHE_FILES: usize = 2_000;

#[derive(Debug, Clone, Copy)]
struct VoiceProfile {
    language: &'static str,
    voice: &'static str,
}

impl VoiceProfile {
    fn detect(text: &str, language_hint: Option<&str>) -> Self {
        let hint = language_hint
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        let chinese = hint.starts_with("zh")
            || hint.contains("chinese")
            || (hint.is_empty() && crate::translate::is_mostly_chinese(text));
        if chinese {
            Self {
                language: "Chinese",
                voice: "Cherry",
            }
        } else {
            Self {
                language: "English",
                voice: "Jennifer",
            }
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PronunciationAudio {
    mime_type: String,
    data_base64: String,
    provider: &'static str,
    language: &'static str,
    voice: &'static str,
    cached: bool,
}

impl PronunciationAudio {
    pub fn cached(&self) -> bool {
        self.cached
    }

    pub fn encoded_len(&self) -> usize {
        self.data_base64.len()
    }
}

struct CachedAudio {
    bytes: Vec<u8>,
    mime_type: &'static str,
}

pub struct PronunciationService {
    cache_dir: PathBuf,
    client: Client,
    /// 避免同一个词被连续点击时重复计费；锁内会再次检查缓存。
    generation_lock: Mutex<()>,
}

impl PronunciationService {
    pub fn new(app_data_dir: &Path) -> Result<Self, String> {
        let cache_dir = app_data_dir.join("pronunciation-cache");
        fs::create_dir_all(&cache_dir).map_err(|e| format!("创建发音缓存目录失败: {e}"))?;
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .user_agent("WordWing/0.1 pronunciation")
            .build()
            .map_err(|e| format!("创建发音请求客户端失败: {e}"))?;
        Ok(Self {
            cache_dir,
            client,
            generation_lock: Mutex::new(()),
        })
    }

    pub async fn get_audio(
        &self,
        text: &str,
        language_hint: Option<&str>,
    ) -> Result<PronunciationAudio, String> {
        let text = normalize_text(text)?;
        let profile = VoiceProfile::detect(&text, language_hint);
        let cache_key = cache_key(&text, profile);

        if let Some(audio) = self.read_cached(&cache_key)? {
            return Ok(to_payload(audio, profile, true));
        }

        let _guard = self.generation_lock.lock().await;
        if let Some(audio) = self.read_cached(&cache_key)? {
            return Ok(to_payload(audio, profile, true));
        }

        let api_key = std::env::var("DASHSCOPE_API_KEY")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                "未配置 DASHSCOPE_API_KEY，已尝试切换到系统语音。systemd 安装请写入 ~/.config/wordwing-env 后重启服务。"
                    .to_string()
            })?;
        let audio_url = self
            .request_audio_url(api_key.trim(), &text, profile)
            .await?;
        let audio = self.download_audio(&audio_url).await?;
        self.write_cached(&cache_key, &audio)?;
        self.prune_cache();
        Ok(to_payload(audio, profile, false))
    }

    async fn request_audio_url(
        &self,
        api_key: &str,
        text: &str,
        profile: VoiceProfile,
    ) -> Result<String, String> {
        let body = serde_json::json!({
            "model": MODEL,
            "input": {
                "text": text,
                "voice": profile.voice,
                "language_type": profile.language
            }
        });
        let response = self
            .client
            .post(ENDPOINT)
            .bearer_auth(api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("发音服务连接失败: {e}"))?;
        let status = response.status();
        let value: Value = response
            .json()
            .await
            .map_err(|e| format!("发音服务响应无法解析: {e}"))?;
        if !status.is_success() {
            return Err(api_error_message(&value, "发音生成失败"));
        }
        ["/output/audio/url", "/output/url", "/output/audio_url"]
            .iter()
            .find_map(|pointer| value.pointer(pointer).and_then(Value::as_str))
            .filter(|url| !url.trim().is_empty())
            .map(str::to_string)
            .ok_or_else(|| api_error_message(&value, "发音服务未返回音频地址"))
    }

    async fn download_audio(&self, audio_url: &str) -> Result<CachedAudio, String> {
        let url = validate_audio_url(audio_url)?;
        let response = self
            .client
            .get(url.clone())
            .send()
            .await
            .map_err(|e| format!("下载发音音频失败: {e}"))?;
        if !response.status().is_success() {
            return Err(format!("下载发音音频失败（HTTP {}）", response.status()));
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_AUDIO_BYTES as u64)
        {
            return Err("发音音频超过 8 MiB 安全限制".to_string());
        }
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_string();
        let bytes = response
            .bytes()
            .await
            .map_err(|e| format!("读取发音音频失败: {e}"))?
            .to_vec();
        if bytes.is_empty() {
            return Err("发音服务返回了空音频".to_string());
        }
        if bytes.len() > MAX_AUDIO_BYTES {
            return Err("发音音频超过 8 MiB 安全限制".to_string());
        }
        let mime_type = detect_audio_format(&content_type, &url)
            .ok_or_else(|| format!("发音服务返回了不支持的音频格式: {content_type}"))?;
        Ok(CachedAudio { bytes, mime_type })
    }

    fn read_cached(&self, cache_key: &str) -> Result<Option<CachedAudio>, String> {
        for (extension, mime_type) in supported_formats() {
            let path = self.cache_dir.join(format!("{cache_key}.{extension}"));
            if !path.exists() {
                continue;
            }
            let bytes = fs::read(&path).map_err(|e| format!("读取发音缓存失败: {e}"))?;
            if bytes.is_empty() || bytes.len() > MAX_AUDIO_BYTES {
                let _ = fs::remove_file(path);
                continue;
            }
            return Ok(Some(CachedAudio { bytes, mime_type }));
        }
        Ok(None)
    }

    fn write_cached(&self, cache_key: &str, audio: &CachedAudio) -> Result<(), String> {
        let extension = extension_for_mime(audio.mime_type)
            .ok_or_else(|| "无法确定发音缓存格式".to_string())?;
        let final_path = self.cache_dir.join(format!("{cache_key}.{extension}"));
        let temp_path = self
            .cache_dir
            .join(format!(".{cache_key}.{}.tmp", Uuid::new_v4().simple()));
        fs::write(&temp_path, &audio.bytes).map_err(|e| format!("写入发音缓存失败: {e}"))?;
        if let Err(error) = fs::rename(&temp_path, &final_path) {
            let _ = fs::remove_file(&temp_path);
            return Err(format!("保存发音缓存失败: {error}"));
        }
        Ok(())
    }

    fn prune_cache(&self) {
        let Ok(entries) = fs::read_dir(&self.cache_dir) else {
            return;
        };
        let mut files = entries
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let path = entry.path();
                let extension = path.extension()?.to_str()?;
                if !supported_formats()
                    .iter()
                    .any(|(candidate, _)| *candidate == extension)
                {
                    return None;
                }
                let metadata = entry.metadata().ok()?;
                Some((
                    path,
                    metadata.len(),
                    metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                ))
            })
            .collect::<Vec<_>>();
        let mut total_bytes = files.iter().map(|(_, size, _)| *size).sum::<u64>();
        if files.len() <= MAX_CACHE_FILES && total_bytes <= MAX_CACHE_BYTES {
            return;
        }
        files.sort_by_key(|(_, _, modified)| *modified);
        let mut total_files = files.len();
        for (path, size, _) in files {
            if total_files <= MAX_CACHE_FILES && total_bytes <= MAX_CACHE_BYTES {
                break;
            }
            if fs::remove_file(path).is_ok() {
                total_files = total_files.saturating_sub(1);
                total_bytes = total_bytes.saturating_sub(size);
            }
        }
    }
}

fn normalize_text(text: &str) -> Result<String, String> {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        return Err("没有可朗读的文本".to_string());
    }
    if normalized.chars().count() > MAX_TEXT_CHARS {
        return Err(format!("单次最多朗读 {MAX_TEXT_CHARS} 个字符"));
    }
    Ok(normalized)
}

fn cache_key(text: &str, profile: VoiceProfile) -> String {
    let mut digest = Sha256::new();
    digest.update(format!(
        "v1\n{MODEL}\n{}\n{}\n{text}",
        profile.language, profile.voice
    ));
    digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn api_error_message(value: &Value, fallback: &str) -> String {
    let message = value
        .pointer("/message")
        .and_then(Value::as_str)
        .or_else(|| value.pointer("/output/message").and_then(Value::as_str))
        .unwrap_or(fallback);
    let code = value.pointer("/code").and_then(Value::as_str);
    match code {
        Some(code) => format!("{message}（{code}）"),
        None => message.to_string(),
    }
}

fn validate_audio_url(value: &str) -> Result<Url, String> {
    let mut url = Url::parse(value).map_err(|_| "发音服务返回了无效的音频地址".to_string())?;
    let host = url
        .host_str()
        .ok_or_else(|| "发音音频地址缺少主机名".to_string())?
        .to_ascii_lowercase();
    if host == "localhost" || host.ends_with(".localhost") || host.ends_with(".local") {
        return Err("拒绝访问本地发音音频地址".to_string());
    }
    match url.scheme() {
        "https" => {}
        // DashScope 当前会返回阿里云 OSS 的 HTTP 签名地址；OSS 同时支持 HTTPS。
        // 只对阿里云自有 OSS 域名升级协议，不允许访问任意明文 HTTP 地址。
        "http" if is_aliyun_oss_host(&host) => {
            url.set_scheme("https")
                .map_err(|_| "无法将发音音频地址升级为 HTTPS".to_string())?;
        }
        _ => return Err("发音音频地址不是安全的 HTTPS 地址".to_string()),
    }
    Ok(url)
}

fn is_aliyun_oss_host(host: &str) -> bool {
    host.ends_with(".aliyuncs.com") && host.split('.').any(|label| label.starts_with("oss-"))
}

fn supported_formats() -> &'static [(&'static str, &'static str)] {
    &[
        ("wav", "audio/wav"),
        ("mp3", "audio/mpeg"),
        ("ogg", "audio/ogg"),
        ("opus", "audio/opus"),
    ]
}

fn detect_audio_format(content_type: &str, url: &Url) -> Option<&'static str> {
    let mime = content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    match mime.as_str() {
        "audio/wav" | "audio/x-wav" | "audio/vnd.wave" => Some("audio/wav"),
        "audio/mpeg" | "audio/mp3" => Some("audio/mpeg"),
        "audio/ogg" | "application/ogg" => Some("audio/ogg"),
        "audio/opus" => Some("audio/opus"),
        _ => match url
            .path()
            .rsplit_once('.')
            .map(|(_, extension)| extension.to_ascii_lowercase())
            .as_deref()
        {
            Some("wav") => Some("audio/wav"),
            Some("mp3") => Some("audio/mpeg"),
            Some("ogg") => Some("audio/ogg"),
            Some("opus") => Some("audio/opus"),
            _ => None,
        },
    }
}

fn extension_for_mime(mime_type: &str) -> Option<&'static str> {
    supported_formats()
        .iter()
        .find_map(|(extension, mime)| (*mime == mime_type).then_some(*extension))
}

fn to_payload(audio: CachedAudio, profile: VoiceProfile, cached: bool) -> PronunciationAudio {
    PronunciationAudio {
        mime_type: audio.mime_type.to_string(),
        data_base64: BASE64.encode(audio.bytes),
        provider: "dashscope-qwen3-tts",
        language: profile.language,
        voice: profile.voice,
        cached,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        cache_key, detect_audio_format, normalize_text, validate_audio_url, VoiceProfile,
        MAX_TEXT_CHARS,
    };
    use reqwest::Url;

    #[test]
    fn detects_chinese_and_english_voice_profiles() {
        let english = VoiceProfile::detect("hello", None);
        assert_eq!(english.language, "English");
        assert_eq!(english.voice, "Jennifer");
        let chinese = VoiceProfile::detect("你好", None);
        assert_eq!(chinese.language, "Chinese");
        assert_eq!(chinese.voice, "Cherry");
    }

    #[test]
    fn explicit_language_hint_wins() {
        assert_eq!(
            VoiceProfile::detect("hello", Some("zh-CN")).language,
            "Chinese"
        );
        assert_eq!(
            VoiceProfile::detect("你好", Some("English")).language,
            "English"
        );
    }

    #[test]
    fn normalizes_and_limits_text() {
        assert_eq!(normalize_text("  hello\n world ").unwrap(), "hello world");
        assert!(normalize_text("").is_err());
        assert!(normalize_text(&"a".repeat(MAX_TEXT_CHARS + 1)).is_err());
    }

    #[test]
    fn cache_key_includes_voice_profile() {
        let english = VoiceProfile::detect("hello", None);
        let chinese = VoiceProfile::detect("hello", Some("Chinese"));
        assert_eq!(cache_key("hello", english), cache_key("hello", english));
        assert_ne!(cache_key("hello", english), cache_key("hello", chinese));
    }

    #[test]
    fn accepts_playable_formats_and_safe_urls() {
        let url = Url::parse("https://example.aliyuncs.com/audio/test.wav?token=x").unwrap();
        assert_eq!(
            detect_audio_format("application/octet-stream", &url),
            Some("audio/wav")
        );
        assert!(validate_audio_url(url.as_str()).is_ok());
        let upgraded = validate_audio_url(
            "http://dashscope-result-bj.oss-cn-beijing.aliyuncs.com/audio/test.wav?token=x",
        )
        .unwrap();
        assert_eq!(upgraded.scheme(), "https");
        assert!(validate_audio_url("http://example.com/test.wav").is_err());
        assert!(validate_audio_url("http://localhost/test.wav").is_err());
    }
}
