import {
  Activity,
  AudioWaveform,
  Box,
  FileAudio,
  Settings2,
  UserRoundPlus,
  Volume2
} from "lucide-react";
import type { RouteComponentProps } from "../../app/routes";
import { ProductActionCard } from "../../components/ui/ProductActionCard";
import { ProductMetric } from "../../components/ui/ProductMetric";
import { ProductPageHeader } from "../../components/ui/ProductPageHeader";

export function HomePage({ runtime, onNavigate }: RouteComponentProps) {
  const readyModels = runtime.models.filter((model) => model.executable).length;
  const readyRunners = runtime.runners.filter((runner) => runner.install_state === "ready").length;
  const serverOnline = runtime.server.status === "online";
  const ttsModels = runtime.models.filter((model) => model.capabilities.includes("tts")).length;
  const sttModels = runtime.models.filter((model) => model.capabilities.includes("stt")).length;
  const cloningModels = runtime.models.filter((model) => model.capabilities.includes("voice_conversion")).length;

  return (
    <section className="tk-page tk-home-page">
      <ProductPageHeader
        eyebrow="Local workspace"
        title="What do you want to create?"
        description="Run speech, transcription, reusable voice creation, and voice-to-voice cloning locally. Takokit only shows controls supported by the model you choose."
      />

      <section className="tk-section">
        <div className="tk-action-grid tk-home-actions">
          <ProductActionCard
            icon={Volume2}
            title="Speak"
            description="Turn text into speech with a local model or one of your saved voices."
            meta={`${ttsModels} TTS models ready`}
            onClick={() => onNavigate("speak")}
          />
          <ProductActionCard
            icon={FileAudio}
            title="Transcribe"
            description="Turn an audio file into text with a local speech-to-text model."
            meta={`${sttModels} STT models ready`}
            onClick={() => onNavigate("transcribe")}
          />
          <ProductActionCard
            icon={UserRoundPlus}
            title="Create a voice"
            description="Create a reusable consent-backed voice from a clean reference recording."
            meta={`${runtime.voices.length} saved voices`}
            onClick={() => onNavigate("voices")}
          />
          <ProductActionCard
            icon={AudioWaveform}
            title="Clone audio"
            description="Keep the words and timing from an existing recording while cloning a reference or RVC voice onto it."
            meta={`${cloningModels} cloning models ready`}
            onClick={() => onNavigate("convert")}
          />
        </div>
      </section>

      <div className="tk-metrics" aria-label="Local runtime summary">
        <ProductMetric label="Models" value={runtime.models.length} detail={`${readyModels} executable`} />
        <ProductMetric label="Voices" value={runtime.voices.length} detail="Saved profiles" />
        <ProductMetric label="Runners" value={readyRunners} detail={`${runtime.runners.length} registered`} />
        <ProductMetric label="Runtime" value={serverOnline ? "Ready" : "Offline"} detail="Managed locally" />
      </div>

      <section className="tk-section">
        <div className="tk-section-heading">
          <div>
            <h2>Manage your local runtime</h2>
            <p>Everything required for everyday use should be manageable here without opening a terminal.</p>
          </div>
        </div>

        <div className="tk-home-manage-grid">
          <button className="tk-home-manage-card" type="button" onClick={() => onNavigate("models")}>
            <span className="tk-home-manage-card__icon"><Box size={18} strokeWidth={1.8} /></span>
            <span className="tk-home-manage-card__body">
              <strong>Models & runners</strong>
              <small>Discover, pull, repair, inspect, and remove local model runtimes.</small>
            </span>
            <span className="tk-home-manage-card__meta">{runtime.models.length} installed</span>
          </button>

          <button className="tk-home-manage-card" type="button" onClick={() => onNavigate("diagnostics")}>
            <span className="tk-home-manage-card__icon"><Activity size={18} strokeWidth={1.8} /></span>
            <span className="tk-home-manage-card__body">
              <strong>Runtime health</strong>
              <small>Inspect daemon state, runner health, logs, workspace, and recovery information.</small>
            </span>
            <span className="tk-home-manage-card__meta">{serverOnline ? "Healthy" : "Needs attention"}</span>
          </button>

          <button className="tk-home-manage-card" type="button" onClick={() => onNavigate("settings")}>
            <span className="tk-home-manage-card__icon"><Settings2 size={18} strokeWidth={1.8} /></span>
            <span className="tk-home-manage-card__body">
              <strong>Storage & settings</strong>
              <small>See where Takokit stores models, runners, voices, outputs, and workspace data.</small>
            </span>
            <span className="tk-home-manage-card__meta">Local only</span>
          </button>
        </div>
      </section>
    </section>
  );
}
