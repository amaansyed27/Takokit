import { ArrowRight, FolderOpen } from "lucide-react";
import { useState } from "react";
import type { RouteComponentProps } from "../../app/routes";
import { LocalAudioPlayer } from "../../components/audio/LocalAudioPlayer";
import { ProductButton } from "../../components/ui/ProductButton";
import { pickAudioFile } from "../../lib/nativePicker";
import { testRvcVoice, type RvcVoiceDetail } from "../../lib/rvcApi";
import type { VoiceConversionApiResponse } from "../../lib/types";
import { setCloneIntent } from "../../lib/workflowIntent";

type Props = Pick<RouteComponentProps, "onNavigate"> & { detail: RvcVoiceDetail };

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
      setError(caught instanceof Error ? caught.message : "The audio picker could not be opened.");
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
    setCloneIntent({ sourcePath: sourcePath.trim() || undefined, targetPath: detail.project.id, mode: "rvc" });
    onNavigate("convert");
  }

  return (
    <div className="tk-rvc-simple-panel tk-rvc-simple-test">
      <header className="tk-rvc-simple-section-heading">
        <div><h3>Test your voice</h3><p>Choose any speech recording. Takokit preserves the words and timing while converting the speaker to this trained voice.</p></div>
      </header>

      {!ready ? <div className="tk-inline-note">Finish training or import a voice first. Takokit activates the finished model automatically.</div> : null}

      <div className="tk-rvc-simple-test-source">
        <div><span>Source audio</span><strong>{sourcePath ? displayFileName(sourcePath) : "Choose speech to convert"}</strong><small>{sourcePath || "Use a different speaker for the clearest test."}</small></div>
        <ProductButton tone="secondary" disabled={busy} onClick={() => void browseSource()}><FolderOpen size={14} /> Browse audio</ProductButton>
      </div>

      {sourcePath ? <LocalAudioPlayer path={sourcePath} compact defer label="Source audio" /> : null}
      {error ? <div className="tk-inline-error" role="alert">{error}</div> : null}

      <div className="tk-rvc-simple-actions">
        <ProductButton tone="primary" loading={busy} disabled={!ready || !sourcePath.trim() || busy} onClick={() => void runTest()}>Test voice</ProductButton>
        <ProductButton tone="secondary" disabled={!ready} onClick={useInConvert}>Use in Clone audio <ArrowRight size={14} /></ProductButton>
      </div>

      {result ? (
        <section className="tk-rvc-simple-test-result">
          <header><strong>Converted voice</strong><span>{result.execution_status === "passed" ? "Ready" : result.execution_status}</span></header>
          <LocalAudioPlayer path={result.output_path} label="Converted test output" />
          <ProductButton tone="primary" onClick={useInConvert}>Continue in Clone audio <ArrowRight size={14} /></ProductButton>
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
