import type { ReactNode } from "react";

type MainContentProps = {
  children: ReactNode;
  pageId: string;
};

export function MainContent({ children, pageId }: MainContentProps) {
  return (
    <main className="main-content">
      <div className="page-transition" key={pageId}>{children}</div>
    </main>
  );
}
