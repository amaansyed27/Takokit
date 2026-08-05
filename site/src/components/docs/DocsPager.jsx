import { RouteLink } from "../../app/router";

export function DocsPager({ previous, next }) {
  return (
    <nav className="docs-pager" aria-label="Documentation pagination">
      {previous ? (
        <RouteLink href={`/docs/${previous.id}`} className="docs-pager__link docs-pager__link--previous">
          <span>Previous</span>
          <strong>← {previous.title}</strong>
        </RouteLink>
      ) : <span />}
      {next ? (
        <RouteLink href={`/docs/${next.id}`} className="docs-pager__link docs-pager__link--next">
          <span>Next</span>
          <strong>{next.title} →</strong>
        </RouteLink>
      ) : <span />}
    </nav>
  );
}
