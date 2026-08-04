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

test("hero keeps a restrained cinematic scroll treatment", async () => {
  const hero = await source("src/components/landing/LandingHero.jsx");
  assert.match(hero, /useScrollProgress/);
  assert.match(hero, /--hero-progress/);
  assert.match(hero, /PlatformInstall/);
  assert.match(hero, /Run open voice models locally/);
});

test("runtime assembly describes real Takokit layers", async () => {
  const assembly = await source("src/components/landing/RuntimeAssembly.jsx");
  for (const capability of ["Models", "Runners", "Adapters", "Interfaces", "Local state", "Consent"]) {
    assert.ok(assembly.includes(capability), `missing ${capability}`);
  }
  assert.match(assembly, /useScrollProgress/);
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
  assert.doesNotMatch(foundation, /#061a2c/i);
  assert.match(vite, /takokit-root-brand-assets/);
  assert.match(vite, /assets\/svg-transparent\/512\.svg/);
});

test("responsive landing removes pinned layouts on compact screens", async () => {
  const responsive = await source("src/styles/landing/responsive.css");
  assert.match(responsive, /max-width: 900px/);
  assert.match(responsive, /position: relative/);
  assert.match(responsive, /max-width: 700px/);
  assert.match(responsive, /max-width: 390px/);
});
