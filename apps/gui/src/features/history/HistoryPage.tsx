import { Clock3, FileAudio, FileText, Play, Plus, RotateCcw, Search, Trash2 } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import type { RouteComponentProps } from "../../app/routes";
import { ConfirmDialog } from "../../components/ui/ConfirmDialog";
import { ProductButton } from "../../components/ui/ProductButton";
import { ProductPageHeader } from "../../components/ui/ProductPageHeader";
import {
  createSession,
  getSession,
  listSessions,
  loadSessionOutput,
  outputFilename,
  removeSession,
  resumeSession
} from "../../lib/sessions";
import type { SessionEvent, SessionRecord, SessionSummary } from "../../lib/types";
import { getWorkspaceContext } from "../../lib/workspace";

export function HistoryPage({ onRefresh }: RouteComponentProps) {
  const [query, setQuery] = useState("");
  const [sessions, setSessions] = useState<SessionSummary[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [record, setRecord] = useState<SessionRecord | null>(null);
  const [loading, setLoading] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [deleteOpen, setDeleteOpen] = useState(false);
  const activeSession = getWorkspaceContext().session;

  useEffect(() => {
    const timer = window.setTimeout(() => void refreshSessions(query), 160);
    return () => window.clearTimeout(timer);
  }, [query]);

  useEffect(() => {
    if (!selectedId) {
      setRecord(null);
      return;
    }
    let cancelled = false;
    void getSession(selectedId)
      .then((next) => { if (!cancelled) setRecord(next); })
      .catch((error) => { if (!cancelled) setNotice(error instanceof Error ? error.message : "Session could not be read."); });
    return () => { cancelled = true; };
  }, [selectedId]);

  async function refreshSessions(search = query) {
    setLoading(true);
    try {
      const next = await listSessions(search);
      setSessions(next);
      setSelectedId((current) => {
        if (current && next.some((session) => session.id === current)) return current;
        if (activeSession && next.some((session) => session.id === activeSession)) return activeSession;
        return next[0]?.id ?? null;
      });
    } catch (error) {
      setNotice(error instanceof Error ? error.message : "Project history could not be loaded.");
    } finally {
      setLoading(false);
    }
  }

  async function createNew() {
    setLoading(true);
    setNotice(null);
    try {
      const next = await createSession();
      setSelectedId(next.summary.id);
      setRecord(next);
      setQuery("");
      await refreshSessions("");
      await onRefresh();
    } catch (error) {
      setNotice(error instanceof Error ? error.message : "A new session could not be created.");
    } finally {
      setLoading(false);
    }
  }

  async function resumeSelected() {
    if (!selectedId) return;
    setLoading(true);
    setNotice(null);
    try {
      const next = await resumeSession(selectedId);
      setRecord(next);
      await onRefresh();
      await refreshSessions();
    } catch (error) {
      setNotice(error instanceof Error ? error.message : "The session could not be opened.");
    } finally {
      setLoading(false);
    }
  }

  async function deleteSelected() {
    if (!selectedId || selectedId === activeSession) return;
    setLoading(true);
    setNotice(null);
    try {
      await removeSession(selectedId);
      setDeleteOpen(false);
      setSelectedId(null);
      setRecord(null);
      await refreshSessions();
    } catch (error) {
      setNotice(error instanceof Error ? error.message : "The session could not be removed.");
    } finally {
      setLoading(false);
    }
  }

  return (
    <section className="tk-page tk-history-page">
      <ProductPageHeader
        eyebrow="Workspace activity"
        title="History"
        description="Reopen past sessions, review transcripts, and play generated audio saved inside this workspace."
        actions={<ProductButton tone="primary" loading={loading} onClick={() => void createNew()}><Plus size={15} /> New session</ProductButton>}
      />

      <label className="tk-history-search"><Search size={15} /><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Search sessions, models, transcripts, or errors" /></label>
      {notice ? <div className="tk-inline-error" role="status">{notice}</div> : null}

      <div className="tk-history-layout">
        <aside className="tk-history-sessions">
          <div className="tk-system-panel__header"><div><h2>Sessions</h2><p>{sessions.length} in this workspace</p></div></div>
          <div className="tk-history-session-list">
            {sessions.map((session) => (
              <button className={session.id === selectedId ? "tk-history-session is-active" : "tk-history-session"} key={session.id} type="button" onClick={() => setSelectedId(session.id)}>
                <div><strong>{session.title}</strong><span>{formatTime(session.updated_at)}</span></div>
                <div className="tk-history-session__meta">
                  {session.id === activeSession ? <span className="is-active">Active</span> : null}
                  <span>{session.output_count} outputs</span>
                  <span>{session.event_count} events</span>
                </div>
              </button>
            ))}
            {!loading && sessions.length === 0 ? <div className="tk-system-empty"><Clock3 size={20} /><div><strong>No sessions found</strong><span>Run a workflow or create a new session.</span></div></div> : null}
          </div>
        </aside>

        <section className="tk-history-detail">
          {record ? (
            <>
              <header className="tk-history-detail__header">
                <div><span>{record.summary.id === activeSession ? "Active session" : "Saved session"}</span><h2>{record.summary.title}</h2><p>{formatTime(record.summary.updated_at)} · {record.summary.output_count} outputs</p></div>
                <div className="tk-history-detail__actions">
                  {record.summary.id !== activeSession ? <ProductButton tone="secondary" loading={loading} onClick={() => void resumeSelected()}><RotateCcw size={14} /> Open session</ProductButton> : null}
                  {record.summary.id !== activeSession ? <button className="tk-row-icon-action is-danger" type="button" title="Delete session" onClick={() => setDeleteOpen(true)}><Trash2 size={15} /></button> : null}
                </div>
              </header>
              <div className="tk-history-timeline">
                {record.events.slice().reverse().map((event) => <HistoryEvent key={event.id} event={event} />)}
                {record.events.length === 0 ? <div className="tk-system-empty"><Clock3 size={20} /><div><strong>Empty session</strong><span>New Takokit workflow activity will appear here.</span></div></div> : null}
              </div>
            </>
          ) : <div className="tk-history-empty"><Clock3 size={25} /><strong>Select a session</strong><span>Its workflow timeline and outputs will appear here.</span></div>}
        </section>
      </div>

      <ConfirmDialog
        open={deleteOpen}
        title="Delete this session?"
        description={<div className="tk-confirm-copy"><p>This removes the selected workspace session and its saved session data. The active session cannot be deleted.</p></div>}
        confirmLabel="Delete session"
        destructive
        busy={loading}
        onCancel={() => setDeleteOpen(false)}
        onConfirm={() => void deleteSelected()}
      />
    </section>
  );
}

function HistoryEvent({ event }: { event: SessionEvent }) {
  const [outputUrl, setOutputUrl] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const audio = useMemo(() => isAudio(event.output_path), [event.output_path]);

  useEffect(() => () => { if (outputUrl) URL.revokeObjectURL(outputUrl); }, [outputUrl]);

  async function loadOutput() {
    if (!event.output_path) return;
    setLoading(true);
    try {
      const url = await loadSessionOutput(event.session_id, event.output_path);
      setOutputUrl((previous) => { if (previous) URL.revokeObjectURL(previous); return url; });
    } finally {
      setLoading(false);
    }
  }

  return (
    <article className="tk-history-event">
      <div className="tk-history-event__rail"><span className={event.state === "failed" ? "is-failed" : event.state === "completed" ? "is-complete" : ""} /></div>
      <div className="tk-history-event__body">
        <header>
          <span className="tk-history-event__icon">{audio ? <FileAudio size={16} /> : <FileText size={16} />}</span>
          <div><strong>{taskLabel(event.task)}</strong><span>{formatTime(event.timestamp)} · {event.model ?? "Takokit"}</span></div>
          <span className={event.state === "failed" ? "tk-event-state is-failed" : event.state === "completed" ? "tk-event-state is-complete" : "tk-event-state"}>{event.state}</span>
        </header>
        {event.input ? <p className="tk-history-event__input">{event.input}</p> : null}
        {event.text ? <div className="tk-history-event__text">{event.text}</div> : null}
        {event.message ? <p className="tk-history-event__message">{event.message}</p> : null}
        {event.output_path ? (
          <div className="tk-history-output">
            <div><strong>{outputFilename(event.output_path)}</strong><span title={event.output_path}>{event.output_path}</span></div>
            {!outputUrl ? <ProductButton tone="ghost" loading={loading} onClick={() => void loadOutput()}><Play size={13} /> {audio ? "Play" : "Open"}</ProductButton> : audio ? <audio controls src={outputUrl} preload="metadata" /> : <a href={outputUrl} download={outputFilename(event.output_path)}>Download output</a>}
          </div>
        ) : null}
      </div>
    </article>
  );
}

function isAudio(path?: string): boolean { return Boolean(path && /\.(wav|mp3|flac|ogg|m4a|aac|wma)$/i.test(path)); }
function formatTime(timestamp: number): string { return new Date(timestamp * 1000).toLocaleString(); }
function taskLabel(task: SessionEvent["task"]): string { return task.split("_").map((part) => part[0].toUpperCase() + part.slice(1)).join(" "); }
