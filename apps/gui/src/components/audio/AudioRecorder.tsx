import { Check, Mic, RotateCcw, Square, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { uploadWorkspaceFile, type WorkspaceFile } from "../../lib/files";

type AudioRecorderProps = {
  onSaved: (file: WorkspaceFile) => void;
  compact?: boolean;
  label?: string;
  reviewBeforeSave?: boolean;
};

export function AudioRecorder({
  onSaved,
  compact = false,
  label = "Record audio",
  reviewBeforeSave = false
}: AudioRecorderProps) {
  const [recording, setRecording] = useState(false);
  const [saving, setSaving] = useState(false);
  const [seconds, setSeconds] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [reviewUrl, setReviewUrl] = useState<string | null>(null);
  const pendingBlobRef = useRef<Blob | null>(null);
  const pendingNameRef = useRef<string | null>(null);
  const recordingRef = useRef(false);
  const savingRef = useRef(false);
  const streamRef = useRef<MediaStream | null>(null);
  const contextRef = useRef<AudioContext | null>(null);
  const sourceRef = useRef<MediaStreamAudioSourceNode | null>(null);
  const processorRef = useRef<ScriptProcessorNode | null>(null);
  const muteRef = useRef<GainNode | null>(null);
  const chunksRef = useRef<Float32Array[]>([]);
  const sampleRateRef = useRef(48_000);
  const timerRef = useRef<number | null>(null);

  useEffect(() => () => cleanup(), []);

  async function start() {
    if (recordingRef.current || savingRef.current) return;
    clearReview();
    setError(null);
    if (!navigator.mediaDevices?.getUserMedia) {
      setError("Microphone recording is not available in this browser.");
      return;
    }

    try {
      const stream = await navigator.mediaDevices.getUserMedia({
        audio: {
          channelCount: 1,
          echoCancellation: true,
          noiseSuppression: true,
          autoGainControl: true
        }
      });
      const context = new AudioContext();
      const source = context.createMediaStreamSource(stream);
      const processor = context.createScriptProcessor(4096, 1, 1);
      const mute = context.createGain();
      mute.gain.value = 0;
      chunksRef.current = [];
      sampleRateRef.current = context.sampleRate;
      processor.onaudioprocess = (event) => {
        chunksRef.current.push(new Float32Array(event.inputBuffer.getChannelData(0)));
      };
      source.connect(processor);
      processor.connect(mute);
      mute.connect(context.destination);

      streamRef.current = stream;
      contextRef.current = context;
      sourceRef.current = source;
      processorRef.current = processor;
      muteRef.current = mute;
      setSeconds(0);
      recordingRef.current = true;
      setRecording(true);
      timerRef.current = window.setInterval(() => {
        setSeconds((current) => {
          if (current >= 299) {
            window.setTimeout(() => void stop(), 0);
            return 300;
          }
          return current + 1;
        });
      }, 1000);
    } catch (caught) {
      cleanup();
      setError(caught instanceof Error ? caught.message : "Microphone permission was not granted.");
    }
  }

  async function stop() {
    if (!recordingRef.current || savingRef.current) return;
    recordingRef.current = false;
    setRecording(false);
    if (timerRef.current !== null) {
      window.clearInterval(timerRef.current);
      timerRef.current = null;
    }

    const chunks = chunksRef.current;
    const sampleRate = sampleRateRef.current;
    cleanupAudioGraph();

    try {
      const samples = flatten(chunks);
      if (samples.length === 0) throw new Error("No microphone audio was captured.");
      const wav = encodeWav(samples, sampleRate);
      const name = recordingName();
      if (reviewBeforeSave) {
        pendingBlobRef.current = wav;
        pendingNameRef.current = name;
        setReviewUrl(URL.createObjectURL(wav));
        return;
      }
      await saveBlob(wav, name);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "The recording could not be saved.");
    }
  }

  async function keepReview() {
    const blob = pendingBlobRef.current;
    const name = pendingNameRef.current;
    if (!blob || !name || savingRef.current) return;
    try {
      await saveBlob(blob, name);
      clearReview();
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "The recording could not be saved.");
    }
  }

  async function saveBlob(blob: Blob, name: string) {
    savingRef.current = true;
    setSaving(true);
    setError(null);
    try {
      const file = await uploadWorkspaceFile(blob, name);
      onSaved(file);
      setSeconds(0);
      chunksRef.current = [];
    } finally {
      savingRef.current = false;
      setSaving(false);
    }
  }

  function cancel() {
    recordingRef.current = false;
    setRecording(false);
    setSeconds(0);
    chunksRef.current = [];
    clearReview();
    cleanup();
  }

  function clearReview() {
    setReviewUrl((current) => {
      if (current) URL.revokeObjectURL(current);
      return null;
    });
    pendingBlobRef.current = null;
    pendingNameRef.current = null;
  }

  function cleanupAudioGraph() {
    processorRef.current?.disconnect();
    sourceRef.current?.disconnect();
    muteRef.current?.disconnect();
    processorRef.current = null;
    sourceRef.current = null;
    muteRef.current = null;
    streamRef.current?.getTracks().forEach((track) => track.stop());
    streamRef.current = null;
    const context = contextRef.current;
    contextRef.current = null;
    if (context && context.state !== "closed") void context.close();
  }

  function cleanup() {
    recordingRef.current = false;
    if (timerRef.current !== null) {
      window.clearInterval(timerRef.current);
      timerRef.current = null;
    }
    cleanupAudioGraph();
    const current = reviewUrl;
    if (current) URL.revokeObjectURL(current);
  }

  return (
    <div className={compact ? "tk-recorder is-compact" : "tk-recorder"}>
      <span className={recording ? "tk-recorder__pulse is-recording" : "tk-recorder__pulse"}>
        <Mic size={16} strokeWidth={1.8} />
      </span>
      <div className="tk-recorder__copy">
        <strong>{recording ? "Recording…" : reviewUrl ? "Review this recording" : saving ? "Saving recording…" : label}</strong>
        <span>{recording ? `${formatDuration(seconds)} · maximum 5 minutes` : reviewUrl ? "Play it back, then keep it or record again." : "Uses your microphone and saves a WAV to Files."}</span>
        {reviewUrl ? <audio className="tk-recorder__review" src={reviewUrl} controls preload="metadata" /> : null}
      </div>
      <div className="tk-recorder__actions">
        {recording ? (
          <>
            <button className="is-stop" type="button" onClick={() => void stop()}>
              <Square size={12} fill="currentColor" /> Stop
            </button>
            <button className="is-cancel" type="button" title="Discard recording" onClick={cancel}>
              <X size={14} />
            </button>
          </>
        ) : reviewUrl ? (
          <>
            <button type="button" disabled={saving} onClick={() => void keepReview()}><Check size={13} /> {saving ? "Saving" : "Keep & add"}</button>
            <button className="is-cancel" type="button" disabled={saving} title="Discard and record again" onClick={() => { clearReview(); void start(); }}><RotateCcw size={14} /></button>
          </>
        ) : (
          <button type="button" disabled={saving} onClick={() => void start()}>
            <Mic size={13} /> {saving ? "Saving" : "Record"}
          </button>
        )}
      </div>
      {error ? <span className="tk-recorder__error">{error}</span> : null}
    </div>
  );
}

