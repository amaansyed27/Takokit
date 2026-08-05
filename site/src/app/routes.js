export function normalizePath(pathname) {
  const clean = pathname.replace(/\/+/g, "/").replace(/\/$/, "");
  return clean || "/";
}

export function matchRoute(pathname) {
  const path = normalizePath(pathname);
  if (path === "/") return { name: "home", params: {} };
  if (path === "/models" || path === "/library") {
    return { name: "models", params: {}, legacy: path === "/library" };
  }
  if (path === "/download") return { name: "download", params: {} };
  if (path === "/docs" || path.startsWith("/docs/")) {
    return {
      name: "docs",
      params: { slug: path === "/docs" ? "install" : path.slice("/docs/".length) },
    };
  }
  for (const base of ["/models/", "/library/"]) {
    if (!path.startsWith(base)) continue;
    const encoded = path.slice(base.length);
    const legacy = base === "/library/";
    if (!encoded) break;
    if (legacy && encoded.includes(":")) {
      const split = encoded.indexOf(":");
      return {
        name: "model",
        legacy,
        params: {
          model: decodeURIComponent(encoded.slice(0, split)),
          tag: decodeURIComponent(encoded.slice(split + 1)),
        },
      };
    }
    const segments = encoded.split("/").map(decodeURIComponent);
    return {
      name: "model",
      legacy,
      params: { model: segments[0], tag: segments[1] || undefined },
    };
  }
  return { name: "not-found", params: {} };
}
