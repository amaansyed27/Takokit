import { access, readFile } from "node:fs/promises";

const root = new URL("../", import.meta.url);
for (const path of [
  "index.html",
  "library/index.html",
  "model.html",
  "docs/index.html",
  "download/index.html",
  "assets/app.js",
  "assets/base.css",
  "assets/pages.css",
  "assets/detail.css",
  "assets/responsive.css",
  "assets/brand/takokit-mark.svg",
  "assets/brand/takokit-wordmark.svg",
  "assets/brand/takokit-lockup.svg",
  "api/v1/registry.js",
  "vercel.json",
]) {
  await access(new URL(path, root));
}

const registryPath = new URL("../../registry/index.json", import.meta.url);
try {
  const registry = JSON.parse(await readFile(registryPath, "utf8"));
  const releases = registry.models.reduce((total, model) => total + model.tags.length, 0);
  if (registry.schema_version !== 1 || registry.models.length !== 24 || releases !== 31) {
    throw new Error(`unexpected registry shape: ${registry.models.length} families / ${releases} releases`);
  }
} catch (error) {
  if (error.code !== "ENOENT") throw error;
  console.warn("Registry file is outside this standalone checkout; skipped cross-tree validation.");
}

const script = await readFile(new URL("assets/app.js", root), "utf8");
if (!script.includes('"/v1/registry.json"') || !script.includes("tako pull")) {
  throw new Error("site client is not wired to the registry");
}

for (const marker of ["mountLibrary", "mountModel", "mountChrome"]) {
  if (!script.includes(marker)) {
    throw new Error(`site client is missing ${marker}`);
  }
}

const pages = await Promise.all(
  ["index.html", "library/index.html", "model.html", "docs/index.html", "download/index.html"].map(
    (path) => readFile(new URL(path, root), "utf8"),
  ),
);
for (const [index, page] of pages.entries()) {
  if (!page.includes("data-site-header") || !page.includes("data-site-footer")) {
    throw new Error(`page ${index + 1} is not wired to shared site chrome`);
  }
}
console.log("Takokit Library site validation passed");
