import { Check, CircleAlert, Copy, FileText, RefreshCw, Server, Wrench } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import type { RouteComponentProps } from "../../app/routes";
import { ProductButton } from "../../components/ui/ProductButton";
import { ProductPageHeader } from "../../components/ui/ProductPageHeader";
import { getDoctor } from "../../lib/api";
import { openStorageLocation } from "../../lib/storage";
import type { DoctorCheck, DoctorResponse } from "../../lib/types";

export function ServerPage({ runtime, onRefresh, onNavigate }: RouteComponentProps) {
  const [doctor, setDoctor] = useState<DoctorResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [copied, setCopied] = useState<string | null>(null);

  useEffect(() => { void refreshDoctor(); }, [runtime.server.status, runtime.buildId]);

  const counts = useMemo(() => {
    const checks = doctor?.checks ?? [];
    return {
      ok: checks.filter((check) => check.status === "ok").length,
      warn: checks.filter((check) => check.status === "warn").length,
      fail: checks.filter((check) => check.status === "fail").length
    };
  }, [doctor]);

  const groups = useMemo(() => {
    const map = new Map<string, DoctorCheck[]>();
    for (const check of doctor?.checks ?? []) {
      const current = map.get(check.section) ?? [];
      current.push(check);
      map.set(check.section, current);
    }
    return [...map.entries()];
  }, [doctor]);

  async function refreshDoctor() {
    if (runtime.server.status !== "online") {
      setDoctor(null);
      return;
    }
    setLoading(true);
    setNotice(null);
    try {
      setDoctor(await getDoctor());
    } catch (error) {
      setNotice(error instanceof Error ? error.message : "Diagnostics could not be loaded.");
    } finally {
      setLoading(false);
    }
  }

  async function retryRuntime() {
    await onRefresh();
    await refreshDoctor();
  }

  function copy(value: string) {
    void navigator.clipboard.writeText(value);
    setCopied(value);
    window.setTimeout(() => setCopied(null), 1200);
  }

  return (
    <section className="tk-page tk-diagnostics-page">
      <ProductPageHeader
        eyebrow="System health"
        title="Diagnostics"
        description="A readable view of daemon identity, storage, registry health, installed records, runners, and logs."
        actions={<ProductButton tone="secondary" loading={loading} onClick={() => void retryRuntime()}><RefreshCw size={15} /> Run checks</ProductButton>}
      />

      <div className="tk-diagnostics-summary">
        <Metric label="Runtime" value={runtime.server.status === "online" ? "Online" : "Offline"} detail={runtime.server.uptime} />
        <Metric label="Healthy checks" value={String(counts.ok)} detail={`${counts.warn} warnings · ${counts.fail} failures`} />
        <Metric label="Executable models" value={String(doctor?.executable_models.length ?? runtime.models.filter((model) => model.executable).length)} detail={`${runtime.models.length} installed`} />
        <Metric label="Build" value={runtime.buildId.slice(0, 8)} detail={runtime.buildId} />
      </div>

      {runtime.server.status !== "online" ? (
        <div className="tk-diagnostics-offline">
          <CircleAlert size={22} />
          <div><strong>Local runtime is unavailable</strong><span>Start or restart Takokit, then run the checks again.</span></div>
          <ProductButton tone="secondary" onClick={() => void onRefresh()}>Retry runtime</ProductButton>
        </div>
      ) : null}
      {notice ? <div className="tk-inline-error" role="alert">{notice}</div> : null}

      {doctor ? (
        <div className="tk-diagnostics-layout">
          <section className="tk-diagnostics-checks">
            {groups.map(([section, checks]) => (
              <div className="tk-doctor-group" key={section}>
                <div className="tk-doctor-group__header"><span>{section}</span><strong>{checks.filter((check) => check.status === "ok").length}/{checks.length} healthy</strong></div>
                {checks.map((check) => (
                  <div className="tk-doctor-check" key={`${section}-${check.label}`}>
                    <span className={check.status === "ok" ? "tk-doctor-check__icon is-ok" : check.status === "fail" ? "tk-doctor-check__icon is-fail" : "tk-doctor-check__icon is-warn"}>
                      {check.status === "ok" ? <Check size={13} /> : <CircleAlert size={13} />}
                    </span>
                    <div><strong>{check.label}</strong><span title={check.detail}>{check.detail ?? "No additional detail"}</span></div>
                    <button className="tk-row-icon-action" type="button" title="Copy detail" onClick={() => copy(check.detail ?? check.label)}>{copied === (check.detail ?? check.label) ? <Check size={14} /> : <Copy size={14} />}</button>
                  </div>
                ))}
              </div>
            ))}
          </section>

          <aside className="tk-diagnostics-side">
            <div className="tk-diagnostics-card">
              <span className="tk-diagnostics-card__icon"><Server size={18} /></span>
              <div><strong>Local daemon</strong><span>{runtime.server.url}</span></div>
              <dl><div><dt>Mode</dt><dd>{runtime.server.uptime}</dd></div><div><dt>Build</dt><dd title={runtime.buildId}>{runtime.buildId.slice(0, 12)}</dd></div><div><dt>Storage</dt><dd title={doctor.storage_root}>{doctor.storage_root}</dd></div></dl>
            </div>
            <div className="tk-diagnostics-card">
              <span className="tk-diagnostics-card__icon"><FileText size={18} /></span>
              <div><strong>Logs</strong><span>Runtime and runner diagnostics are written locally.</span></div>
              <code title={doctor.logs_path}>{doctor.logs_path}</code>
              <ProductButton tone="secondary" onClick={() => void openStorageLocation("logs")}>Open logs folder</ProductButton>
            </div>
            <div className="tk-diagnostics-card">
              <span className="tk-diagnostics-card__icon"><Wrench size={18} /></span>
              <div><strong>Something needs repair?</strong><span>Use Models for model lifecycle and Runners for shared runtime repair.</span></div>
              <div className="tk-diagnostics-card__actions"><button type="button" onClick={() => onNavigate("models")}>Models →</button><button type="button" onClick={() => onNavigate("runners")}>Runners →</button></div>
            </div>
          </aside>
        </div>
      ) : loading ? <div className="tk-system-loading">Running local doctor checks…</div> : null}
    </section>
  );
}

function Metric({ label, value, detail }: { label: string; value: string; detail: string }) {
  return <div className="tk-system-metric"><span>{label}</span><strong>{value}</strong><small title={detail}>{detail}</small></div>;
}
