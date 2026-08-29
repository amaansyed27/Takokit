import { access, readFile, readdir } from "node:fs/promises";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { validateRegistry } from "../src/models/registry.js";

const siteRoot = fileURLToPath(new URL("../", import.meta.url));
const repoRoot = resolve(siteRoot, "..");
const landingComponents = [
  "LandingHero",
  "ProductCapabilities",
  "RuntimeAssembly",
  "ModelLibraryPreview",
  "RuntimeArchitecture",
  "FinalCTA",
];
const docsComponents = [
  "DocsCodeBlock",
  "DocsPager",
  "DocsSidebar",
  "DocsTableOfContents",
];
const docsPages = [
  "developers.js",
  "getting-started.js",
  "manage.js",
  "voice-workflows.js",
];
const docsStyles = ["layout.css", "content.css", "responsive.css"];
const landingStyles = [
  "foundation.css",
  "hero.css",
  "capabilities.css",
  "assembly.css",
  "models.css",
  "architecture.css",
  "closing.css",
  "responsive.css",
  "motion.css",
];
const required = [
  "index.html",
  "package.json",
  "vite.config.js",
  "vercel.json",
  "DEPLOYMENT.md",
  "api/v1/registry.js",
  "scripts/verify-deployment.mjs",
  "public/install.ps1",
  "public/install.sh",
  "src/main.jsx",
  "src/app/App.jsx",
  "src/app/router.js",
  "src/app/routes.js",
  "src/pages/HomePage.jsx",
  "src/pages/ModelsPage.jsx",
  "src/pages/ModelDetailPage.jsx",
  "src/pages/DocsPage.jsx",
  "src/pages/DownloadPage.jsx",
  ...landingComponents.map((name) => `src/components/landing/${name}.jsx`),
  "src/components/landing/RollingPullCommand.jsx",
  ...docsComponents.map((name) => `src/components/docs/${name}.jsx`),
  "src/docs/content.js",
  ...docsPages.map((name) => `src/docs/pages/${name}`),
  "src/models/registry.js",
  "src/models/filtering.js",
  "src/styles/index.css",
  "src/styles/docs/index.css",
  ...docsStyles.map((name) => `src/styles/docs/${name}`),
  "src/styles/landing/index.css",
  ...landingStyles.map((name) => `src/styles/landing/${name}`),
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
if (pkg.scripts["verify:deployment"] !== "node scripts/verify-deployment.mjs") {
  throw new Error("deployment verification command is missing");
}

const vite = await readFile(resolve(siteRoot, "vite.config.js"), "utf8");
if (!vite.includes("takokit-root-brand-assets")) {
  throw new Error("the site is not serving canonical assets from the repository root");
}
if (!vite.includes("VERCEL_GIT_COMMIT_SHA") || !vite.includes("raw.githubusercontent.com")) {
  throw new Error("Vercel root-isolation fallback is missing");
}

const vercel = JSON.parse(await readFile(resolve(siteRoot, "vercel.json"), "utf8"));
if (
  vercel.framework !== "vite" ||
  vercel.installCommand !== "npm ci" ||
  vercel.buildCommand !== "npm run build" ||
  vercel.outputDirectory !== "dist"
) {
  throw new Error("Vercel is not configured for the canonical Vite output");
}
if (
  vercel.cleanUrls !== true ||
  !vercel.rewrites.some((rule) => rule.destination === "/") ||
  vercel.rewrites.some((rule) => rule.destination === "/index.html")
) {
  throw new Error("clean URL SPA rewrite is missing or contradictory");
}
if (!vercel.rewrites.some((rule) => rule.source === "/v1/registry.json")) {
  throw new Error("registry rewrite is missing");
}
for (const source of ["/assets/(.*)", "/brand/(.*)", "/install.ps1", "/install.sh"]) {
  if (!vercel.headers.some((rule) => rule.source === source)) {
    throw new Error(`Vercel headers are missing ${source}`);
  }
}

const app = await readFile(resolve(siteRoot, "src/app/App.jsx"), "utf8");
for (const page of ["HomePage", "ModelsPage", "ModelDetailPage", "DocsPage", "DownloadPage"]) {
  if (!app.includes(page)) throw new Error(`app router is missing ${page}`);
}
if (!app.includes("site--landing-home")) throw new Error("landing-specific site shell is missing");

const home = await readFile(resolve(siteRoot, "src/pages/HomePage.jsx"), "utf8");
for (const component of landingComponents) {
  if (!home.includes(component)) throw new Error(`landing homepage is missing ${component}`);
}
if (home.includes("WorkflowPinwheel")) throw new Error("obsolete landing pinwheel is still referenced");
if (!home.includes("takokit-landing")) throw new Error("landing page root class is missing");

const hero = await readFile(resolve(siteRoot, "src/components/landing/LandingHero.jsx"), "utf8");
for (const requiredText of [
  "Run open voice models locally.",
  "RollingPullCommand",
  "Download for Windows",
  "Browse models",
]) {
  if (!hero.includes(requiredText)) throw new Error(`landing hero is missing: ${requiredText}`);
}
if (/WebView|Tauri|native desktop/i.test(hero)) {
  throw new Error("landing hero describes a native desktop wrapper");
}

const platform = await readFile(resolve(siteRoot, "src/lib/platform.js"), "utf8");
if (!platform.includes("tako gui") || /WebView|Tauri/i.test(platform)) {
  throw new Error("Windows platform copy does not describe the local browser GUI correctly");
}

const gettingStarted = await readFile(resolve(siteRoot, "src/docs/pages/getting-started.js"), "utf8");
for (const forbidden of ["Build from source", "git clone", "cargo build", "npm ci"]) {
  if (gettingStarted.includes(forbidden)) {
    throw new Error(`normal-user install docs still contain ${forbidden}`);
  }
}
for (const requiredText of ["v0.1.0", "install.ps1", "tako gui"]) {
  if (!gettingStarted.includes(requiredText)) {
    throw new Error(`normal-user install docs are missing ${requiredText}`);
  }
}

const rollingCommand = await readFile(
  resolve(siteRoot, "src/components/landing/RollingPullCommand.jsx"),
  "utf8",
);
if (!rollingCommand.includes("tako pull ${model}")) {
  throw new Error("rolling pull command does not keep the command prefix stable");
}
for (const model of ["kokoro", "whisper-tiny", "chatterbox", "rvc"]) {
  if (!rollingCommand.includes(model)) throw new Error(`rolling pull command is missing ${model}`);
}

const capabilities = await readFile(
  resolve(siteRoot, "src/components/landing/ProductCapabilities.jsx"),
  "utf8",
);
for (const task of ["speech", "transcription", "cloning", "conversion"]) {
  if (!capabilities.includes(`/models?task=${task}`)) {
    throw new Error(`homepage task shortcut is missing ${task}`);
  }
}

const architecture = await readFile(
  resolve(siteRoot, "src/components/landing/RuntimeArchitecture.jsx"),
  "utf8",
);
for (const step of ["Pull a model", "Run it locally", "Use any interface"]) {
  if (!architecture.includes(step)) throw new Error(`how-it-works section is missing ${step}`);
}

const docsPage = await readFile(resolve(siteRoot, "src/pages/DocsPage.jsx"), "utf8");
for (const component of docsComponents) {
  if (!docsPage.includes(component)) throw new Error(`documentation page is missing ${component}`);
}
if (!docsPage.includes("docs-shell") || !docsPage.includes("docs-content__section")) {
  throw new Error("documentation reading layout is missing");
}

const docsStyleIndex = await readFile(resolve(siteRoot, "src/styles/docs/index.css"), "utf8");
for (const stylesheet of docsStyles) {
  if (!docsStyleIndex.includes(stylesheet)) {
    throw new Error(`documentation styles are missing ${stylesheet}`);
  }
}
const docsLayout = await readFile(resolve(siteRoot, "src/styles/docs/layout.css"), "utf8");
const docsContent = await readFile(resolve(siteRoot, "src/styles/docs/content.css"), "utf8");
for (const selector of [".docs-sidebar", ".docs-toc"]) {
  if (!docsLayout.includes(selector)) throw new Error(`documentation layout is missing ${selector}`);
}
for (const selector of [".docs-content", ".docs-pager"]) {
  if (!docsContent.includes(selector)) throw new Error(`documentation content is missing ${selector}`);
}

const stylesIndex = await readFile(resolve(siteRoot, "src/styles/index.css"), "utf8");
if (!stylesIndex.includes("docs/index.css")) throw new Error("documentation stylesheet is not loaded");

const landingIndex = await readFile(resolve(siteRoot, "src/styles/landing/index.css"), "utf8");
for (const stylesheet of landingStyles) {
  if (!landingIndex.includes(stylesheet)) throw new Error(`landing styles are missing ${stylesheet}`);
}
if (landingIndex.includes("workflows.css")) throw new Error("obsolete pinwheel styles are still imported");

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
