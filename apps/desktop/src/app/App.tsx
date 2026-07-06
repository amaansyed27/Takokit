import { useEffect, useState } from "react";
import { AppShell } from "./AppShell";
import { type PageId } from "./navigation";
import { routes } from "./routes";
import { mockRuntime } from "../lib/mockData";

const routeIds = new Set<PageId>(routes.map((route) => route.id));

function pageFromHash(): PageId {
  const hash = window.location.hash.replace("#", "");
  return routeIds.has(hash as PageId) ? (hash as PageId) : "home";
}

export function App() {
  const [activePage, setActivePage] = useState<PageId>(() => pageFromHash());
  const route = routes.find((item) => item.id === activePage) ?? routes[0];
  const Page = route.component;

  useEffect(() => {
    const onHashChange = () => setActivePage(pageFromHash());
    window.addEventListener("hashchange", onHashChange);
    return () => window.removeEventListener("hashchange", onHashChange);
  }, []);

  function navigate(page: PageId) {
    setActivePage(page);
    window.history.replaceState(null, "", `#${page}`);
  }

  return (
    <AppShell activePage={activePage} onNavigate={navigate} runtime={mockRuntime}>
      <Page runtime={mockRuntime} onNavigate={navigate} />
    </AppShell>
  );
}
