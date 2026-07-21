import { useEffect, useState } from "react";
import { AppShell } from "./AppShell";
import { type PageId } from "./navigation";
import { routes } from "./routes";
import { mockRuntime } from "../lib/mockData";
import { loadRuntimeSnapshot } from "../lib/api";
import { withVerifiedInstalledModels } from "../lib/installedModels";
import { initializeWorkspaceSession } from "../lib/sessions";
import type { RuntimeSnapshot } from "../lib/types";

const routeIds = new Set<PageId>(routes.map((route) => route.id));

function pageFromHash(): PageId {
  const hash = window.location.hash.replace("#", "");
  return routeIds.has(hash as PageId) ? (hash as PageId) : "home";
}

export function App() {
  const [activePage, setActivePage] = useState<PageId>(() => pageFromHash());
  const [runtime, setRuntime] = useState<RuntimeSnapshot>(() => ({ ...mockRuntime, models: [] }));
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
      } finally {
        if (!cancelled) await refreshRuntime();
      }
    }
    void initialize();
    return () => {
      cancelled = true;
    };
  }, []);

  async function refreshRuntime() {
    const snapshot = await loadRuntimeSnapshot();
    try {
      setRuntime(await withVerifiedInstalledModels(snapshot));
    } catch {
      setRuntime({ ...snapshot, models: [] });
    }
  }

  function navigate(page: PageId) {
    setActivePage(page);
    const url = new URL(window.location.href);
    url.hash = page;
    window.history.replaceState(null, "", `${url.pathname}${url.search}${url.hash}`);
  }

  return (
    <AppShell activePage={activePage} onNavigate={navigate} runtime={runtime}>
      <Page runtime={runtime} onNavigate={navigate} onRefresh={refreshRuntime} />
    </AppShell>
  );
}
