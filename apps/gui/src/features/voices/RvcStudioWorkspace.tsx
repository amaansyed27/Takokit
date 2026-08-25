import { ArrowLeft, CheckCircle2, FlaskConical, Gauge, Music2, Waves } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import type { RouteComponentProps } from "../../app/routes";
import { getRvcVoice, type RvcVoiceDetail } from "../../lib/rvcApi";
import { RvcCheckpointsPanel } from "./RvcCheckpointsPanel";
import { RvcSamplesPanel } from "./RvcSamplesPanel";
import { RvcTestPanel } from "./RvcTestPanel";
import { RvcTrainingPanel } from "./RvcTrainingPanel";

type Tab = "samples" | "training" | "checkpoints" | "test";

type Props = Pick<RouteComponentProps, "onNavigate" | "onRefresh"> & {
  voice: string;
  initialSamplePath?: string;
  onBack: () => void;
};

const tabs: { id: Tab; label: string; icon: typeof Waves }[] = [
  { id: "samples", label: "Samples", icon: Waves },
  { id: "training", label: "Training", icon: Gauge },
  { id: "checkpoints", label: "Checkpoints", icon: CheckCircle2 },
  { id: "test", label: "Test", icon: FlaskConical }
];

export function RvcStudioWorkspace({ voice, initialSamplePath, onBack, onNavigate, onRefresh }: Props) {
  const [detail, setDetail] = useState<RvcVoiceDetail | null>(null);
  const [tab, setTab] = useState<Tab>("samples");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setError(null);
    try {
      setDetail(await getRvcVoice(voice));
      await onRefresh();
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "Voice Studio could not refresh this project.");
    } finally {
      setLoading(false);
    }
  }, [voice, onRefresh]);

  useEffect(() => { void refresh(); }, [refresh]);

  if (!detail) {
    return (
      <section className="tk-rvc-studio">
        <button type="button" className="tk-rvc-back" onClick={onBack}><ArrowLeft size={15} /> Advanced voices</button>
        {error ? <div className="tk-inline-error">{error}</div> : <div className="tk-rvc-loading">{loading ? "Loading voice studio…" : "Voice project is unavailable."}</div>}
      </section>
    );
  }

  const project = detail.project;
  return (
    <section className="tk-rvc-studio">
      <header className="tk-rvc-studio__header">
        <button type="button" className="tk-rvc-back" onClick={onBack}><ArrowLeft size={15} /> Advanced voices</button>
        <div className="tk-rvc-studio__identity">
          <span className="tk-rvc-studio__icon"><Music2 size={19} strokeWidth={1.6} /></span>
          <div><span>RVC Voice Studio</span><h2>{project.name}</h2><p>{project.imported ? "Imported voice" : "Custom training project"} · {stateLabel(project.state)}</p></div>
        </div>
        <div className="tk-rvc-studio__facts">
          <span><strong>{detail.dataset.included_sample_count}</strong> included samples</span>
          <span><strong>{formatDuration(detail.dataset.usable_duration_ms)}</strong> usable audio</span>
          <span><strong>{detail.checkpoints.length}</strong> checkpoints</span>
          <span className={detail.managed ? "is-ready" : ""}><strong>{detail.managed ? "Ready" : "Not ready"}</strong> conversion target</span>
        </div>
      </header>

      <nav className="tk-rvc-tabs" aria-label="Voice Studio sections">
        {tabs.map(({ id, label, icon: Icon }) => (
          <button key={id} type="button" className={tab === id ? "is-active" : ""} onClick={() => setTab(id)}>
            <Icon size={15} /> {label}
          </button>
        ))}
      </nav>

      {error ? <div className="tk-inline-error" role="alert">{error}</div> : null}
      {tab === "samples" ? <RvcSamplesPanel detail={detail} initialSamplePath={initialSamplePath} onChanged={refresh} /> : null}
      {tab === "training" ? <RvcTrainingPanel detail={detail} onChanged={refresh} /> : null}
      {tab === "checkpoints" ? <RvcCheckpointsPanel detail={detail} onChanged={refresh} /> : null}
      {tab === "test" ? <RvcTestPanel detail={detail} onNavigate={onNavigate} /> : null}
    </section>
  );
}

function stateLabel(state: string): string {
  return state.replace(/_/g, " ").replace(/\b\w/g, (letter) => letter.toUpperCase());
}

function formatDuration(milliseconds: number): string {
  if (!milliseconds) return "0:00";
  const seconds = Math.round(milliseconds / 1000);
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
}
