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

test("capability section reveals workflows on viewport entry", async () => {
  const capabilities = await source("src/components/landing/ProductCapabilities.jsx");
  assert.match(capabilities, /IntersectionObserver/);
  assert.match(capabilities, /is-visible/);
  for (const capability of ["Speak", "Transcribe", "Clone", "Convert"]) {
    assert.ok(capabilities.includes(capability), `missing ${capability}`);
  }
});

test("runtime assembly describes real Takokit layers without per-frame scroll state", async () => {
  const assembly = await source("src/components/landing/RuntimeAssembly.jsx");
  for (const capability of ["Models", "Runners", "Adapters", "Interfaces", "Local state", "Consent"]) {
    assert.ok(assembly.includes(capability), `missing ${capability}`);
  }
  assert.match(assembly, /IntersectionObserver/);
  assert.doesNotMatch(assembly, /useScrollProgress/);
  assert.match(assembly, /runtime-assembly__ingredient/);
  assert.match(assembly, /\/brand\/takokit-mark\.svg/);
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

test("responsive landing adapts the sticky story for compact screens", async () => {
  const responsive = await source("src/styles/landing/responsive.css");
  assert.match(responsive, /max-width: 900px/);
  assert.match(responsive, /runtime-assembly__intro/);
  assert.match(responsive, /position: relative/);
  assert.match(responsive, /max-width: 700px/);
  assert.match(responsive, /max-width: 390px/);
});
