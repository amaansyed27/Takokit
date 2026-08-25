import { ArrowRight, FolderOpen, FlaskConical } from "lucide-react";
import { useState } from "react";
import type { RouteComponentProps } from "../../app/routes";
import { LocalAudioPlayer } from "../../components/audio/LocalAudioPlayer";
import { ProductButton } from "../../components/ui/ProductButton";
import { pickAudioFile } from "../../lib/nativePicker";
import { testRvcVoice, type RvcVoiceDetail } from "../../lib/rvcApi";
import type { VoiceConversionApiResponse } from "../../lib/types";
import { setCloneIntent } from "../../lib/workflowIntent";

type Props = Pick<RouteComponentProps, "onNavigate"> & {
  detail: RvcVoiceDetail;
};

export function RvcTestPanel({ detail, onNavigate }: Props) {
  const [sourcePath, setSourcePath] = useState("");
  const [result, setResult] = useState<VoiceConversionApiResponse | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const ready = Boolean(detail.managed && detail.conversion_target);

  async function browseSource() {
    setBusy(true);
    setError(null);
    try {
      const selected = await pickAudioFile();
      if (selected) {
        setSourcePath(selected);
        setResult(null);
      }
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "The source audio picker could not be opened.");
    } finally {
      setBusy(false);
    }
  }

  async function runTest() {
    if (!ready || !sourcePath.trim()) return;
    setBusy(true);
    setError(null);
    setResult(null);
    try {
      setResult(await testRvcVoice(detail.project.id, sourcePath.trim()));
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "The voice test failed.");
    } finally {
      setBusy(false);
    }
  }

  function useInConvert() {
    const target = detail.conversion_target ?? detail.managed?.id ?? detail.project.id;
    setCloneIntent({ sourcePath: sourcePath.trim() || undefined, targetPath: target, mode: "rvc" });
    onNavigate("convert");
  }

  return (
    <div className="tk-rvc-panel tk-rvc-test-panel">
      <section className="tk-rvc-test-intro">
        <span><FlaskConical size={18} /></span>
        <div>
          <strong>Test this voice through the normal RVC converter</strong>
          <p>This does not use a separate preview engine. It sends the selected source audio through the same conversion service used by Clone audio.</p>
        </div>
      </section>

      {!ready ? (
        <div className="tk-inline-note">Select a valid checkpoint in Checkpoints before testing or using this voice in Convert.</div>
      ) : null}

      <section className="tk-rvc-test-source">
        <div>
          <span>Source speech audio</span>
          <strong>{sourcePath ? displayFileName(sourcePath) : "Choose an audio recording"}</strong>
          <small>{sourcePath || "The words and timing from this recording are preserved."}</small>
        </div>
        <ProductButton tone="secondary" disabled={busy} onClick={() => void browseSource()}>
          <FolderOpen size={14} /> Browse audio
        </ProductButton>
      </section>

      {sourcePath ? <LocalAudioPlayer path={sourcePath} compact defer label="Source audio" /> : null}
      {error ? <div className="tk-inline-error" role="alert">{error}</div> : null}

      <div className="tk-rvc-test-actions">
        <ProductButton tone="primary" loading={busy} disabled={!ready || !sourcePath.trim() || busy} onClick={() => void runTest()}>
          Test voice
        </ProductButton>
        <ProductButton tone="secondary" disabled={!ready} onClick={useInConvert}>
          Use in Convert <ArrowRight size={14} />
        </ProductButton>
      </div>

      {result ? (
        <section className="tk-rvc-test-result">
          <header><strong>Converted test output</strong><span>{result.execution_status === "passed" ? "Execution passed" : result.execution_status}</span></header>
          <LocalAudioPlayer path={result.output_path} label="RVC test output" />
          <p>{result.quality_notice}</p>
          <ProductButton tone="primary" onClick={useInConvert}>Open in Convert <ArrowRight size={14} /></ProductButton>
        </section>
      ) : null}
    </div>
  );
}

function displayFileName(path: string): string {
  const normalized = path.trim().replace(/[\\/]+$/, "");
  const parts = normalized.split(/[\\/]/).filter(Boolean);
  return parts.length > 0 ? parts[parts.length - 1] : "Selected audio";
}
