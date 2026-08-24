import { Check, ChevronRight, CircleAlert, Cpu, Database, Server, Wrench } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import type { PageId } from "../../app/navigation";
import type { RuntimeSnapshot } from "../../lib/types";

type RuntimeControlProps = {
  runtime: RuntimeSnapshot;
  onNavigate: (page: PageId) => void;
};

export function RuntimeControl({ runtime, onNavigate }: RuntimeControlProps) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const online = runtime.server.status === "online";
  const readyModels = runtime.models.filter((model) => model.executable).length;
  const readyRunners = runtime.runners.filter((runner) => runner.install_state === "ready").length;
  const statusLabel = online ? "Local runtime" : "Runtime offline";

  const daemonLabel = useMemo(() => {
    if (!online) return "Not connected";
    return runtime.server.uptime.replace(" · ", " / ");
  }, [online, runtime.server.uptime]);

  useEffect(() => {
    function handlePointerDown(event: PointerEvent) {
      if (!rootRef.current?.contains(event.target as Node)) setOpen(false);
    }

    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") setOpen(false);
    }

    document.addEventListener("pointerdown", handlePointerDown);
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("pointerdown", handlePointerDown);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, []);

  function navigate(page: PageId) {
    setOpen(false);
    onNavigate(page);
  }

  return (
    <div className="tk-runtime-control" ref={rootRef}>
      <button
        className={online ? "tk-runtime-trigger is-online" : "tk-runtime-trigger"}
        type="button"
        aria-expanded={open}
        aria-haspopup="dialog"
        onClick={() => setOpen((value) => !value)}
      >
        <span className={online ? "tk-runtime-trigger__dot is-online" : "tk-runtime-trigger__dot"} aria-hidden="true" />
        <span>{statusLabel}</span>
        <ChevronRight className={open ? "is-open" : ""} size={14} strokeWidth={1.9} aria-hidden="true" />
      </button>

      {open ? (
        <div className="tk-runtime-popover" role="dialog" aria-label="Local runtime status">
          <div className="tk-runtime-popover__header">
            <span className={online ? "tk-runtime-popover__icon is-online" : "tk-runtime-popover__icon"}>
              {online ? <Check size={16} strokeWidth={2.2} /> : <CircleAlert size={16} strokeWidth={2} />}
            </span>
            <div>
              <strong>{online ? "Runtime ready" : "Runtime unavailable"}</strong>
              <span>{online ? "Takokit is running locally on this device." : "The local daemon is not responding."}</span>
            </div>
          </div>

          <div className="tk-runtime-popover__stats">
            <RuntimeStat icon={Cpu} label="Models" value={`${readyModels} ready`} />
            <RuntimeStat icon={Wrench} label="Runners" value={`${readyRunners} ready`} />
            <RuntimeStat icon={Server} label="Daemon" value={daemonLabel} />
          </div>

          <div className="tk-runtime-popover__meta">
            <div>
              <span>Storage</span>
              <code title={runtime.storagePath}>{runtime.storagePath}</code>
            </div>
            <div>
              <span>Build</span>
              <code title={runtime.buildId}>{runtime.buildId.slice(0, 12)}</code>
            </div>
          </div>

          <div className="tk-runtime-popover__actions">
            <button type="button" onClick={() => navigate("diagnostics")}>
              <Server size={15} strokeWidth={1.8} />
              Diagnostics
            </button>
            <button type="button" onClick={() => navigate("models")}>
              <Database size={15} strokeWidth={1.8} />
              Models & runners
            </button>
          </div>
        </div>
      ) : null}
    </div>
  );
}

type RuntimeStatProps = {
  icon: typeof Cpu;
  label: string;
  value: string;
};

function RuntimeStat({ icon: Icon, label, value }: RuntimeStatProps) {
  return (
    <div className="tk-runtime-stat">
      <Icon size={15} strokeWidth={1.8} aria-hidden="true" />
      <span>{label}</span>
      <strong title={value}>{value}</strong>
    </div>
  );
}
