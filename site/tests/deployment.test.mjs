import assert from "node:assert/strict";
import test from "node:test";
import { access, readFile } from "node:fs/promises";

const root = new URL("../", import.meta.url);

test("Vercel builds the canonical Vite source", async () => {
  const config = JSON.parse(await readFile(new URL("vercel.json", root), "utf8"));
  const pkg = JSON.parse(await readFile(new URL("package.json", root), "utf8"));
  assert.equal(config.framework, "vite");
  assert.equal(config.installCommand, "npm ci");
  assert.equal(config.outputDirectory, "dist");
  assert.equal(config.buildCommand, "npm run build");
  assert.match(config.devCommand, /npm run dev/);
  assert.equal(pkg.scripts.build, "vite build");
  assert.equal(pkg.scripts["verify:deployment"], "node scripts/verify-deployment.mjs");
  assert.ok(!pkg.scripts.build.includes("scripts/build.mjs"));
  await access(new URL("scripts/verify-deployment.mjs", root));
});

test("SPA, stable release, direct download, and raw installer routes are explicit", async () => {
  const config = JSON.parse(await readFile(new URL("vercel.json", root), "utf8"));
  assert.equal(config.cleanUrls, true);
  assert.ok(config.rewrites.some((rule) => rule.source === "/v1/registry.json"));
  assert.ok(config.rewrites.some((rule) => rule.source === "/v1/releases/stable/windows-x86_64.json"));
  assert.ok(config.rewrites.some((rule) => rule.source === "/download/windows"));
  const spa = config.rewrites.find((rule) => rule.destination === "/");
  assert.ok(spa);
  assert.match(spa.source, /install\\\.ps1/);
  assert.match(spa.source, /download\/windows/);
  assert.ok(!config.rewrites.some((rule) => rule.destination === "/index.html"));

  const installHeaders = config.headers.find((rule) => rule.source === "/install.ps1");
  assert.ok(installHeaders);
  assert.ok(installHeaders.headers.some((header) =>
    header.key === "Content-Type" && header.value === "text/plain; charset=utf-8"));
});

test("registry API follows the deployed Git revision and exposes diagnostics", async () => {
  const api = await readFile(new URL("api/v1/registry.js", root), "utf8");
  assert.match(api, /VERCEL_GIT_COMMIT_SHA/);
  assert.match(api, /TAKOKIT_REPOSITORY_REF/);
  assert.match(api, /X-Takokit-Registry-Ref/);
  assert.match(api, /stale-while-revalidate/);
});

test("root assets use separate Vite build and serve plugins", async () => {
  const vite = await readFile(new URL("vite.config.js", root), "utf8");
  assert.match(vite, /canonicalAssetsBuildPlugin/);
  assert.match(vite, /canonicalAssetsServePlugin/);
  assert.match(vite, /apply: "build"/);
  assert.match(vite, /apply: "serve"/);
  assert.match(vite, /this\.emitFile/);
});
