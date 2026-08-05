import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";

const root = new URL("../", import.meta.url);

test("Vercel builds the canonical Vite source", async () => {
  const config = JSON.parse(await readFile(new URL("vercel.json", root), "utf8"));
  const pkg = JSON.parse(await readFile(new URL("package.json", root), "utf8"));
  assert.equal(config.framework, "vite");
  assert.equal(config.outputDirectory, "dist");
  assert.equal(config.buildCommand, "npm run build");
  assert.equal(pkg.scripts.build, "vite build");
  assert.ok(!pkg.scripts.build.includes("scripts/build.mjs"));
});

test("SPA and registry rewrites are present", async () => {
  const config = JSON.parse(await readFile(new URL("vercel.json", root), "utf8"));
  assert.ok(config.rewrites.some((rule) => rule.source === "/v1/registry.json"));
  assert.ok(config.rewrites.some((rule) => rule.destination === "/index.html"));
  assert.ok(config.headers.some((rule) => rule.source === "/assets/(.*)"));
});
