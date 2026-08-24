import { useEffect, useState } from "react";
import { AppShell } from "./AppShell";
import { type PageId } from "./navigation";
import { routes } from "./routes";
import { WorkspaceChooser } from "../components/workspace/WorkspaceChooser";
import { loadRuntimeSnapshot } from "../lib/api";
import { initializeWorkspaceSession } from "../lib/sessions";
import { getWorkspaceContext, workspaceNeedsSelection } from "../lib/workspace";
import type { RuntimeSnapshot } from "../lib/types";

const routeIds = new Set<PageId>(routes.map((route) => route.id));

function pageFromHash(): PageId {
  const hash = window.location.hash.replace("#", "");
  return routeIds.has(hash as PageId) ? (hash as PageId) : "home";
}

function emptyRuntime(): RuntimeSnapshot {
  const workspace = getWorkspaceContext().workspace ?? "Not selected";
  return {
    storagePath: "Unavailable until the daemon responds",
    workspacePath: workspace,
    buildId: "unknown",
    server: {
      status: "offline",
      url: window.location.origin,
      uptime: "Not connected"
    },
    models: [],
    catalogModels: [],
    runners: [],
    voices: [],
    capabilities: [],
    modeNote: "Takokit has not confirmed local runtime state."
  };
}

export function App() {
  const [activePage, setActivePage] = useState<PageId>(() => pageFromHash());
  const [runtime, setRuntime] = useState<RuntimeSnapshot>(() => emptyRuntime());
  const [workspaceOpen, setWorkspaceOpen] = useState(() => workspaceNeedsSelection());
  const [runtimeError, setRuntimeError] = useState<string | null>(null);
  const route = routes.find((item) => item.id === activePage) ?? routes[0];
  const Page = route.component;

  useEffect(() => {
    const onHashChange = () => setActivePage(pageFromHash());
    window.addEventListener("hashchange", onHashChange);
    return () => window.removeEventListener("hashchange", onHashChange);
  }, []);

  useEffect(() => {
    let cancelled = false;
    async function initialize() {
      try {
        await initializeWorkspaceSession();
      } catch (error) {
        if (!cancelled) {
          setRuntimeError(error instanceof Error ? error.message : "The selected session could not be opened.");
        }
      }
      if (!cancelled) await refreshRuntime();
    }
    void initialize();
    return () => {
      cancelled = true;
    };
  }, []);

  async function refreshRuntime() {
    try {
      const snapshot = await loadRuntimeSnapshot();
      setRuntime(snapshot);
      setRuntimeError(null);
    } catch (error) {
      setRuntime((current) => ({
        ...emptyRuntime(),
        workspacePath: getWorkspaceContext().workspace ?? current.workspacePath
      }));
      setRuntimeError(error instanceof Error ? error.message : "Takokit daemon is unavailable.");
    }
  }

  async function workspaceSelected() {
    setWorkspaceOpen(false);
    await refreshRuntime();
  }

  function navigate(page: PageId) {
    setActivePage(page);
    const url = new URL(window.location.href);
    url.hash = page;
    window.history.replaceState(null, "", `${url.pathname}${url.search}${url.hash}`);
  }

  return (
    <>
      <AppShell
        activePage={activePage}
        onNavigate={navigate}
        onChangeWorkspace={() => setWorkspaceOpen(true)}
        runtime={runtime}
      >
        {runtimeError && (
          <div className="runtime-error" role="alert">
            <strong>Takokit backend error</strong>
            <span>{runtimeError}</span>
            <button type="button" onClick={() => void refreshRuntime()}>Retry</button>
          </div>
        )}
        <Page runtime={runtime} onNavigate={navigate} onRefresh={refreshRuntime} />
      </AppShell>
      <WorkspaceChooser
        open={workspaceOpen}
        onClose={workspaceNeedsSelection() ? undefined : () => setWorkspaceOpen(false)}
        onSelected={workspaceSelected}
      />
    </>
  );
}
