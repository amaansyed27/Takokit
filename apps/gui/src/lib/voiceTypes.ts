export type VoiceProfile = {
  id: string;
  name: string;
  model_id: string;
  sample_path: string;
  created_at: number;
  consent_affirmed: boolean;
  consent_note?: string;
};
