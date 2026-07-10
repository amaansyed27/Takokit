import { useState } from "react";
import type { RouteComponentProps } from "../../app/routes";
import { Badge } from "../../components/ui/Badge";
import { Button } from "../../components/ui/Button";
import { Section } from "../../components/ui/Section";
import { Table, TableRow } from "../../components/ui/Table";
import { getRunnerDoctor, installAdapter, installRunner, pullRunner, removeRunner } from "../../lib/api";

export function RunnersPage({ runtime, onRefresh }: RouteComponentProps) {
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [doctor, setDoctor] = useState<Record<string, unknown> | null>(null);
  const apiUnavailable = runtime.server.status !== "online";
  const adapterRecords = Array.isArray(doctor?.adapters) ? doctor.adapters.filter((item): item is Record<string, unknown> => Boolean(item) && typeof item === "object") : [];

  async function runAction(label: string, action: () => Promise<void>) {
    setBusyAction(label);
    setNotice(null);
    try {
      await action();
      await onRefresh();
      setNotice("Runner state updated.");
    } catch (error) {
      setNotice(error instanceof Error ? error.message : "Takokit API action failed.");
    } finally {
      setBusyAction(null);
    }
  }

  async function runDoctor(id: string) {
    setBusyAction(`doctor-${id}`);
    setNotice(null);
    try {
      setDoctor(await getRunnerDoctor(id));
    } catch (error) {
      setNotice(error instanceof Error ? error.message : "Runner doctor failed.");
    } finally {
      setBusyAction(null);
    }
  }

  return (
    <section className="page">
      <header className="page__header">
        <h1>Runners</h1>
        <p>Shared runtime families, contract install state, runtime readiness, and dependency strategy.</p>
      </header>

      <div className="stats-grid">
        <div className="stat-tile"><span>Runner families</span><strong className="stat-tile__value">{runtime.runners.length}</strong><small>Shared runtimes</small></div>
        <div className="stat-tile"><span>Contracts</span><strong className="stat-tile__value">{runtime.runners.filter((runner) => runner.installed).length}</strong><small>Installed locally</small></div>
        <div className="stat-tile"><span>Ready</span><strong className="stat-tile__value">{runtime.runners.filter((runner) => runner.install_state === "ready").length}</strong><small>Executable runtime</small></div>
        <div className="stat-tile"><span>Storage</span><strong>~/.takokit</strong><small>Runner-managed deps</small></div>
      </div>

      <Section title="Runner registry" description="Pull installs the contract. Install initializes or verifies the runtime.">
        <Table columns={["Runner", "Families", "Tasks", "State", "Actions"]} ariaLabel="Runtime runners">
          {runtime.runners.map((runner) => (
            <TableRow key={runner.id}>
              <div>
                <strong>{runner.name}</strong>
                <span className="table-note">{runner.id}</span>
              </div>
              <span>{runner.supported_model_families?.join(", ") || "-"}</span>
              <span>{runner.supported_tasks?.join(", ") || "-"}</span>
              <span className="badge-list">
                <Badge tone={runner.installed ? "success" : "warning"}>{runner.installed ? "contract installed" : "contract missing"}</Badge>
                <Badge tone={runner.install_state === "ready" ? "success" : runner.install_state === "failed" ? "warning" : "neutral"}>
                  {runner.install_state ?? "runtime-missing"}
                </Badge>
              </span>
              <span className="action-cluster">
                <Button
                  type="button"
                  variant="ghost"
                  disabled={apiUnavailable || runner.installed}
                  loading={busyAction === `pull-${runner.id}`}
                  onClick={() => runAction(`pull-${runner.id}`, () => pullRunner(runner.id).then(() => undefined))}
                >
                  Pull
                </Button>
                <Button
                  type="button"
                  variant="ghost"
                  disabled={apiUnavailable || !runner.installed || runner.install_state === "ready"}
                  loading={busyAction === `install-${runner.id}`}
                  onClick={() => runAction(`install-${runner.id}`, () => installRunner(runner.id).then(() => undefined))}
                >
                  Install
                </Button>
                <Button
                  type="button"
                  variant="ghost"
                  disabled={apiUnavailable}
                  loading={busyAction === `doctor-${runner.id}`}
                  onClick={() => runDoctor(runner.id)}
                >
                  Doctor
                </Button>
                <Button
                  type="button"
                  variant="ghost"
                  disabled={apiUnavailable || !runner.installed}
                  loading={busyAction === `remove-${runner.id}`}
                  onClick={() => runAction(`remove-${runner.id}`, () => removeRunner(runner.id))}
                >
                  Remove
                </Button>
              </span>
            </TableRow>
          ))}
        </Table>
        {notice && <p className="notice-line">{notice}</p>}
      </Section>

      <Section title="Runner detail">
        {doctor ? (
          <div className="details-panel">
            <div className="details-panel__main">
              <h3>{String(doctor.name ?? doctor.id ?? "Runner doctor")}</h3>
              <div className="detail-grid">
                <span><strong>ID</strong>{String(doctor.id ?? "-")}</span>
                <span><strong>Contract</strong>{String(doctor.contract_installed ?? "-")}</span>
                <span><strong>Runtime state</strong>{String(doctor.runtime_state ?? "-")}</span>
                <span><strong>Runtime path</strong>{String(doctor.runtime_path ?? "-")}</span>
                <span><strong>Logs</strong>{String(doctor.logs_path ?? "-")}</span>
                <span><strong>Note</strong>{String(doctor.note ?? "-")}</span>
              </div>
              {adapterRecords.length > 0 ? (
                <div className="detail-grid">
                  {adapterRecords.map((adapter) => (
                    <span key={String(adapter.id ?? adapter.model_family ?? "adapter")}>
                      <strong>{String(adapter.id ?? "adapter")}</strong>{String(adapter.state ?? "unknown")}
                      {String(adapter.id ?? "") === "qwen3_tts" && String(adapter.state ?? "") !== "ready" ? (
                        <Button
                          type="button"
                          variant="ghost"
                          disabled={apiUnavailable || doctor.runtime_state !== "ready"}
                          loading={busyAction === "install-qwen3_tts"}
                          onClick={() => runAction("install-qwen3_tts", () => installAdapter("qwen3_tts"))}
                        >
                          Install Qwen adapter
                        </Button>
                      ) : null}
                      {String(adapter.notes ?? "") ? <small>{String(adapter.notes)}</small> : null}
                    </span>
                  ))}
                </div>
              ) : null}
            </div>
            <div className="details-panel__side">
              <Badge tone={doctor.runtime_state === "ready" ? "success" : "warning"}>{String(doctor.runtime_state ?? "unknown")}</Badge>
              <span className="details-panel__runner">Takokit keeps runner dependencies under the local storage root.</span>
            </div>
          </div>
        ) : (
          <div className="empty-state">
            <strong>No runner selected</strong>
            <p>Run doctor on a runner to inspect runtime paths, logs, and adapter slots.</p>
          </div>
        )}
      </Section>
    </section>
  );
}
