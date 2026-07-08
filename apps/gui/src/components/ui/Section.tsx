import type { ReactNode } from "react";

type SectionProps = {
  title?: string;
  description?: string;
  children: ReactNode;
  className?: string;
};

export function Section({ title, description, children, className = "" }: SectionProps) {
  return (
    <section className={`section ${className}`}>
      {(title || description) && (
        <header className="section__header">
          {title && <h2>{title}</h2>}
          {description && <p>{description}</p>}
        </header>
      )}
      {children}
    </section>
  );
}

