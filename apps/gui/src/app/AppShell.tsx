import type { ReactNode } from "react";
import { MainContent } from "../components/layout/MainContent";
import { Sidebar } from "../components/layout/Sidebar";
import { Button } from "../components/ui/Button";
import type { RuntimeSnapshot } from "../lib/types";
import type { PageId } from "./navigation";

type AppShellProps = {
  activePage: PageId;
  onNavigate: (page: PageId) => void;
  onChangeWorkspace: () => void;
  runtime: RuntimeSnapshot;
  children: ReactNode;
};

export function AppShell({ activePage, onNavigate, onChangeWorkspace, runtime, children }: AppShellProps) {
  return (
    <div className="app-shell">
      <Sidebar activePage={activePage} onNavigate={onNavigate} runtime={runtime} />
      <div className="app-shell__content">
        <div className="workspace-bar">
          <div>
            <span>Active workspace</span>
            <strong title={runtime.workspacePath}>{runtime.workspacePath}</strong>
          </div>
          <div className="workspace-bar__meta">
            <span>Build {runtime.buildId}</span>
            <Button type="button" variant="ghost" onClick={onChangeWorkspace}>Change workspace</Button>
          </div>
        </div>
        <MainContent pageId={activePage}>{children}</MainContent>
      </div>
    </div>
  );
}
