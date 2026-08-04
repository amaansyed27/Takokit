import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const root = new URL("../", import.meta.url);

async function source(path) {
  return readFile(new URL(path, root), "utf8");
}

test("homepage uses the rebuilt Takokit landing journey", async () => {
  const home = await source("src/pages/HomePage.jsx");
  for (const component of [
    "LandingHero",
    "ProductCapabilities",
    "RuntimeAssembly",
    "WorkflowPinwheel",
    "ModelLibraryPreview",
    "RuntimeArchitecture",
    "FinalCTA",
  ]) {
    assert.match(home, new RegExp(component));
  }
  assert.match(home, /className="takokit-landing"/);
});

test("runtime assembly describes real Takokit layers", async () => {
  const assembly = await source("src/components/landing/RuntimeAssembly.jsx");
  for (const capability of [
    "Models",
    "Runners",
    "Adapters",
    "Interfaces",
    "Local state",
    "Consent",
  ]) {
    assert.ok(assembly.includes(capability), `missing ${capability}`);
  }
  assert.match(assembly, /\/brand\/takokit-mark\.svg/);
});

test("workflow pinwheel stays discrete and retains valid commands", async () => {
  const pinwheel = await source("src/components/landing/WorkflowPinwheel.jsx");
  assert.match(pinwheel, /activeIndex \* -90/);
  assert.match(pinwheel, /tako speak/);
  assert.match(pinwheel, /tako transcribe/);
  assert.match(pinwheel, /tako clone/);
  assert.match(pinwheel, /--consent/);
  assert.match(pinwheel, /tako convert/);
});

test("landing uses Dawnlight fonts with the Takokit asset palette", async () => {
  const styles = await source("src/styles/index.css");
  const landingStyles = await source("src/styles/landing/index.css");
  const foundation = await source("src/styles/landing/foundation.css");
  const html = await source("index.html");
  const vite = await source("vite.config.js");
  assert.match(styles, /landing\/index\.css/);
  assert.match(landingStyles, /assembly\.css/);
  assert.match(landingStyles, /workflows\.css/);
  assert.match(html, /family=Orbitron/);
  assert.match(html, /family=Space\+Mono/);
  assert.match(foundation, /#ffd204/i);
  assert.doesNotMatch(foundation, /#061a2c/i);
  assert.match(vite, /takokit-root-brand-assets/);
  assert.match(vite, /assets\/svg-transparent\/512\.svg/);
});

test("responsive landing removes pinned layouts on compact screens", async () => {
  const responsive = await source("src/styles/landing/responsive.css");
  assert.match(responsive, /max-width: 960px/);
  assert.match(responsive, /position: relative/);
  assert.match(responsive, /max-width: 700px/);
  assert.match(responsive, /max-width: 390px/);
});
