export function DocsTableOfContents({ sections }) {
  if (!sections.length) return null;

  return (
    <aside className="docs-toc" aria-label="On this page">
      <strong>On this page</strong>
      <nav>
        {sections.map((section) => (
          <a href={`#${section.id}`} key={section.id}>{section.title}</a>
        ))}
      </nav>
    </aside>
  );
}
