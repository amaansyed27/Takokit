import { FolderOpen, HardDrive, RefreshCw } from "lucide-react";
import { useEffect, useState } from "react";
import type { RouteComponentProps } from "../../app/routes";
import { ProductButton } from "../../components/ui/ProductButton";
import { ProductPageHeader } from "../../components/ui/ProductPageHeader";
import { getStorageOverview, openStorageLocation, type OpenStorageTarget } from "../../lib/storage";
import type { StorageOverview } from "../../lib/types";

export function StoragePage({ runtime, onNavigate }: RouteComponentProps) {
  const [overview, setOverview] = useState<StorageOverview | null>(null);
  const [loading, setLoading] = useState(true);
  const [busyTarget, setBusyTarget] = useState<OpenStorageTarget | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void refresh();
  }, [runtime.workspacePath, runtime.server.status]);

  async function refresh() {
    if (runtime.server.status !== "online") {
      setOverview(null);
      setLoading(false);
      return;
    }
    setLoading(true);
    setError(null);
    try {
      setOverview(await getStorageOverview());
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "Storage usage could not be measured.");
    } finally {
      setLoading(false);
    }
  }

  async function open(target: OpenStorageTarget) {
    setBusyTarget(target);
    setError(null);
    try {
      await openStorageLocation(target);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "The location could not be opened.");
    } finally {
      setBusyTarget(null);
    }
  }

  const total = Math.max(overview?.total_bytes ?? 0, 1);

  return (
    <section className="tk-page tk-storage-page">
      <ProductPageHeader
        eyebrow="Local data"
        title="Storage"
        description="See exactly where Takokit keeps models, runtimes, voices, cache, logs, and workspace outputs. Nothing here is cloud-backed."
        actions={
          <ProductButton tone="secondary" type="button" loading={loading} onClick={() => void refresh()}>
            <RefreshCw size={15} strokeWidth={1.8} /> Refresh usage
          </ProductButton>
        }
      />

      <div className="tk-storage-summary">
        <Metric label="Takokit storage" value={formatBytes(overview?.total_bytes ?? 0)} detail={overview?.storage_root ?? runtime.storagePath} />
        <Metric label="Workspace data" value={formatBytes(overview?.workspace_bytes ?? 0)} detail="Active .tako sessions and outputs" />
        <Metric label="Models ready" value={String(runtime.models.filter((item) => item.executable).length)} detail={`${runtime.models.length} installed`} />
        <Metric label="Saved voices" value={String(runtime.voices.filter((voice) => voice.source === "local-profile").length)} detail="Reusable local profiles" />
      </div>

      {runtime.server.status !== "online" ? (
        <div className="tk-system-empty">
          <HardDrive size={22} strokeWidth={1.7} />
          <div><strong>Runtime unavailable</strong><span>Start the local runtime to measure storage usage.</span></div>
        </div>
      ) : null}

      {error ? <div className="tk-inline-error" role="alert">{error}</div> : null}

      {overview ? (
        <div className="tk-storage-layout">
          <section className="tk-system-panel">
            <div className="tk-system-panel__header">
              <div><h2>Usage</h2><p>Measured from the current Takokit storage root.</p></div>
            </div>
            <div className="tk-storage-list">
              {overview.entries.map((entry) => (
                <div className="tk-storage-row" key={entry.id}>
                  <div className="tk-storage-row__copy">
                    <strong>{entry.label}</strong>
                    <code title={entry.path}>{entry.path}</code>
                  </div>
                  <div className="tk-storage-row__amount">
                    <strong>{formatBytes(entry.bytes)}</strong>
                    <div className="tk-storage-meter" aria-hidden="true">
                      <span style={{ width: `${Math.max(entry.bytes > 0 ? 2 : 0, Math.min(100, (entry.bytes / total) * 100))}%` }} />
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </section>

          <aside className="tk-storage-actions">
            <div>
              <span className="tk-storage-actions__icon"><FolderOpen size={18} strokeWidth={1.8} /></span>
              <div><strong>Open local data</strong><span>Use your system file manager when you need the actual files.</span></div>
            </div>
            <ProductButton tone="secondary" loading={busyTarget === "storage"} onClick={() => void open("storage")}>Open Takokit storage</ProductButton>
            <ProductButton tone="secondary" loading={busyTarget === "workspace"} onClick={() => void open("workspace")}>Open workspace</ProductButton>
            <ProductButton tone="secondary" loading={busyTarget === "voices"} onClick={() => void open("voices")}>Open voices</ProductButton>
            <ProductButton tone="secondary" loading={busyTarget === "logs"} onClick={() => void open("logs")}>Open logs</ProductButton>
            <div className="tk-storage-actions__note">
              <strong>Need to reclaim space?</strong>
              <span>Remove models from Models or runtimes from Runners so shared dependencies can be retained safely.</span>
              <button type="button" onClick={() => onNavigate("models")}>Manage models →</button>
            </div>
          </aside>
        </div>
      ) : loading ? <div className="tk-system-loading">Measuring local storage…</div> : null}
    </section>
  );
}

function Metric({ label, value, detail }: { label: string; value: string; detail: string }) {
  return <div className="tk-system-metric"><span>{label}</span><strong>{value}</strong><small title={detail}>{detail}</small></div>;
}

function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let index = 0;
  while (value >= 1024 && index < units.length - 1) {
    value /= 1024;
    index += 1;
  }
  return `${value.toFixed(index === 0 ? 0 : value >= 10 ? 1 : 2)} ${units[index]}`;
}
