import { RouteLink } from "../app/router";
import { CommandBar } from "../components/CommandBar";
import { DOC_GROUPS, findDoc } from "../docs/content";

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

  return (
    <main className="shell page docs-layout">
      <aside className="docs-nav" aria-label="Documentation">
        {DOC_GROUPS.map((group) => (
          <section key={group.title}>
            <h2>{group.title}</h2>
            {group.pages.map(([id, title]) => (
              <RouteLink
                key={id}
                href={`/docs/${id}`}
                className={id === slug ? "is-active" : ""}
                aria-current={id === slug ? "page" : undefined}
              >
                {title}
              </RouteLink>
            ))}
          </section>
        ))}
      </aside>
      <article className="docs-article">
        <header>
          <p className="eyebrow">Documentation</p>
          <h1>{doc.title}</h1>
          <p>{doc.intro}</p>
        </header>
        {doc.sections?.map(([title, text]) => (
          <section key={title}><h2>{title}</h2><p>{text}</p></section>
        ))}
        {doc.commands?.length > 0 && (
          <section><h2>Commands</h2>{doc.commands.map((command) => <CommandBar key={command}>{command}</CommandBar>)}</section>
        )}
        {doc.code && <section><h2>Example</h2><CommandBar>{doc.code}</CommandBar></section>}
      </article>
    </main>
  );
}
