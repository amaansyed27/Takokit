import { RouteLink } from "../app/router";

export function NotFoundPage() {
  return (
    <main className="shell page not-found">
      <p className="eyebrow">404</p>
      <h1>Page not found</h1>
      <p>The requested Takokit page does not exist.</p>
      <RouteLink href="/" className="button button-primary">Return home</RouteLink>
    </main>
  );
}
