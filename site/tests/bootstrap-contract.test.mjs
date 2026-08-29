import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";

const scriptUrl = new URL("../public/install.ps1", import.meta.url);

test("PowerShell bootstrap uses release metadata and the canonical installer", async () => {
  const script = await readFile(scriptUrl, "utf8");
  assert.match(script, /v1\/releases\/stable\/windows-x86_64\.json/);
  assert.match(script, /System\.Security\.Cryptography\.SHA256/);
  assert.match(script, /ComputeHash/);
  assert.match(script, /SHA-256|sha256/i);
  assert.match(script, /installer\.sha256/);
  assert.match(script, /checksum mismatch/);
  assert.match(script, /\/VERYSILENT/);
  assert.match(script, /Confirm-InstalledTakokit/);
  assert.match(script, /tako gui/);
});

test("PowerShell bootstrap contains no source-build or CI artifact dependency", async () => {
  const script = await readFile(scriptUrl, "utf8");
  for (const forbidden of [
    "git clone",
    "cargo build",
    "npm ci",
    "npm run build",
    "actions/runs",
    "RUNNER_TEMP",
    "$Tako",
  ]) {
    assert.ok(!script.includes(forbidden), `bootstrap must not contain ${forbidden}`);
  }
  assert.ok(!/Takokit-v0\.0\.1-windows-x86_64-installer/.test(script));
  assert.ok(!/Takokit-v0\.1\.0-windows-x86_64-installer/.test(script));
});

test("PowerShell bootstrap fails closed around stable/test trust", async () => {
  const script = await readFile(scriptUrl, "utf8");
  assert.match(script, /stable release metadata points to a test fixture/);
  assert.match(script, /production signing identity/);
  assert.match(script, /must use HTTPS/);
  assert.match(script, /checksum mismatch/);
  assert.match(script, /AllowInsecureLoopbackForTesting/);
});
