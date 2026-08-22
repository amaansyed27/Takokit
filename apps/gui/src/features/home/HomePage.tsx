import {
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

  return (
    <section className="tk-page">
      <div className="tk-home-hero">
        <ProductPageHeader
          eyebrow="Local-first audio AI"
          title="Create with your local voice models."
          description="Generate speech, transcribe audio, clone voices, and convert recordings without leaving your machine."
        />
        <div className="tk-home-hero__status">
          <span>Runtime</span>
          <strong>{serverOnline ? "Ready on this device" : "Unavailable"}</strong>
          <small>{runtime.server.url}</small>
        </div>
      </div>

      <div className="tk-metrics" aria-label="Runtime summary">
        <ProductMetric label="Models" value={runtime.models.length} detail={`${readyModels} ready to run`} />
        <ProductMetric label="Voices" value={runtime.voices.length} detail="Saved profiles" />
        <ProductMetric label="Runners" value={readyRunners} detail={`${runtime.runners.length} available`} />
        <ProductMetric label="Mode" value="Local" detail={serverOnline ? "Daemon connected" : "Daemon offline"} />
      </div>

      <section className="tk-section">
        <div className="tk-section-heading">
          <div>
            <h2>Create</h2>
            <p>Choose what you want to make. Takokit will only show settings the selected model actually supports.</p>
          </div>
        </div>
        <div className="tk-action-grid">
          <ProductActionCard
            icon={Volume2}
            title="Speak"
            description="Turn text into speech using a local TTS model or one of your saved voices."
            meta={`${runtime.models.filter((model) => model.capabilities.includes("tts")).length} installed TTS models`}
            onClick={() => onNavigate("speak")}
          />
          <ProductActionCard
            icon={FileAudio}
            title="Transcribe"
            description="Turn an audio file into text with an installed speech-to-text model."
            meta={`${runtime.models.filter((model) => model.capabilities.includes("stt")).length} installed STT models`}
            onClick={() => onNavigate("transcribe")}
          />
          <ProductActionCard
            icon={UserRoundPlus}
            title="Create a voice"
            description="Create a reusable consent-backed voice profile from reference audio."
            meta={`${runtime.voices.length} saved voices`}
            onClick={() => onNavigate("voices")}
          />
          <ProductActionCard
            icon={AudioWaveform}
            title="Convert voice"
            description="Keep the spoken words while changing the voice toward a reference or RVC target."
            meta={`${runtime.models.filter((model) => model.capabilities.includes("voice_conversion")).length} conversion models`}
            onClick={() => onNavigate("convert")}
          />
        </div>
      </section>

      <section className="tk-section">
        <div className="tk-section-heading">
          <div>
            <h2>Your local library</h2>
            <p>Models, runners, and voice profiles stay under Takokit-managed local storage.</p>
          </div>
        </div>
        <div className="tk-home-library">
          <div className="tk-home-library__primary">
            <div className="tk-home-library__copy">
              <Box size={22} strokeWidth={1.7} aria-hidden="true" />
              <strong>Models & runtimes</strong>
              <p>Browse the model library, pull new models, repair broken installs, remove models safely, and inspect the runner each model uses.</p>
            </div>
            <button className="tk-text-button" type="button" onClick={() => onNavigate("models")}>Manage models →</button>
          </div>
          <div className="tk-home-library__secondary">
            <div className="tk-home-library__copy">
              <Settings2 size={20} strokeWidth={1.7} aria-hidden="true" />
              <strong>Runtime health</strong>
            </div>
            <dl>
              <div><dt>Installed</dt><dd>{runtime.models.length}</dd></div>
              <div><dt>Ready</dt><dd>{readyModels}</dd></div>
              <div><dt>Runner runtimes</dt><dd>{readyRunners}</dd></div>
              <div><dt>Server</dt><dd>{serverOnline ? "Online" : "Offline"}</dd></div>
            </dl>
            <button className="tk-text-button" type="button" onClick={() => onNavigate("diagnostics")}>Open diagnostics →</button>
          </div>
        </div>
      </section>
    </section>
  );
}
