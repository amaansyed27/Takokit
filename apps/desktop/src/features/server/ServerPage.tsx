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
  "GET /v1/models",
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
        <p>Local Axum daemon status, API endpoints, and logs placeholder.</p>
      </header>

      <div className="stats-grid">
        <div className="stat-tile"><span>Status</span><strong>{status.label}</strong><small>{status.uptime}</small></div>
        <div className="stat-tile"><span>Local URL</span><strong>{status.url}</strong><small>localhost only</small></div>
      </div>

      <Section title="Endpoints">
        <Table columns={["Route", "Action", "State", "Notes", "Copy"]} ariaLabel="Server endpoints">
          {endpoints.map((endpoint) => (
            <TableRow key={endpoint}>
              <code>{endpoint}</code>
              <span>{endpoint.split(" ")[0]}</span>
              <span>{endpoint.includes("speech") ? "mock ready" : "scaffold"}</span>
              <span>{endpoint.includes("clone") || endpoint.includes("train") ? "returns typed not-implemented error" : "local API shape"}</span>
              <Button variant="ghost" type="button" onClick={() => copyValue(endpoint)}>
                {copied === endpoint ? <Check size={14} /> : <Copy size={14} />} Copy
              </Button>
            </TableRow>
          ))}
        </Table>
      </Section>

      <Section title="Logs">
        <pre className="logs-panel">No live logs connected yet. Future daemon logs will stream here from ~/.takokit/logs.</pre>
      </Section>
    </section>
  );
}

