import type { ReactNode } from "react";
import { MainContent } from "../components/layout/MainContent";
import { Sidebar } from "../components/layout/Sidebar";
import type { RuntimeSnapshot } from "../lib/types";
import type { PageId } from "./navigation";

type AppShellProps = {
  activePage: PageId;
  onNavigate: (page: PageId) => void;
  runtime: RuntimeSnapshot;
  children: ReactNode;
};

export function AppShell({ activePage, onNavigate, runtime, children }: AppShellProps) {
  return (
    <div className="app-shell">
      <Sidebar activePage={activePage} onNavigate={onNavigate} runtime={runtime} />
      <MainContent>{children}</MainContent>
    </div>
  );
}