function flatten(chunks: Float32Array[]): Float32Array {
  const length = chunks.reduce((total, chunk) => total + chunk.length, 0);
  const samples = new Float32Array(length);
  let offset = 0;
  for (const chunk of chunks) {
    samples.set(chunk, offset);
    offset += chunk.length;
  }
  return samples;
}

function encodeWav(samples: Float32Array, sampleRate: number): Blob {
  const buffer = new ArrayBuffer(44 + samples.length * 2);
  const view = new DataView(buffer);
  writeAscii(view, 0, "RIFF");
  view.setUint32(4, 36 + samples.length * 2, true);
  writeAscii(view, 8, "WAVE");
  writeAscii(view, 12, "fmt ");
  view.setUint32(16, 16, true);
  view.setUint16(20, 1, true);
  view.setUint16(22, 1, true);
  view.setUint32(24, sampleRate, true);
  view.setUint32(28, sampleRate * 2, true);
  view.setUint16(32, 2, true);
  view.setUint16(34, 16, true);
  writeAscii(view, 36, "data");
  view.setUint32(40, samples.length * 2, true);

  let offset = 44;
  for (const sample of samples) {
    const clamped = Math.max(-1, Math.min(1, sample));
    view.setInt16(offset, clamped < 0 ? clamped * 0x8000 : clamped * 0x7fff, true);
    offset += 2;
  }
  return new Blob([buffer], { type: "audio/wav" });
}

function writeAscii(view: DataView, offset: number, text: string) {
  for (let index = 0; index < text.length; index += 1) {
    view.setUint8(offset + index, text.charCodeAt(index));
  }
}

function recordingName(): string {
  const now = new Date();
  const stamp = [
    now.getFullYear(),
    String(now.getMonth() + 1).padStart(2, "0"),
    String(now.getDate()).padStart(2, "0"),
    "-",
    String(now.getHours()).padStart(2, "0"),
    String(now.getMinutes()).padStart(2, "0"),
    String(now.getSeconds()).padStart(2, "0")
  ].join("");
  return `recording-${stamp}.wav`;
}

function formatDuration(seconds: number): string {
  const minutes = Math.floor(seconds / 60);
  const remainder = seconds % 60;
  return `${minutes}:${String(remainder).padStart(2, "0")}`;
}
