import { Clock3, FileAudio, FileText, Plus, RotateCcw, Search, Trash2 } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import type { RouteComponentProps } from "../../app/routes";
import { Badge } from "../../components/ui/Badge";
import { Button } from "../../components/ui/Button";
import { Section } from "../../components/ui/Section";
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
  const activeSession = getWorkspaceContext().session;

  useEffect(() => {
    const timer = window.setTimeout(() => void refreshSessions(query), 180);
    return () => window.clearTimeout(timer);
  }, [query]);

  useEffect(() => {
    if (!selectedId) {
      setRecord(null);
      return;
    }
    let cancelled = false;
    void getSession(selectedId)
      .then((next) => {
        if (!cancelled) setRecord(next);
      })
      .catch((error) => {
        if (!cancelled) setNotice(error instanceof Error ? error.message : "Could not read the session.");
      });
    return () => {
      cancelled = true;
    };
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
      setNotice(error instanceof Error ? error.message : "Could not load project history.");
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
      await refreshSessions("");
      setQuery("");
      setNotice("New project session created and activated.");
    } catch (error) {
      setNotice(error instanceof Error ? error.message : "Could not create a session.");
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
      setNotice(`Opened ${next.summary.title}. New output will be added to this session.`);
    } catch (error) {
      setNotice(error instanceof Error ? error.message : "Could not open the session.");
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
      setSelectedId(null);
      setRecord(null);
      await refreshSessions();
      setNotice("Session removed from this project.");
    } catch (error) {
      setNotice(error instanceof Error ? error.message : "Could not remove the session.");
    } finally {
      setLoading(false);
    }
  }

  return (
    <section className="page">
      <header className="page__header history-header">
        <div>
          <h1>History</h1>
          <p>Search and reopen sessions saved in this project&apos;s <code>.tako</code> directory.</p>
        </div>
        <Button type="button" variant="primary" loading={loading} onClick={() => void createNew()}>
          <Plus size={16} /> New session
        </Button>
      </header>

      <div className="history-search">
        <Search size={17} aria-hidden="true" />
        <input
          className="search-input"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder="Search transcripts, text, models, titles, or errors…"
          aria-label="Search session history"
        />
      </div>

      <div className="history-layout">
        <Section title={`Sessions · ${sessions.length}`}>
          <div className="history-list">
            {sessions.map((session) => (
              <button
                className={session.id === selectedId ? "history-session active" : "history-session"}
                key={session.id}
                type="button"
                onClick={() => setSelectedId(session.id)}
              >
                <span>
                  <strong>{session.title}</strong>
                  <small>{formatTime(session.updated_at)}</small>
                </span>
                <span className="badge-list">
                  {session.id === activeSession ? <Badge tone="success">active</Badge> : null}
                  <Badge tone="neutral">{session.event_count} events</Badge>
                  <Badge tone="neutral">{session.output_count} outputs</Badge>
                </span>
              </button>
            ))}
            {!loading && sessions.length === 0 ? (
              <div className="empty-state">
                <strong>No matching sessions</strong>
                <p>Generate speech or a transcript, or create a new session.</p>
              </div>
            ) : null}
          </div>
        </Section>

        <Section title={record?.summary.title ?? "Session details"}>
          {record ? (
            <div className="history-detail">
              <div className="history-actions">
                <Button type="button" variant="primary" loading={loading} onClick={() => void resumeSelected()}>
                  <RotateCcw size={16} /> Open session
                </Button>
                <Button
                  type="button"
                  variant="ghost"
                  disabled={record.summary.id === activeSession}
                  onClick={() => void deleteSelected()}
                >
                  <Trash2 size={16} /> Delete
                </Button>
              </div>
              <div className="detail-grid">
                <span><strong>Session ID</strong>{record.summary.id}</span>
                <span><strong>Workspace</strong>{record.summary.workspace_root}</span>
                <span><strong>Created</strong>{formatTime(record.summary.created_at)}</span>
                <span><strong>Updated</strong>{formatTime(record.summary.updated_at)}</span>
              </div>
              <div className="history-events">
                {record.events.slice().reverse().map((event) => (
                  <HistoryEvent key={event.id} event={event} />
                ))}
                {record.events.length === 0 ? (
                  <div className="empty-state">
                    <strong>Empty session</strong>
                    <p>New speech, transcription, cloning, and training activity will appear here.</p>
                  </div>
                ) : null}
              </div>
            </div>
          ) : (
            <div className="empty-state">
              <Clock3 size={24} />
              <strong>Select a session</strong>
              <p>Its activity, transcripts, and generated audio will appear here.</p>
            </div>
          )}
        </Section>
      </div>
      {notice ? <p className="notice-line">{notice}</p> : null}
    </section>
  );
}

function HistoryEvent({ event }: { event: SessionEvent }) {
  const [outputUrl, setOutputUrl] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const audio = useMemo(() => isAudio(event.output_path), [event.output_path]);

  useEffect(() => () => {
    if (outputUrl) URL.revokeObjectURL(outputUrl);
  }, [outputUrl]);

  async function loadOutput() {
    if (!event.output_path) return;
    setLoading(true);
    try {
      const url = await loadSessionOutput(event.session_id, event.output_path);
      setOutputUrl((previous) => {
        if (previous) URL.revokeObjectURL(previous);
        return url;
      });
    } finally {
      setLoading(false);
    }
  }

  return (
    <article className="history-event">
      <header>
        <span className="history-event__icon">{audio ? <FileAudio size={17} /> : <FileText size={17} />}</span>
        <div>
          <strong>{taskLabel(event.task)}</strong>
          <small>{formatTime(event.timestamp)} · {event.model ?? "no model"}</small>
        </div>
        <Badge tone={event.state === "completed" ? "success" : event.state === "failed" ? "warning" : "neutral"}>
          {event.state}
        </Badge>
      </header>
      {event.input ? <p>{event.input}</p> : null}
      {event.text ? <pre className="history-transcript">{event.text}</pre> : null}
      {event.message ? <p className="notice-line">{event.message}</p> : null}
      {event.output_path ? (
        <div className="history-output">
          <span>{outputFilename(event.output_path)}</span>
          {!outputUrl ? (
            <Button type="button" variant="ghost" loading={loading} onClick={() => void loadOutput()}>
              Load output
            </Button>
          ) : audio ? (
            <audio controls src={outputUrl} preload="metadata" />
          ) : (
            <a href={outputUrl} download={outputFilename(event.output_path)}>Open output</a>
          )}
        </div>
      ) : null}
    </article>
  );
}

function isAudio(path?: string): boolean {
  return Boolean(path && /\.(wav|mp3|flac|ogg)$/i.test(path));
}

function formatTime(timestamp: number): string {
  return new Date(timestamp * 1000).toLocaleString();
}

function taskLabel(task: SessionEvent["task"]): string {
  return task.split("_").map((part) => part[0].toUpperCase() + part.slice(1)).join(" ");
}
