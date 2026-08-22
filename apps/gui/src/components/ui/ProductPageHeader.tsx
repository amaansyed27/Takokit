import type { ReactNode } from "react";

type ProductPageHeaderProps = {
  eyebrow?: string;
  title: string;
  description?: string;
  actions?: ReactNode;
};

export function ProductPageHeader({ eyebrow, title, description, actions }: ProductPageHeaderProps) {
  return (
    <header className="tk-page-header">
      <div className="tk-page-header__copy">
        {eyebrow ? <span className="tk-eyebrow">{eyebrow}</span> : null}
        <h1>{title}</h1>
        {description ? <p>{description}</p> : null}
      </div>
      {actions ? <div className="tk-page-header__actions">{actions}</div> : null}
    </header>
  );
}
