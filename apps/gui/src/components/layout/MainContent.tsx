import type { ReactNode } from "react";

type MainContentProps = {
  children: ReactNode;
  pageId: string;
};

export function MainContent({ children, pageId }: MainContentProps) {
  return (
    <main className="tk-content">
      <div className="tk-content__inner">
        <div className="tk-page-transition" key={pageId}>{children}</div>
      </div>
    </main>
  );
}
