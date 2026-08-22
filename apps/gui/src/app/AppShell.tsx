import type { ReactNode } from "react";
import { MainContent } from "../components/layout/MainContent";
import { ProductSidebar } from "../components/layout/ProductSidebar";
import { ProductTopBar } from "../components/layout/ProductTopBar";
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
    <div className="tk-shell">
      <ProductSidebar activePage={activePage} onNavigate={onNavigate} runtime={runtime} />
      <div className="tk-shell__workspace">
        <ProductTopBar runtime={runtime} onChangeWorkspace={onChangeWorkspace} onNavigate={onNavigate} />
        <MainContent pageId={activePage}>{children}</MainContent>
      </div>
    </div>
  );
}
