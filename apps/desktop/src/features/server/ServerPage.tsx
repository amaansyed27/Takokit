import { SectionHeader } from "../../components/SectionHeader";
import type { RuntimeSnapshot } from "../../lib/types";

type ServerPageProps = {
  runtime: RuntimeSnapshot;
};

export function ServerPage({ runtime }: ServerPageProps) {
  return (
    <section className="page-flow">
      <SectionHeader title="Server" description="Local Axum daemon status and API details." />
      <div className="settings-list">
        <div><span>Status</span><strong>{runtime.server.status}</strong></div>
        <div><span>Bind address</span><code>{runtime.server.url}</code></div>
        <div><span>Health route</span><code>GET /health</code></div>
        <div><span>Speech route</span><code>POST /v1/audio/speech</code></div>
      </div>
    </section>
  );
}

