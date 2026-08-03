import { createElement, useEffect, useMemo, useState } from "react";
import { matchRoute } from "./routes.js";

export { matchRoute, normalizePath } from "./routes.js";

export function useLocationRoute() {
  const read = () => ({
    pathname: window.location.pathname,
    search: window.location.search,
  });
  const [location, setLocation] = useState(read);

  useEffect(() => {
    const update = () => setLocation(read());
    const click = (event) => {
      const link = event.target.closest("a[data-route]");
      if (
        !link ||
        event.defaultPrevented ||
        event.button !== 0 ||
        event.metaKey ||
        event.ctrlKey ||
        event.shiftKey ||
        event.altKey
      ) return;
      const url = new URL(link.href, window.location.href);
      if (url.origin !== window.location.origin) return;
      event.preventDefault();
      window.history.pushState({}, "", url.pathname + url.search + url.hash);
      update();
      window.scrollTo({ top: 0, behavior: "auto" });
    };
    window.addEventListener("popstate", update);
    document.addEventListener("click", click);
    return () => {
      window.removeEventListener("popstate", update);
      document.removeEventListener("click", click);
    };
  }, []);

  return useMemo(() => ({
    ...location,
    route: matchRoute(location.pathname),
    query: new URLSearchParams(location.search),
  }), [location]);
}

export function navigate(href, { replace = false } = {}) {
  const url = new URL(href, window.location.href);
  window.history[replace ? "replaceState" : "pushState"](
    {},
    "",
    url.pathname + url.search + url.hash,
  );
  window.dispatchEvent(new PopStateEvent("popstate"));
}

export const RouteLink = ({ href, children, className = "", ...props }) =>
  createElement("a", {
    href,
    "data-route": true,
    className,
    ...props,
  }, children);
