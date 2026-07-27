import { access, readFile } from "node:fs/promises";

const root = new URL("../", import.meta.url);
for (const path of [
  "index.html",
  "vite.config.js",
  "public/brand/takokit-mark.svg",
  "public/brand/takokit-lockup.svg",
  "src/main.jsx",
  "src/App.jsx",
  "src/router.js",
  "src/registry.js",
  "src/styles.css",
  "src/components/Chrome.jsx",
  "api/v1/registry.js",
  "vercel.json",
]) {
  await access(new URL(path, root));
}

const pkg = JSON.parse(await readFile(new URL("package.json", root), "utf8"));
if (!pkg.dependencies.react || !pkg.dependencies.vite || !pkg.dependencies["@vitejs/plugin-react"]) {
  throw new Error("React/Vite dependencies are missing");
}

const app = await readFile(new URL("src/App.jsx", root), "utf8");
const router = await readFile(new URL("src/router.js", root), "utf8");
if (router.includes("<a ")) {
  throw new Error("src/router.js contains JSX but does not use a .jsx extension");
}
const chrome = await readFile(new URL("src/components/Chrome.jsx", root), "utf8");
for (const [name, source] of [["App.jsx", app], ["Chrome.jsx", chrome]]) {
  if (source.includes("assets/brand/")) {
    throw new Error(`${name} references the deleted legacy brand directory`);
  }
}
for (const route of ["/library", "/docs", "/download"]) {
  if (!app.includes(route)) throw new Error(`missing React route ${route}`);
}

const registryPath = new URL("../../registry/index.json", import.meta.url);
const registry = JSON.parse(await readFile(registryPath, "utf8"));
const releases = registry.models.reduce((total, model) => total + model.tags.length, 0);
if (registry.schema_version !== 1 || registry.models.length !== 24 || releases !== 31) {
  throw new Error(`unexpected registry shape: ${registry.models.length} families / ${releases} releases`);
}

console.log("Takokit React site validation passed");
