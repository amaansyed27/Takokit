import { ArrowLeft, Check, FlaskConical, Gauge, Waves } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import type { RouteComponentProps } from "../../app/routes";
import { getRvcVoice, type RvcVoiceDetail } from "../../lib/rvcApi";
import { RvcCheckpointsPanel } from "./RvcCheckpointsPanel";
import { RvcSamplesPanel } from "./RvcSamplesPanel";
import { RvcTestPanel } from "./RvcTestPanel";
import { RvcTrainingPanel } from "./RvcTrainingPanel";

type Tab = "data" | "train" | "test";

type Props = Pick<RouteComponentProps, "onNavigate" | "onRefresh"> & {
  voice: string;
  initialSamplePath?: string;
  onBack: () => void;
};

const tabs: { id: Tab; label: string; icon: typeof Waves }[] = [
  { id: "data", label: "1. Voice data", icon: Waves },
  { id: "train", label: "2. Train", icon: Gauge },
  { id: "test", label: "3. Test & use", icon: FlaskConical }
];

export function RvcStudioWorkspace({ voice, initialSamplePath, onBack, onNavigate, onRefresh }: Props) {
  const [detail, setDetail] = useState<RvcVoiceDetail | null>(null);
  const [tab, setTab] = useState<Tab>("data");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const initialTabResolved = useRef(false);

  const refresh = useCallback(async () => {
    setError(null);
    try {
      const next = await getRvcVoice(voice);
      setDetail(next);
      if (!initialTabResolved.current) {
        if (next.managed) setTab("test");
        else if (next.samples.length > 0) setTab("train");
        initialTabResolved.current = true;
      }
      await onRefresh();
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "This voice could not be refreshed.");
    } finally {
      setLoading(false);
    }
  }, [voice, onRefresh]);

  useEffect(() => { void refresh(); }, [refresh]);

  if (!detail) {
    return (
      <section className="tk-rvc-simple-studio">
        <button type="button" className="tk-rvc-back" onClick={onBack}><ArrowLeft size={15} /> Trained voices</button>
        {error ? <div className="tk-inline-error">{error}</div> : <p>{loading ? "Loading voice…" : "Voice is unavailable."}</p>}
      </section>
    );
  }

  const running = detail.active_job?.status === "queued" || detail.active_job?.status === "running";
  const ready = Boolean(detail.managed);
  const nextTab: Tab = ready ? "test" : detail.samples.length === 0 ? "data" : "train";

  return (
    <section className="tk-rvc-simple-studio">
      <button type="button" className="tk-rvc-back" onClick={onBack}><ArrowLeft size={15} /> Trained voices</button>

      <header className="tk-rvc-simple-studio__header">
        <div>
          <span>{detail.project.imported ? "Imported voice" : "Trained voice"}</span>
          <h2>{detail.project.name}</h2>
          <p>{ready ? "Ready to use" : running ? "Training in progress" : friendlyState(detail)}</p>
        </div>
        <div className="tk-rvc-simple-facts">
          <span><strong>{detail.dataset.included_sample_count}</strong> recordings</span>
          <span><strong>{formatDuration(detail.dataset.usable_duration_ms)}</strong> usable audio</span>
          <span className={ready ? "is-ready" : ""}><strong>{ready ? "Ready" : "Not ready"}</strong></span>
        </div>
      </header>

      <nav className="tk-rvc-simple-steps" aria-label="Voice training steps">
        {tabs.map(({ id, label, icon: Icon }) => {
          const done = id === "data" ? detail.dataset.ready_for_preparation || ready : id === "train" ? ready : false;
          return (
            <button key={id} type="button" className={tab === id ? "is-active" : done ? "is-done" : ""} onClick={() => setTab(id)}>
              {done ? <Check size={14} /> : <Icon size={14} />} {label}
            </button>
          );
        })}
      </nav>

      {!ready && !running ? (
        <button type="button" className="tk-rvc-simple-next" onClick={() => setTab(nextTab)}>
          <span>Next</span><strong>{nextTab === "data" ? "Add voice recordings" : "Train this voice"}</strong>
        </button>
      ) : null}

      {error ? <div className="tk-inline-error" role="alert">{error}</div> : null}
      {tab === "data" ? <RvcSamplesPanel detail={detail} initialSamplePath={initialSamplePath} onChanged={refresh} /> : null}
      {tab === "train" ? <RvcTrainingPanel detail={detail} onChanged={refresh} /> : null}
      {tab === "test" ? <RvcTestPanel detail={detail} onNavigate={onNavigate} /> : null}

      {(detail.checkpoints.length > 0 || detail.project.imported) ? (
        <details className="tk-rvc-simple-advanced tk-rvc-simple-model-tools">
          <summary>Advanced model files & sharing</summary>
          <RvcCheckpointsPanel detail={detail} onChanged={refresh} />
        </details>
      ) : null}
    </section>
  );
}

function friendlyState(detail: RvcVoiceDetail): string {
  if (detail.samples.length === 0) return "Add recordings to begin";
  if (!detail.dataset.ready_for_preparation) return "Checking recordings";
  if (["failed", "cancelled"].includes(detail.project.state)) return "Training needs attention";
  return "Ready to train";
}

function formatDuration(milliseconds: number): string {
  if (!milliseconds) return "0:00";
  const seconds = Math.round(milliseconds / 1000);
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
}
