import { ArrowLeft, ArrowRight, Check, CheckCircle2, FlaskConical, Gauge, Music2, Waves } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import type { RouteComponentProps } from "../../app/routes";
import { ProductButton } from "../../components/ui/ProductButton";
import { getRvcVoice, type RvcVoiceDetail, type RvcVoiceState } from "../../lib/rvcApi";
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
  const guide = nextAction(detail);
  const stages = workflowStages(detail);

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

      <section className="tk-rvc-workflow-guide" aria-label="RVC workflow progress">
        <div className="tk-rvc-workflow-stages">
          {stages.map((stage, index) => (
            <button key={stage.label} type="button" className={stage.done ? "is-done" : guide.tab === stage.tab ? "is-current" : ""} onClick={() => setTab(stage.tab)}>
              <span>{stage.done ? <Check size={13} /> : index + 1}</span>
              <strong>{stage.label}</strong>
            </button>
          ))}
        </div>
        <div className="tk-rvc-next-action">
          <div>
            <span>Recommended next</span>
            <strong>{guide.title}</strong>
            <p>{guide.description}</p>
          </div>
          <ProductButton tone="primary" onClick={() => setTab(guide.tab)}>
            {guide.button} <ArrowRight size={14} />
          </ProductButton>
        </div>
      </section>

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

function workflowStages(detail: RvcVoiceDetail): { label: string; tab: Tab; done: boolean }[] {
  const prepared = isPreparedOrLater(detail.project.state) || detail.checkpoints.length > 0;
  return [
    { label: "Dataset", tab: "samples", done: detail.project.imported || detail.dataset.ready_for_preparation || prepared },
    { label: "Prepare", tab: "training", done: detail.project.imported || prepared },
    { label: "Train", tab: "training", done: detail.project.imported || detail.checkpoints.length > 0 },
    { label: "Activate & test", tab: detail.managed ? "test" : "checkpoints", done: Boolean(detail.managed) }
  ];
}

function nextAction(detail: RvcVoiceDetail): { tab: Tab; title: string; description: string; button: string } {
  const { project, dataset, samples, checkpoints, managed, active_job: job } = detail;
  if (managed) return { tab: "test", title: "Test the ready voice", description: "Your conversion target is active. Run a real voice conversion test, then use it from Clone audio.", button: "Open Test" };
  if (project.imported && checkpoints.length > 0) return { tab: "checkpoints", title: "Activate the imported checkpoint", description: "Choose the checkpoint and optional index that Takokit should use as this voice's managed conversion target.", button: "Choose checkpoint" };
  if (samples.length === 0) return { tab: "samples", title: "Add training recordings", description: "Add one or more clean recordings from the same speaker, then inspect the dataset before preparation.", button: "Open Samples" };
  if (!dataset.ready_for_preparation) return { tab: "samples", title: "Inspect the dataset", description: "Recordings are present, but Takokit has not validated their duration and audio properties yet. Click Inspect dataset in Samples.", button: "Inspect samples" };
  if (job && (job.status === "queued" || job.status === "running")) return { tab: "training", title: job.stage.includes("train") ? "Training is running" : "Preparation is running", description: "Keep this page open to watch logs, or leave and return later; the managed job continues independently.", button: "View job" };
  if (project.state === "failed" || project.state === "cancelled" || job?.status === "failed" || job?.status === "cancelled" || job?.status === "stale") return { tab: "training", title: "Recover the interrupted job", description: "The last preparation or training job stopped. Review the failure/logs and use Recover when appropriate.", button: "Open Training" };
  if (project.state === "ready_to_train") return { tab: "training", title: "Run hardware preflight, then train", description: "Preparation is complete. Run preflight to verify CUDA, VRAM, RAM and disk, then start the managed training job.", button: "Continue training" };
  if (checkpoints.length > 0) return { tab: "checkpoints", title: "Choose the checkpoint to use", description: "Training produced validated artifacts. Activate a checkpoint and optional index before testing the voice.", button: "Open Checkpoints" };
  return { tab: "training", title: "Prepare the dataset", description: "Your inspected samples are ready. Choose a preset and run Prepare to preprocess audio and extract RMVPE/HubERT features.", button: "Open Training" };
}

function isPreparedOrLater(state: RvcVoiceState): boolean {
  return ["ready_to_train", "training", "building_index", "validating_artifacts", "ready"].includes(state);
}

function stateLabel(state: string): string {
  return state.replace(/_/g, " ").replace(/\b\w/g, (letter) => letter.toUpperCase());
}

function formatDuration(milliseconds: number): string {
  if (!milliseconds) return "0:00";
  const seconds = Math.round(milliseconds / 1000);
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
}
