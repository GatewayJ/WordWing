import { useEffect, useRef, useState } from "react";
import { CircleAlert, LoaderCircle, Square, Volume2 } from "lucide-react";
import {
  canPronounce,
  playPronunciation,
  preparePronunciationPlayback,
  PronunciationCancelledError,
  type PronunciationPlayback,
} from "../lib/pronunciation";

type PronounceButtonProps = {
  text: string;
  language?: string;
  className?: string;
};

type PlaybackState = "idle" | "loading" | "playing" | "error";

export function PronounceButton({ text, language, className = "" }: PronounceButtonProps) {
  const [state, setState] = useState<PlaybackState>("idle");
  const [detail, setDetail] = useState("使用 DashScope 发音；不可用时自动改用系统语音");
  const playbackRef = useRef<PronunciationPlayback | null>(null);
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      playbackRef.current?.stop();
      playbackRef.current = null;
    };
  }, []);

  const disabled = !canPronounce(text);
  const onClick = async () => {
    if (disabled || state === "loading") return;
    if (state === "playing") {
      playbackRef.current?.stop();
      playbackRef.current = null;
      setState("idle");
      return;
    }

    preparePronunciationPlayback();
    setState("loading");
    setDetail("正在准备发音…");
    try {
      const playback = await playPronunciation(text, language);
      if (!mountedRef.current) {
        playback.stop();
        return;
      }
      playbackRef.current = playback;
      setState("playing");
      setDetail(
        playback.source === "system"
          ? "正在使用系统语音（云端服务暂不可用）"
          : playback.cached
            ? "正在播放本地缓存发音"
            : "正在播放 DashScope 发音",
      );
      void playback.done.then(() => {
        if (!mountedRef.current || playbackRef.current !== playback) return;
        playbackRef.current = null;
        setState("idle");
        setDetail("使用 DashScope 发音；不可用时自动改用系统语音");
      });
    } catch (error) {
      if (!mountedRef.current) return;
      if (error instanceof PronunciationCancelledError) {
        setState("idle");
        return;
      }
      setState("error");
      setDetail(String(error));
    }
  };

  const label = state === "playing" ? `停止朗读：${text}` : `朗读：${text}`;
  const classes = [
    "btn-icon",
    "pronunciation-button",
    `pronunciation-button--${state}`,
    className,
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <button
      type="button"
      className={classes}
      disabled={disabled}
      aria-label={label}
      title={disabled ? "文本为空或超过 200 个字符，无法发音" : detail}
      onClick={() => void onClick()}
    >
      {state === "loading" ? (
        <LoaderCircle size={16} aria-hidden />
      ) : state === "playing" ? (
        <Square size={13} fill="currentColor" aria-hidden />
      ) : state === "error" ? (
        <CircleAlert size={16} aria-hidden />
      ) : (
        <Volume2 size={17} aria-hidden />
      )}
    </button>
  );
}
