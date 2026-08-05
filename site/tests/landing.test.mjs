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

test("hero keeps motion lightweight and preserves installation", async () => {
  const hero = await source("src/components/landing/LandingHero.jsx");
  assert.doesNotMatch(hero, /useScrollProgress/);
  assert.match(hero, /landing-hero__wave/);
  assert.match(hero, /PlatformInstall/);
  assert.match(hero, /Run open voice models locally/);
});

test("capabilities use large typography with a lightweight reveal fallback", async () => {
  const capabilities = await source("src/components/landing/ProductCapabilities.jsx");
  const styles = await source("src/styles/landing/capabilities.css");
  assert.match(capabilities, /IntersectionObserver/);
  assert.match(capabilities, /capability-moment/);
  assert.match(styles, /animation-timeline: view\(\)/);
  assert.match(styles, /capability-copy-left/);
  for (const capability of ["Speak", "Transcribe", "Clone", "Convert"]) {
    assert.ok(capabilities.includes(capability), `missing ${capability}`);
  }
});

test("runtime story uses direct feature bands without scroll-driven React state", async () => {
  const runtime = await source("src/components/landing/RuntimeAssembly.jsx");
  const styles = await source("src/styles/landing/assembly.css");
  for (const capability of ["Models", "Runners", "Every interface", "Local by default"]) {
    assert.ok(runtime.includes(capability), `missing ${capability}`);
  }
  assert.doesNotMatch(runtime, /useScrollProgress|IntersectionObserver|useState/);
  assert.match(runtime, /runtime-band/);
  assert.match(runtime, /Different workflows/);
  assert.match(runtime, /\/brand\/takokit-mark\.svg/);
  assert.match(styles, /animation-timeline: view\(\)/);
  assert.match(styles, /runtime-copy-left/);
});

test("landing uses Dawnlight fonts with the Takokit palette", async () => {
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
});

test("responsive landing keeps the parallax stories readable on compact screens", async () => {
  const responsive = await source("src/styles/landing/responsive.css");
  assert.match(responsive, /max-width: 900px/);
  assert.match(responsive, /capability-moment/);
  assert.match(responsive, /runtime-band/);
  assert.match(responsive, /max-width: 700px/);
  assert.match(responsive, /max-width: 390px/);
});
