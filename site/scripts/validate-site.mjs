import { access, readFile, readdir } from "node:fs/promises";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { validateRegistry } from "../src/models/registry.js";

const siteRoot = fileURLToPath(new URL("../", import.meta.url));
const repoRoot = resolve(siteRoot, "..");
const required = [
  "index.html",
  "package.json",
  "vite.config.js",
  "vercel.json",
  "api/v1/registry.js",
  "src/main.jsx",
  "src/app/App.jsx",
  "src/app/router.js",
  "src/app/routes.js",
  "src/pages/HomePage.jsx",
  "src/pages/ModelsPage.jsx",
  "src/pages/ModelDetailPage.jsx",
  "src/pages/DocsPage.jsx",
  "src/pages/DownloadPage.jsx",
  "src/components/landing/CinematicHero.jsx",
  "src/components/landing/FeatureTaco.jsx",
  "src/components/landing/CapabilityPinwheel.jsx",
  "src/components/landing/RecommendedModelsScene.jsx",
  "src/components/landing/RuntimeFlow.jsx",
  "src/hooks/useScrollProgress.js",
  "src/models/registry.js",
  "src/models/filtering.js",
  "src/styles/index.css",
  "src/styles/landing/index.css",
];
const rootAssets = [
  "assets/svg-transparent/512.svg",
  "assets/svg-white/512-white.svg",
  "assets/favicon/favicon.ico",
  "assets/favicon/favicon-32x32.png",
  "assets/favicon/site.webmanifest",
];

for (const path of required) await access(resolve(siteRoot, path));
for (const path of rootAssets) await access(resolve(repoRoot, path));

const pkg = JSON.parse(await readFile(resolve(siteRoot, "package.json"), "utf8"));
if (pkg.scripts.build !== "vite build") throw new Error("canonical site build is not Vite");
if (pkg.scripts.build.includes("scripts/build.mjs")) throw new Error("obsolete static builder is active");

const vite = await readFile(resolve(siteRoot, "vite.config.js"), "utf8");
if (!vite.includes("takokit-root-brand-assets")) {
  throw new Error("the site is not serving canonical assets from the repository root");
}

const vercel = JSON.parse(await readFile(resolve(siteRoot, "vercel.json"), "utf8"));
if (vercel.framework !== "vite" || vercel.outputDirectory !== "dist") {
  throw new Error("Vercel is not configured for the canonical Vite output");
}
if (!vercel.rewrites.some((rule) => rule.destination === "/index.html")) {
  throw new Error("SPA rewrite is missing");
}
if (!vercel.rewrites.some((rule) => rule.source === "/v1/registry.json")) {
  throw new Error("registry rewrite is missing");
}

const app = await readFile(resolve(siteRoot, "src/app/App.jsx"), "utf8");
for (const page of ["HomePage", "ModelsPage", "ModelDetailPage", "DocsPage", "DownloadPage"]) {
  if (!app.includes(page)) throw new Error(`app router is missing ${page}`);
}

const home = await readFile(resolve(siteRoot, "src/pages/HomePage.jsx"), "utf8");
for (const scene of [
  "CinematicHero",
  "FeatureTaco",
  "CapabilityPinwheel",
  "RecommendedModelsScene",
  "RuntimeFlow",
]) {
  if (!home.includes(scene)) throw new Error(`cinematic homepage is missing ${scene}`);
}

const sourceEntries = await readdir(resolve(siteRoot, "src"), { recursive: true });
if (sourceEntries.some((path) => String(path).includes("assets/base.css"))) {
  throw new Error("obsolete static site source remains referenced");
}

const registryPath = process.env.TAKOKIT_REGISTRY_PATH || resolve(repoRoot, "registry/index.json");
const registry = JSON.parse(await readFile(registryPath, "utf8"));
const errors = validateRegistry(registry);
if (errors.length) throw new Error(`registry validation failed:\n${errors.join("\n")}`);

const families = registry.models.length;
const releases = registry.models.reduce((total, model) => total + model.tags.length, 0);
if (families < 1 || releases < families) throw new Error("registry catalog is unexpectedly empty");

console.log(`Takokit site validation passed: ${families} families / ${releases} releases`);
