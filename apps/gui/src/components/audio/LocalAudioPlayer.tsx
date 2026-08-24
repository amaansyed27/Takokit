import { LoaderCircle, Play, Volume2 } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { loadLocalAudio } from "../../lib/audioPreview";

type LocalAudioPlayerProps = {
  path: string;
  compact?: boolean;
  defer?: boolean;
  label?: string;
};

export function LocalAudioPlayer({ path, compact = false, defer = false, label = "Audio preview" }: LocalAudioPlayerProps) {
  const [url, setUrl] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [shouldAutoplay, setShouldAutoplay] = useState(false);
  const audioRef = useRef<HTMLAudioElement | null>(null);

  useEffect(() => {
    setError(null);
    setShouldAutoplay(false);
    setUrl((previous) => {
      if (previous) URL.revokeObjectURL(previous);
      return null;
    });

    if (!path.trim() || defer) return;
    void load(false);

    return () => {
      setUrl((previous) => {
        if (previous) URL.revokeObjectURL(previous);
        return null;
      });
    };
    // Loading is intentionally keyed only to the selected local path.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [path, defer]);

  useEffect(() => {
    if (!url || !shouldAutoplay) return;
    void audioRef.current?.play().catch(() => undefined);
    setShouldAutoplay(false);
  }, [url, shouldAutoplay]);

  async function load(autoplay: boolean) {
    if (!path.trim() || loading) return;
    setLoading(true);
    setError(null);
    setShouldAutoplay(autoplay);
    try {
      const next = await loadLocalAudio(path);
      setUrl((previous) => {
        if (previous) URL.revokeObjectURL(previous);
        return next;
      });
    } catch (caught) {
      setShouldAutoplay(false);
      setError(caught instanceof Error ? caught.message : "Audio preview is unavailable.");
    } finally {
      setLoading(false);
    }
  }

  if (!path.trim()) return null;

  if (!url) {
    return (
      <div className={compact ? "tk-audio-player is-compact" : "tk-audio-player"}>
        <span className="tk-audio-player__icon">
          {loading ? <LoaderCircle className="is-spinning" size={16} /> : <Volume2 size={16} strokeWidth={1.8} />}
        </span>
        <div className="tk-audio-player__copy">
          <strong>{label}</strong>
          <span>{error ?? (loading ? "Loading local audio…" : "Listen before continuing")}</span>
        </div>
        <button type="button" disabled={loading} onClick={() => void load(true)}>
          <Play size={13} fill="currentColor" />
          {loading ? "Loading" : "Play"}
        </button>
      </div>
    );
  }

  return (
    <div className={compact ? "tk-audio-player is-compact is-loaded" : "tk-audio-player is-loaded"}>
      <span className="tk-audio-player__icon"><Volume2 size={16} strokeWidth={1.8} /></span>
      <audio
        ref={audioRef}
        controls
        preload="metadata"
        src={url}
        onError={() => setError("This browser cannot play the selected audio format.")}
      />
      {error ? <span className="tk-audio-player__error">{error}</span> : null}
    </div>
  );
}
