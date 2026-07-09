import { useEffect, useMemo, useState } from "react";
import type { RouteComponentProps } from "../../app/routes";
import { Badge } from "../../components/ui/Badge";
import { Section } from "../../components/ui/Section";
import { Table, TableRow } from "../../components/ui/Table";
import { getLibraryModels, getLibraryRunners, type LibraryEntry } from "../../lib/api";

export function LibraryPage({ runtime }: RouteComponentProps) {
  const [models, setModels] = useState<LibraryEntry[]>([]);
  const [runners, setRunners] = useState<LibraryEntry[]>([]);
  const [notice, setNotice] = useState<string | null>(null);
  const apiUnavailable = runtime.server.status !== "online";

  useEffect(() => {
    let cancelled = false;
    if (apiUnavailable) {
      setNotice("Start takokit serve or takokit gui to load the curated library registry.");
      return;
    }

    Promise.all([getLibraryModels(), getLibraryRunners()])
      .then(([modelData, runnerData]) => {
        if (cancelled) return;
        setModels(modelData);
        setRunners(runnerData);
        setNotice(null);
      })
      .catch((error) => {
        if (!cancelled) setNotice(error instanceof Error ? error.message : "Library registry failed to load.");
      });

    return () => {
      cancelled = true;
    };
  }, [apiUnavailable]);

  const runtimeModelIds = useMemo(() => new Set(runtime.models.map((model) => model.id)), [runtime.models]);

  return (
    <section className="page">
      <header className="page__header">
        <h1>Library</h1>
        <p>Curated discovery entries are separate from runtime manifests and do not imply executable support.</p>
      </header>

      <div className="stats-grid">
        <div className="stat-tile"><span>Library models</span><strong className="stat-tile__value">{models.length || "-"}</strong><small>Curated entries</small></div>
        <div className="stat-tile"><span>Runtime models</span><strong className="stat-tile__value">{runtime.models.length}</strong><small>Installed planner knows</small></div>
        <div className="stat-tile"><span>Library runners</span><strong className="stat-tile__value">{runners.length || "-"}</strong><small>Shared runtime families</small></div>
        <div className="stat-tile"><span>Policy</span><strong>honest</strong><small>No fake support labels</small></div>
      </div>

      <Section title="Runtime vs library" description={runtime.modeNote}>
        <div className="capability-strip">
          <div className="capability-chip">
            <strong>Runtime models</strong>
            <span>Have manifests under registry/models and participate in plan, pull, test, and execution checks.</span>
          </div>
          <div className="capability-chip">
            <strong>Library models</strong>
            <span>Are curated discovery records. They can document licenses and targets before runnable artifacts exist.</span>
          </div>
          <div className="capability-chip">
            <strong>Runner families</strong>
            <span>Shared runners support many model families without turning Takokit into a one-model wrapper.</span>
          </div>
        </div>
      </Section>

      <Section title="Curated models">
        {notice && <p className="notice-line">{notice}</p>}
        <Table columns={["Model", "Family", "Tasks", "Runtime", "Status"]} ariaLabel="Library models">
          {models.map((model) => {
            const id = text(model.id);
            return (
              <TableRow key={id}>
                <div>
                  <strong>{text(model.name) || id}</strong>
                  <span className="table-note">{text(model.license) || "license unknown"}</span>
                </div>
                <span>{text(model.family) || "-"}</span>
                <span>{list(model.tasks)}</span>
                <span>{text(model.runner) || "-"}</span>
                <span className="badge-list">
                  <Badge tone={runtimeModelIds.has(id) ? "success" : "neutral"}>{runtimeModelIds.has(id) ? "runtime manifest" : "library only"}</Badge>
                  <Badge tone={text(model.commercial_use) === "yes" ? "success" : "warning"}>commercial {text(model.commercial_use) || "unknown"}</Badge>
                </span>
              </TableRow>
            );
          })}
        </Table>
      </Section>

      <Section title="Curated runners">
        <Table columns={["Runner", "Kind", "Platforms", "Status", "Notes"]} ariaLabel="Library runners">
          {runners.map((runner) => (
            <TableRow key={text(runner.id)}>
              <strong>{text(runner.name) || text(runner.id)}</strong>
              <span>{text(runner.kind) || "-"}</span>
              <span>{list(runner.supported_platforms)}</span>
              <Badge tone={text(runner.runtime_status) === "ready" ? "success" : "neutral"}>{text(runner.runtime_status) || "planned"}</Badge>
              <span>{text(runner.notes) || "-"}</span>
            </TableRow>
          ))}
        </Table>
      </Section>
    </section>
  );
}

function text(value: unknown): string {
  return typeof value === "string" ? value : "";
}

function list(value: unknown): string {
  return Array.isArray(value) ? value.map((item) => String(item)).join(", ") : text(value) || "-";
}
