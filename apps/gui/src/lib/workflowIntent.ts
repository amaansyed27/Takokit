const SPEAK_INTENT_KEY = "takokit:speak-intent";
const TRANSCRIBE_INTENT_KEY = "takokit:transcribe-intent";
const VOICE_INTENT_KEY = "takokit:voice-intent";
const CLONE_INTENT_KEY = "takokit:clone-intent";

export type SpeakIntent = {
  modelId?: string;
  voiceId?: string;
  text?: string;
};

export type TranscribeIntent = {
  filePath?: string;
};

export type VoiceIntent = {
  samplePath?: string;
};

export type CloneIntent = {
  sourcePath?: string;
  targetPath?: string;
  mode?: "reference" | "rvc";
};

export function setSpeakIntent(intent: SpeakIntent): void {
  writeIntent(SPEAK_INTENT_KEY, intent);
}

export function consumeSpeakIntent(): SpeakIntent | null {
  return consumeIntent<SpeakIntent>(SPEAK_INTENT_KEY);
}

export function setTranscribeIntent(intent: TranscribeIntent): void {
  writeIntent(TRANSCRIBE_INTENT_KEY, intent);
}

export function consumeTranscribeIntent(): TranscribeIntent | null {
  return consumeIntent<TranscribeIntent>(TRANSCRIBE_INTENT_KEY);
}

export function setVoiceIntent(intent: VoiceIntent): void {
  writeIntent(VOICE_INTENT_KEY, intent);
}

export function consumeVoiceIntent(): VoiceIntent | null {
  return consumeIntent<VoiceIntent>(VOICE_INTENT_KEY);
}

export function setCloneIntent(intent: CloneIntent): void {
  writeIntent(CLONE_INTENT_KEY, intent);
}

export function consumeCloneIntent(): CloneIntent | null {
  return consumeIntent<CloneIntent>(CLONE_INTENT_KEY);
}

function writeIntent(key: string, intent: object): void {
  window.sessionStorage.setItem(key, JSON.stringify(intent));
}

function consumeIntent<T>(key: string): T | null {
  const raw = window.sessionStorage.getItem(key);
  if (!raw) return null;
  window.sessionStorage.removeItem(key);
  try {
    return JSON.parse(raw) as T;
  } catch {
    return null;
  }
}
