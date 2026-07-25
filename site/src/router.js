import { useEffect, useState } from "react";

export function useRoute() {
  const [route, setRoute] = useState(() => location.pathname + location.search);
  useEffect(() => {
    const update = () => setRoute(location.pathname + location.search);
    const click = (event) => {
      const link = event.target.closest("a[data-route]");
      if (!link || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
      event.preventDefault();
      history.pushState({}, "", link.getAttribute("href"));
      update();
      window.scrollTo({ top: 0, behavior: "instant" });
    };
    addEventListener("popstate", update);
    document.addEventListener("click", click);
    return () => {
      removeEventListener("popstate", update);
      document.removeEventListener("click", click);
    };
  }, []);
  return route;
}

export const RouteLink = ({ href, children, className = "", ...props }) => (
  <a href={href} data-route className={className} {...props}>{children}</a>
);
