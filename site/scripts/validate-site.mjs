import { access, readFile, readdir } from "node:fs/promises";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { DOC_ORDER, DOCS } from "../src/docs/content.js";
import { PLATFORM_DETAILS, installCommand } from "../src/lib/platform.js";
import { validateRegistry } from "../src/models/registry.js";

const siteRoot = fileURLToPath(new URL("../", import.meta.url));
const repoRoot = resolve(siteRoot, "..");
const readSite = (path) => readFile(resolve(siteRoot, path), "utf8");
const readRepo = (path) => readFile(resolve(repoRoot, path), "utf8");

const landingComponents = [
  "LandingHero",
  "ProductCapabilities",
  "RuntimeAssembly",
  "ModelLibraryPreview",
  "RuntimeArchitecture",
  "FinalCTA",
];
const docsComponents = ["DocsCodeBlock", "DocsPager", "DocsSidebar", "DocsTableOfContents"];
const docsPages = [
  "developers.js",
  "getting-started.js",
  "interfaces.js",
  "manage.js",
  "release.js",
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

const pkg = JSON.parse(await readSite("package.json"));
if (pkg.scripts.build !== "vite build") throw new Error("canonical site build is not Vite");
if (pkg.scripts["verify:deployment"] !== "node scripts/verify-deployment.mjs") {
  throw new Error("deployment verification command is missing");
}

const vite = await readSite("vite.config.js");
if (!vite.includes("takokit-root-brand-assets")) throw new Error("canonical root brand assets are not served");
if (!vite.includes("VERCEL_GIT_COMMIT_SHA") || !vite.includes("raw.githubusercontent.com")) {
  throw new Error("Vercel root-isolation fallback is missing");
}

const vercel = JSON.parse(await readSite("vercel.json"));
if (vercel.framework !== "vite" || vercel.installCommand !== "npm ci" || vercel.buildCommand !== "npm run build" || vercel.outputDirectory !== "dist") {
  throw new Error("Vercel is not configured for canonical Vite output");
}
if (vercel.cleanUrls !== true || !vercel.rewrites.some((rule) => rule.destination === "/")) {
  throw new Error("clean SPA routes are not configured");
}
if (!vercel.rewrites.some((rule) => rule.source === "/v1/registry.json")) throw new Error("registry rewrite is missing");
for (const source of ["/assets/(.*)", "/brand/(.*)", "/install.ps1", "/install.sh"]) {
  if (!vercel.headers.some((rule) => rule.source === source)) throw new Error(`Vercel headers are missing ${source}`);
}

const app = await readSite("src/app/App.jsx");
for (const page of ["HomePage", "ModelsPage", "ModelDetailPage", "DocsPage", "DownloadPage"]) {
  if (!app.includes(page)) throw new Error(`app router is missing ${page}`);
}

const hero = await readSite("src/components/landing/LandingHero.jsx");
for (const requiredText of ["Run open voice models locally.", "RollingPullCommand", "Install Takokit", "Browse models", "Read docs", "public beta"]) {
  if (!hero.includes(requiredText)) throw new Error(`landing hero is missing: ${requiredText}`);
}
if (/One Windows runtime|Download for Windows|WebView|Tauri|Electron/i.test(hero)) {
  throw new Error("landing hero contains stale single-platform/native-wrapper copy");
}

for (const platform of ["windows", "linux", "macos"]) {
  if (PLATFORM_DETAILS[platform]?.available !== true) throw new Error(`${platform} is not enabled in public platform selector`);
  if (!installCommand(platform)) throw new Error(`${platform} install command is missing`);
}
if (!installCommand("windows").includes("install.ps1") || !installCommand("linux").includes("install.sh") || !installCommand("macos").includes("install.sh")) {
  throw new Error("cross-platform public bootstrap commands drifted");
}

const gettingStarted = await readSite("src/docs/pages/getting-started.js");
for (const requiredText of ["v0.3.0", "install.ps1", "install.sh", "tako gui", "Linux x86_64", "Apple Silicon"]) {
  if (!gettingStarted.includes(requiredText)) throw new Error(`getting-started docs are missing ${requiredText}`);
}
for (const forbidden of ["v0.1.0", "Linux and macOS packages are coming later", "git clone", "cargo build", "apps/desktop"]) {
  if (gettingStarted.includes(forbidden)) throw new Error(`normal-user docs contain stale/developer-only text: ${forbidden}`);
}

const releaseDocs = await readSite("src/docs/pages/release.js");
for (const requiredText of ["every 0.x.x release is Beta", "1.0.0", "Windows 10/11 x86_64", "Linux x86_64", "macOS 12+ arm64", "takokit-release-v1"]) {
  if (!releaseDocs.includes(requiredText)) throw new Error(`release policy docs are missing ${requiredText}`);
}

const docIds = DOC_ORDER.map(({ id }) => id);
if (new Set(docIds).size !== docIds.length) throw new Error("duplicate documentation slug");
for (const id of docIds) {
  const doc = DOCS[id];
  if (!doc?.title || !doc?.intro || !Array.isArray(doc.sections) || doc.sections.length === 0) {
    throw new Error(`documentation page ${id} is incomplete`);
  }
  const sectionIds = doc.sections.map(({ id: sectionId }) => sectionId);
  if (new Set(sectionIds).size !== sectionIds.length) throw new Error(`duplicate section id in ${id}`);
}

const docsPage = await readSite("src/pages/DocsPage.jsx");
for (const component of docsComponents) {
  if (!docsPage.includes(component)) throw new Error(`documentation page is missing ${component}`);
}
for (const capability of ["section.items", "section.warning", "docs-list"]) {
  if (!docsPage.includes(capability)) throw new Error(`documentation renderer is missing ${capability}`);
}

const developerDocs = await readSite("src/docs/pages/developers.js");
for (const requiredText of ["OpenAI-compatible audio", "Chat completions", "Not supported", "/openapi.json", "/api/v1", "takokit.dawnlightlabs.com/v1/registry.json"]) {
  if (!developerDocs.includes(requiredText)) throw new Error(`developer docs are missing ${requiredText}`);
}
if (/general OpenAI API compatibility is supported|full OpenAI compatibility/i.test(developerDocs)) {
  throw new Error("developer docs overclaim OpenAI compatibility");
}

const voiceDocs = await readSite("src/docs/pages/voice-workflows.js");
for (const requiredText of ["Advanced RVC studio", "tako voice rvc train", "tako voice rvc export", "--f0-method rmvpe"]) {
  if (!voiceDocs.includes(requiredText)) throw new Error(`voice docs are missing ${requiredText}`);
}
if (voiceDocs.includes("planned separately under Issue #68")) throw new Error("voice docs still claim Advanced RVC is future work");

const cliArgs = await readRepo("apps/cli/src/args.rs");
const interfaceDocs = await readSite("src/docs/pages/interfaces.js");
const commandPairs = [
  ["Start", "tako start"], ["Stop", "tako stop"], ["Serve", "tako serve"], ["Server", "tako server"],
  ["Gui", "tako gui"], ["Doctor", "tako doctor"], ["Version", "tako version"], ["Status", "tako status"],
  ["Storage", "tako storage"], ["Update", "tako update"], ["Reset", "tako reset"], ["Capabilities", "tako capabilities"],
  ["Models", "tako models"], ["Runners", "tako runners"], ["Speak", "tako speak"], ["Pull", "tako pull"],
  ["Show", "tako show"], ["Plan", "tako plan"], ["Rm", "tako rm"], ["Run", "tako run"],
  ["Runner", "tako runner"], ["Adapter", "tako adapter"], ["Sessions", "tako sessions"],
  ["Transcribe", "tako transcribe"], ["Clone", "tako clone"], ["Convert", "tako convert"], ["Train", "tako train"],
];
for (const [variant, command] of commandPairs) {
  if (!cliArgs.includes(`${variant}`)) throw new Error(`CLI source no longer contains expected variant ${variant}`);
  if (!interfaceDocs.includes(command)) throw new Error(`CLI docs are missing ${command}`);
}

const apiInventory = await readRepo("docs/api-route-inventory.md");
for (const route of ["GET /v1/models", "POST /v1/audio/speech", "POST /v1/audio/transcriptions", "/api/v1"]) {
  if (!apiInventory.includes(route)) throw new Error(`API inventory is missing ${route}`);
}
if (!apiInventory.includes("audio-only subset")) throw new Error("API inventory lost audio-only compatibility boundary");

const stylesIndex = await readSite("src/styles/index.css");
if (!stylesIndex.includes("docs/index.css")) throw new Error("documentation stylesheet is not loaded");
const docsStyleIndex = await readSite("src/styles/docs/index.css");
for (const stylesheet of docsStyles) if (!docsStyleIndex.includes(stylesheet)) throw new Error(`documentation styles are missing ${stylesheet}`);
const landingIndex = await readSite("src/styles/landing/index.css");
for (const stylesheet of landingStyles) if (!landingIndex.includes(stylesheet)) throw new Error(`landing styles are missing ${stylesheet}`);

const sourceEntries = await readdir(resolve(siteRoot, "src"), { recursive: true });
if (sourceEntries.some((path) => String(path).includes("assets/base.css"))) throw new Error("obsolete static-site source remains");

const registryPath = process.env.TAKOKIT_REGISTRY_PATH || resolve(repoRoot, "registry/index.json");
const registry = JSON.parse(await readFile(registryPath, "utf8"));
const errors = validateRegistry(registry);
if (errors.length) throw new Error(`registry validation failed:\n${errors.join("\n")}`);
const families = registry.models.length;
const releases = registry.models.reduce((total, model) => total + model.tags.length, 0);
if (families < 1 || releases < families) throw new Error("registry catalog is unexpectedly empty");

console.log(`Takokit site validation passed: ${families} families / ${releases} releases / ${docIds.length} docs pages`);
