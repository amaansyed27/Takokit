import { Check, Copy } from "lucide-react";
import { useState } from "react";
import type { RouteComponentProps } from "../../app/routes";
import { Button } from "../../components/ui/Button";
import { Section } from "../../components/ui/Section";
import { Table, TableRow } from "../../components/ui/Table";
import { useServerStatus } from "../../hooks/useServerStatus";

const endpoints = [
  "GET /health",
  "GET /v1/status",
  "GET /v1/capabilities",
  "GET /v1/models",
  "GET /v1/models/:id",
  "POST /v1/models/pull",
  "DELETE /v1/models/:id",
  "GET /v1/runners",
  "GET /v1/runners/:id",
  "POST /v1/runners/pull",
  "DELETE /v1/runners/:id",
  "GET /v1/voices",
  "POST /v1/audio/speech",
  "POST /v1/audio/transcriptions",
  "POST /v1/voices/clone",
  "POST /v1/voices/train"
];

export function ServerPage({ runtime }: RouteComponentProps) {
  const [copied, setCopied] = useState<string | null>(null);
  const status = useServerStatus(runtime);

  function copyValue(value: string) {
    void navigator.clipboard?.writeText(value);
    setCopied(value);
    window.setTimeout(() => setCopied(null), 1400);
  }

  return (
    <section className="page">
      <header className="page__header">
        <h1>Server</h1>
        <p>Local daemon, browser GUI, and API routes.</p>
      </header>

      <div className="stats-grid">
        <div className="stat-tile"><span>Status</span><strong>{status.label}</strong><small>{status.uptime}</small></div>
        <div className="stat-tile"><span>Local URL</span><strong>{status.url}</strong><small>localhost only</small></div>
      </div>

      <Section title="Runtime matrix" description="Daemon at a glance.">
        <div className="status-matrix">
          <div className="status-cell">
            <strong>Bind</strong>
            <span>127.0.0.1 only</span>
          </div>
          <div className="status-cell">
            <strong>Speech</strong>
            <span>Mock adapter ready</span>
          </div>
          <div className="status-cell">
            <strong>Storage</strong>
            <span>{runtime.storagePath}/outputs</span>
          </div>
          <div className="status-cell">
            <strong>Logs</strong>
            <span>~/.takokit/logs</span>
          </div>
        </div>
      </Section>

      <Section title="Endpoints">
        <div className="command-note">
          <code>takokit gui</code>
          <span>starts the daemon when needed and opens the local web GUI</span>
        </div>
        <div className="command-note">
          <code>takokit</code>
          <span>opens the interactive terminal launcher for mock speech and metadata pulls</span>
        </div>
        <div className="command-note">
          <code>takokit doctor</code>
          <span>checks storage, registry manifests, runner records, server status, and GUI build output</span>
        </div>
        <Table columns={["Route", "Action", "State", "Notes", "Copy"]} ariaLabel="Server endpoints">
          {endpoints.map((endpoint) => (
            <TableRow key={endpoint}>
              <code>{endpoint}</code>
              <span>{endpoint.split(" ")[0]}</span>
              <span>{endpoint.includes("speech") ? "mock ready" : endpoint.includes("models") || endpoint.includes("runners") ? "package route" : "scaffold"}</span>
              <span>{endpoint.includes("clone") || endpoint.includes("train") ? "typed placeholder" : "local route"}</span>
              <Button variant="ghost" type="button" onClick={() => copyValue(endpoint)}>
                {copied === endpoint ? <Check size={14} /> : <Copy size={14} />} Copy
              </Button>
            </TableRow>
          ))}
        </Table>
      </Section>

      <Section title="Logs">
        <pre className="logs-panel">No live logs yet. Future daemon output streams from ~/.takokit/logs.</pre>
      </Section>
    </section>
  );
}
