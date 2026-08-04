import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const root = new URL("../", import.meta.url);

async function source(path) {
  return readFile(new URL(path, root), "utf8");
}

test("homepage uses the cinematic Takokit journey", async () => {
  const home = await source("src/pages/HomePage.jsx");
  for (const component of [
    "CinematicHero",
    "FeatureTaco",
    "CapabilityPinwheel",
    "RecommendedModelsScene",
    "RuntimeFlow",
  ]) {
    assert.match(home, new RegExp(component));
  }
  assert.match(home, /className="takokit-cinematic"/);
});

test("feature taco describes real Takokit capabilities", async () => {
  const featureTaco = await source("src/components/landing/FeatureTaco.jsx");
  for (const capability of [
    "CURATED MODEL CATALOG",
    "MANAGED RUNNERS",
    "CLI · TUI · GUI · API",
    "LOCAL STATE",
    "CONSENT CONTROLS",
    "WINDOWS · LINUX · macOS",
  ]) {
    assert.ok(featureTaco.includes(capability), `missing ${capability}`);
  }
});

test("workflow pinwheel retains valid public CLI commands", async () => {
  const pinwheel = await source("src/components/landing/CapabilityPinwheel.jsx");
  assert.match(pinwheel, /tako speak/);
  assert.match(pinwheel, /tako transcribe/);
  assert.match(pinwheel, /tako clone/);
  assert.match(pinwheel, /--consent/);
  assert.match(pinwheel, /tako convert/);
});

test("landing styles and Dawnlight typography are loaded", async () => {
  const styles = await source("src/styles/index.css");
  const html = await source("index.html");
  assert.match(styles, /landing\/index\.css/);
  assert.match(html, /family=Orbitron/);
  assert.match(html, /family=Space\+Mono/);
});
