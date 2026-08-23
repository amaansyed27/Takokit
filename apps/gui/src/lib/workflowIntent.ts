const SPEAK_INTENT_KEY = "takokit:speak-intent";

type SpeakIntent = {
  modelId?: string;
  voiceId?: string;
};

export function setSpeakIntent(intent: SpeakIntent): void {
  window.sessionStorage.setItem(SPEAK_INTENT_KEY, JSON.stringify(intent));
}

export function consumeSpeakIntent(): SpeakIntent | null {
  const raw = window.sessionStorage.getItem(SPEAK_INTENT_KEY);
  if (!raw) return null;
  window.sessionStorage.removeItem(SPEAK_INTENT_KEY);
  try {
    return JSON.parse(raw) as SpeakIntent;
  } catch {
    return null;
  }
}
