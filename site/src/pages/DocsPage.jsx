import { RouteLink } from "../app/router";
import { DocsCodeBlock } from "../components/docs/DocsCodeBlock";
import { DocsPager } from "../components/docs/DocsPager";
import { DocsSidebar } from "../components/docs/DocsSidebar";
import { DocsTableOfContents } from "../components/docs/DocsTableOfContents";
import {
  DOC_GROUPS,
  adjacentDocs,
  findDoc,
  findDocGroup,
} from "../docs/content";

function DocSection({ section }) {
  return (
    <section className="docs-content__section" id={section.id} aria-labelledby={`${section.id}-title`}>
      <h2 id={`${section.id}-title`}>
        <a href={`#${section.id}`}>{section.title}</a>
      </h2>
      {section.body?.map((paragraph) => <p key={paragraph}>{paragraph}</p>)}
      {section.commands?.map((command) => (
        <DocsCodeBlock key={command} label="Command">{command}</DocsCodeBlock>
      ))}
      {section.code && <DocsCodeBlock label="Example">{section.code}</DocsCodeBlock>}
      {section.note && <aside className="docs-note"><strong>Note</strong><p>{section.note}</p></aside>}
    </section>
  );
}

export function DocsPage({ slug }) {
  const doc = findDoc(slug);
  if (!doc) {
    return (
      <main className="shell page not-found">
        <p className="eyebrow">Documentation</p>
        <h1>Document not found</h1>
        <RouteLink href="/docs">Open documentation</RouteLink>
      </main>
    );
  }

  const group = findDocGroup(slug);
  const { previous, next } = adjacentDocs(slug);
  const sections = doc.sections || [];

  return (
    <main className="docs-page">
      <div className="docs-shell">
        <DocsSidebar groups={DOC_GROUPS} slug={slug} />

        <article className="docs-content">
          <nav className="docs-breadcrumbs" aria-label="Breadcrumb">
            <RouteLink href="/docs">Docs</RouteLink>
            <span aria-hidden="true">/</span>
            <span>{group?.title || "Documentation"}</span>
          </nav>

          <header className="docs-content__header">
            <p className="docs-content__category">{group?.title || "Documentation"}</p>
            <h1>{doc.title}</h1>
            <p>{doc.intro}</p>
          </header>

          <div className="docs-content__body">
            {sections.map((section) => <DocSection key={section.id} section={section} />)}
          </div>

          <DocsPager previous={previous} next={next} />
        </article>

        <DocsTableOfContents sections={sections} />
      </div>
    </main>
  );
}
