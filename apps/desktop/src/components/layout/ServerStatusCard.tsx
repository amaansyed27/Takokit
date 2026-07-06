import { Circle, Info } from "lucide-react";
import { Tooltip } from "../ui/Tooltip";
import type { RuntimeSnapshot } from "../../lib/types";

type ServerStatusCardProps = {
  runtime: RuntimeSnapshot;
};

export function ServerStatusCard({ runtime }: ServerStatusCardProps) {
  const isOnline = runtime.server.status === "online";

  return (
    <section className="server-status-card" aria-label="Server status">
      <div className="server-status-card__header">
        <span>Server</span>
        <Tooltip content="Local API status. No remote calls are made from this panel.">
          <button className="icon-button subtle" type="button" aria-label="Server status help">
            <Info size={14} />
          </button>
        </Tooltip>
      </div>
      <div className="server-status-card__state">
        <Circle className={isOnline ? "status-pulse" : ""} size={10} fill="currentColor" />
        <strong>{isOnline ? "Running" : "Mock mode"}</strong>
      </div>
      <code>{runtime.server.url}</code>
      <span>Uptime {runtime.server.uptime}</span>
    </section>
  );
}

