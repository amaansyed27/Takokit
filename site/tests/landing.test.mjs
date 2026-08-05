import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const root = new URL("../", import.meta.url);

async function source(path) {
  return readFile(new URL(path, root), "utf8");
}

test("homepage uses the simplified Takokit landing flow", async () => {
  const home = await source("src/pages/HomePage.jsx");
  for (const component of [
    "LandingHero",
    "ProductCapabilities",
    "RuntimeAssembly",
    "ModelLibraryPreview",
    "RuntimeArchitecture",
    "FinalCTA",
  ]) {
    assert.match(home, new RegExp(component));
  }
  assert.doesNotMatch(home, /WorkflowPinwheel/);
  assert.match(home, /className="takokit-landing"/);
});

test("hero rotates only the model reference while keeping the pull command stable", async () => {
  const hero = await source("src/components/landing/LandingHero.jsx");
  const command = await source("src/components/landing/RollingPullCommand.jsx");
  const styles = await source("src/styles/landing/hero.css");
  assert.doesNotMatch(hero, /useScrollProgress|PlatformInstall/);
  assert.match(hero, /landing-hero__wave/);
  assert.match(hero, /Run open voice models locally/);
  assert.match(hero, /RollingPullCommand/);
  assert.match(hero, /Download for Windows/);
  assert.match(hero, /Browse models/);
  assert.match(command, /tako pull \$\{model\}/);
  for (const model of ["kokoro", "whisper-tiny", "chatterbox", "rvc"]) {
    assert.ok(command.includes(model), `missing rotating model ${model}`);
  }
  assert.match(command, /prefers-reduced-motion: reduce/);
  assert.match(styles, /rolling-pull-command__window/);
  assert.match(styles, /hero-model-roll/);
  assert.match(styles, /min-height: 52px/);
});

test("task shortcuts use user-facing labels and shareable model filter URLs", async () => {
  const capabilities = await source("src/components/landing/ProductCapabilities.jsx");
  const styles = await source("src/styles/landing/capabilities.css");
  assert.match(capabilities, /IntersectionObserver/);
  assert.match(capabilities, /RouteLink/);
  assert.match(styles, /animation-timeline: view\(\)/);
  assert.match(styles, /capability-copy-left/);
  for (const capability of [
    "Generate speech",
    "Transcribe audio",
    "Clone a voice",
    "Convert a voice",
  ]) {
    assert.ok(capabilities.includes(capability), `missing ${capability}`);
  }
  for (const task of ["speech", "transcription", "cloning", "conversion"]) {
    assert.ok(capabilities.includes(`/models?task=${task}`), `missing ${task} task URL`);
  }
});

test("runtime story uses a simple sticky wheel with discrete text steps", async () => {
  const runtime = await source("src/components/landing/RuntimeAssembly.jsx");
  const styles = await source("src/styles/landing/assembly.css");
  for (const capability of ["Models", "Runners", "Every interface", "Local by default"]) {
    assert.ok(runtime.includes(capability), `missing ${capability}`);
  }
  assert.match(runtime, /IntersectionObserver/);
  assert.match(runtime, /useState/);
  assert.doesNotMatch(runtime, /useScrollProgress|runtime-wheel__track/);
  assert.match(runtime, /runtime-flow__rotor/);
  assert.match(runtime, /runtime-flow__steps/);
  assert.match(runtime, /Different workflows/);
  assert.match(runtime, /\/brand\/takokit-mark\.svg/);
  assert.match(styles, /position: sticky/);
  assert.match(styles, /runtime-flow__steps li\.is-active/);
  assert.doesNotMatch(styles, /animation-timeline: --runtime-wheel|runtime-text-roll/);
});

test("how Takokit works uses the required three direct steps", async () => {
  const architecture = await source("src/components/landing/RuntimeArchitecture.jsx");
  for (const step of ["Pull a model", "Run it locally", "Use any interface"]) {
    assert.ok(architecture.includes(step), `missing ${step}`);
  }
  assert.match(architecture, /CLI, GUI, TUI, and the local API/);
  assert.match(architecture, /runtime-architecture__nodes--three/);
});

test("landing uses Dawnlight fonts with the Takokit palette and canonical assets", async () => {
  const styles = await source("src/styles/index.css");
  const landingStyles = await source("src/styles/landing/index.css");
  const foundation = await source("src/styles/landing/foundation.css");
  const html = await source("index.html");
  const vite = await source("vite.config.js");
  assert.match(styles, /landing\/index\.css/);
  assert.match(landingStyles, /assembly\.css/);
  assert.doesNotMatch(landingStyles, /workflows\.css/);
  assert.match(html, /family=Orbitron/);
  assert.match(html, /family=Space\+Mono/);
  assert.match(foundation, /#ffd204/i);
  assert.match(foundation, /#eeebe3/i);
  assert.doesNotMatch(foundation, /#061a2c/i);
  assert.match(vite, /takokit-root-brand-assets/);
  assert.match(vite, /assets\/svg-transparent\/512\.svg/);
  assert.match(vite, /VERCEL_GIT_COMMIT_SHA/);
  assert.match(vite, /raw\.githubusercontent\.com/);
});

test("responsive landing keeps the scroll stories readable on compact screens", async () => {
  const responsive = await source("src/styles/landing/responsive.css");
  const assembly = await source("src/styles/landing/assembly.css");
  assert.match(responsive, /max-width: 900px/);
  assert.match(responsive, /capability-moment/);
  assert.match(assembly, /max-width: 900px/);
  assert.match(assembly, /runtime-flow__sticky/);
  assert.match(assembly, /max-width: 700px/);
  assert.match(assembly, /max-width: 390px/);
});
