import { Check, Copy } from "lucide-react";
import { useEffect, useState } from "react";
import type { RouteComponentProps } from "../../app/routes";
import { Badge } from "../../components/ui/Badge";
import { Button } from "../../components/ui/Button";
import { Section } from "../../components/ui/Section";
import { Table, TableRow } from "../../components/ui/Table";
import { useServerStatus } from "../../hooks/useServerStatus";
import { getDoctor } from "../../lib/api";
import type { DoctorResponse } from "../../lib/types";

const endpoints = [
  "GET /health",
  "GET /v1/status",
  "GET /v1/capabilities",
  "GET /v1/models",
  "GET /v1/models/:id",
  "GET /v1/models/:id/plan",
  "POST /v1/models/pull",
  "DELETE /v1/models/:id",
  "GET /v1/runners",
  "GET /v1/runners/:id",
  "GET /v1/runners/:id/doctor",
  "POST /v1/runners/pull",
  "POST /v1/runners/install",
  "DELETE /v1/runners/:id",
  "GET /v1/library/models",
  "GET /v1/library/runners",
  "GET /v1/doctor",
  "GET /v1/test/launch",
  "POST /v1/audio/speech",
  "POST /v1/audio/transcriptions"
];

export function ServerPage({ runtime }: RouteComponentProps) {
  const [copied, setCopied] = useState<string | null>(null);
  const [doctor, setDoctor] = useState<DoctorResponse | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const status = useServerStatus(runtime);

  useEffect(() => {
    let cancelled = false;
    if (runtime.server.status !== "online") {
      setDoctor(null);
      setNotice("Start takokit serve or takokit gui to run doctor checks through the API.");
      return;
    }

    getDoctor()
      .then((report) => {
        if (cancelled) return;
        setDoctor(report);
        setNotice(null);
      })
      .catch((error) => {
        if (!cancelled) setNotice(error instanceof Error ? error.message : "Doctor endpoint failed.");
      });

    return () => {
      cancelled = true;
    };
  }, [runtime.server.status]);

  function copyValue(value: string) {
    void navigator.clipboard?.writeText(value);
    setCopied(value);
    window.setTimeout(() => setCopied(null), 1400);
  }

  return (
    <section className="page">
      <header className="page__header">
        <h1>Diagnostics</h1>
        <p>Storage, server health, registry parsing, runner records, GUI build output, and log paths.</p>
      </header>

      <div className="stats-grid">
        <div className="stat-tile"><span>Status</span><strong>{status.label}</strong><small>{status.uptime}</small></div>
        <div className="stat-tile"><span>Local URL</span><strong>{status.url}</strong><small>localhost only</small></div>
        <div className="stat-tile"><span>Storage</span><strong>{doctor?.storage_root ?? runtime.storagePath}</strong><small>TAKOKIT_HOME aware</small></div>
        <div className="stat-tile"><span>Executable</span><strong className="stat-tile__value">{doctor?.executable_models.length ?? runtime.models.filter((model) => model.executable).length}</strong><small>Planner summary</small></div>
      </div>

      <Section title="Doctor checks">
        {notice && <p className="notice-line">{notice}</p>}
        <Table columns={["Section", "Check", "Status", "Detail", "Action"]} ariaLabel="Doctor checks">
          {(doctor?.checks ?? []).map((check) => (
            <TableRow key={`${check.section}-${check.label}`}>
              <strong>{check.section}</strong>
              <span>{check.label}</span>
              <Badge tone={check.status === "ok" ? "success" : "warning"}>{check.status}</Badge>
              <span>{check.detail ?? "-"}</span>
              <Button variant="ghost" type="button" onClick={() => copyValue(check.detail ?? check.label)}>
                {copied === (check.detail ?? check.label) ? <Check size={14} /> : <Copy size={14} />} Copy
              </Button>
            </TableRow>
          ))}
        </Table>
        {!doctor && !notice && <p className="notice-line">Doctor report loading.</p>}
      </Section>

      <Section title="Runtime matrix" description="Daemon at a glance.">
        <div className="status-matrix">
          <div className="status-cell">
            <strong>Bind</strong>
            <span>127.0.0.1 only</span>
          </div>
          <div className="status-cell">
            <strong>Speech</strong>
            <span>{runtime.models.find((model) => model.id === "mock-tts")?.executionStatus ?? "internal test path"}</span>
          </div>
          <div className="status-cell">
            <strong>Outputs</strong>
            <span>{runtime.storagePath}/outputs</span>
          </div>
          <div className="status-cell">
            <strong>Logs</strong>
            <span>{doctor?.logs_path ?? "~/.takokit/logs"}</span>
          </div>
        </div>
      </Section>

      <Section title="Endpoints">
        <div className="command-note">
          <code>takokit gui</code>
          <span>starts the daemon when needed and opens the local web GUI</span>
        </div>
        <div className="command-note">
          <code>takokit doctor --json</code>
          <span>returns storage, registry, runner, GUI, and executable-model diagnostics</span>
        </div>
        <Table columns={["Route", "Method", "State", "Notes", "Copy"]} ariaLabel="Server endpoints">
          {endpoints.map((endpoint) => (
            <TableRow key={endpoint}>
              <code>{endpoint}</code>
              <span>{endpoint.split(" ")[0]}</span>
              <span>{endpoint.includes("audio") ? "execution route" : endpoint.includes("doctor") || endpoint.includes("test") ? "diagnostic route" : "package route"}</span>
              <span>local API</span>
              <Button variant="ghost" type="button" onClick={() => copyValue(endpoint)}>
                {copied === endpoint ? <Check size={14} /> : <Copy size={14} />} Copy
              </Button>
            </TableRow>
          ))}
        </Table>
      </Section>

      <Section title="Logs">
        <pre className="logs-panel">{doctor ? `Main logs: ${doctor.logs_path}\nRunner logs live under ${runtime.storagePath}/runners/<runner>/logs.` : "Doctor data unavailable."}</pre>
      </Section>
    </section>
  );
}
