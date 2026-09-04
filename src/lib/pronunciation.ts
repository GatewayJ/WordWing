import { invoke } from "@tauri-apps/api/core";

const MAX_TEXT_CHARS = 200;

type PronunciationAudio = {
  mimeType: string;
  dataBase64: string;
  provider: string;
  language: string;
  voice: string;
  cached: boolean;
};

export type PronunciationPlayback = {
  source: "dashscope" | "system";
  cached: boolean;
  fallbackReason?: string;
  done: Promise<void>;
  stop: () => void;
};

export class PronunciationCancelledError extends Error {
  constructor() {
    super("发音播放已取消");
    this.name = "PronunciationCancelledError";
  }
}

let playbackSequence = 0;
let activeStop: (() => void) | null = null;
let sharedAudioContext: AudioContext | null = null;

type AudioContextConstructor = new () => AudioContext;

/**
 * 必须在原始点击事件中调用，以免等待云端请求后被 WebKit 的自动播放策略拦截。
 */
export function preparePronunciationPlayback(): void {
  const audioWindow = window as typeof window & {
    webkitAudioContext?: AudioContextConstructor;
  };
  const AudioContextClass = window.AudioContext ?? audioWindow.webkitAudioContext;
  if (!AudioContextClass) return;
  if (!sharedAudioContext || sharedAudioContext.state === "closed") {
    sharedAudioContext = new AudioContextClass();
  }
  if (sharedAudioContext.state === "suspended") {
    void sharedAudioContext.resume().catch(() => undefined);
  }
}

export function normalizedPronunciationText(text: string): string {
  return text.trim().replace(/\s+/g, " ");
}

export function canPronounce(text: string): boolean {
  const normalized = normalizedPronunciationText(text);
  return normalized.length > 0 && Array.from(normalized).length <= MAX_TEXT_CHARS;
}

export function stopActivePronunciation() {
  playbackSequence += 1;
  activeStop?.();
  activeStop = null;
}

export async function playPronunciation(
  text: string,
  language?: string,
): Promise<PronunciationPlayback> {
  const normalized = normalizedPronunciationText(text);
  if (!normalized) throw new Error("没有可朗读的文本");
  if (Array.from(normalized).length > MAX_TEXT_CHARS) {
    throw new Error(`单次最多朗读 ${MAX_TEXT_CHARS} 个字符`);
  }

  preparePronunciationPlayback();
  stopActivePronunciation();
  const sequence = playbackSequence;
  let cloudError = "";
  try {
    const payload = await invoke<PronunciationAudio>("get_pronunciation_audio", {
      text: normalized,
      language: language ?? null,
    });
    if (sequence !== playbackSequence) throw new PronunciationCancelledError();
    return await startAudioPlayback(payload);
  } catch (error) {
    if (error instanceof PronunciationCancelledError || sequence !== playbackSequence) {
      throw new PronunciationCancelledError();
    }
    cloudError = String(error);
  }

  try {
    return startSystemPlayback(normalized, language, cloudError);
  } catch (systemError) {
    throw new Error(`${cloudError}；系统语音也不可用：${String(systemError)}`);
  }
}

async function startAudioPlayback(
  payload: PronunciationAudio,
): Promise<PronunciationPlayback> {
  if (!/^audio\/(wav|mpeg|ogg|opus)$/.test(payload.mimeType)) {
    throw new Error(`不支持的发音音频格式：${payload.mimeType}`);
  }
  const bytes = decodeBase64(payload.dataBase64);
  if (sharedAudioContext) {
    try {
      return await startWebAudioPlayback(sharedAudioContext, bytes, payload.cached);
    } catch {
      // 个别 WebKit 音频解码器不支持服务端格式时，再尝试 HTMLAudioElement。
    }
  }
  return await startHtmlAudioPlayback(bytes, payload);
}

async function startWebAudioPlayback(
  context: AudioContext,
  bytes: Uint8Array,
  cached: boolean,
): Promise<PronunciationPlayback> {
  const copiedBuffer = bytes.buffer.slice(
    bytes.byteOffset,
    bytes.byteOffset + bytes.byteLength,
  ) as ArrayBuffer;
  const audioBuffer = await context.decodeAudioData(copiedBuffer);
  if (context.state === "suspended") await context.resume();
  const source = context.createBufferSource();
  source.buffer = audioBuffer;
  source.connect(context.destination);

  let finish!: () => void;
  const done = new Promise<void>((resolve) => {
    let settled = false;
    finish = () => {
      if (settled) return;
      settled = true;
      if (activeStop === stop) activeStop = null;
      resolve();
    };
  });
  const stop = () => {
    source.onended = null;
    try {
      source.stop();
    } catch {
      // 已自然结束时 stop 会抛 InvalidStateError，收尾逻辑仍需继续。
    }
    source.disconnect();
    finish();
  };
  source.onended = finish;
  activeStop = stop;
  source.start();
  return {
    source: "dashscope",
    cached,
    done,
    stop,
  };
}

async function startHtmlAudioPlayback(
  bytes: Uint8Array,
  payload: PronunciationAudio,
): Promise<PronunciationPlayback> {
  const blob = new Blob([bytes], { type: payload.mimeType });
  const objectUrl = URL.createObjectURL(blob);
  const audio = new Audio(objectUrl);
  audio.preload = "auto";

  let finish!: () => void;
  const done = new Promise<void>((resolve) => {
    let settled = false;
    finish = () => {
      if (settled) return;
      settled = true;
      URL.revokeObjectURL(objectUrl);
      if (activeStop === stop) activeStop = null;
      resolve();
    };
  });
  const stop = () => {
    audio.pause();
    audio.removeAttribute("src");
    audio.load();
    finish();
  };
  audio.addEventListener("ended", finish, { once: true });
  audio.addEventListener("error", finish, { once: true });
  activeStop = stop;
  try {
    await audio.play();
  } catch (error) {
    stop();
    throw error;
  }
  return {
    source: "dashscope",
    cached: payload.cached,
    done,
    stop,
  };
}

function decodeBase64(data: string): Uint8Array {
  const binary = window.atob(data);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
}

function startSystemPlayback(
  text: string,
  language: string | undefined,
  fallbackReason: string,
): PronunciationPlayback {
  if (!("speechSynthesis" in window) || typeof SpeechSynthesisUtterance === "undefined") {
    throw new Error("当前 WebView 没有提供 SpeechSynthesis");
  }
  const synth = window.speechSynthesis;
  const utterance = new SpeechSynthesisUtterance(text);
  const chinese = language
    ? /^(zh|chinese)/i.test(language)
    : /[\u3400-\u9fff]/u.test(text);
  utterance.lang = chinese ? "zh-CN" : "en-US";
  utterance.rate = chinese ? 0.95 : 0.88;
  const languagePrefix = chinese ? "zh" : "en";
  const matchingVoice = synth
    .getVoices()
    .find((voice) => voice.lang.toLocaleLowerCase().startsWith(languagePrefix));
  if (matchingVoice) utterance.voice = matchingVoice;

  let finish!: () => void;
  const done = new Promise<void>((resolve) => {
    let settled = false;
    finish = () => {
      if (settled) return;
      settled = true;
      if (activeStop === stop) activeStop = null;
      resolve();
    };
  });
  const stop = () => {
    synth.cancel();
    finish();
  };
  utterance.addEventListener("end", finish, { once: true });
  utterance.addEventListener("error", finish, { once: true });
  activeStop = stop;
  synth.cancel();
  synth.speak(utterance);
  return {
    source: "system",
    cached: false,
    fallbackReason,
    done,
    stop,
  };
}
