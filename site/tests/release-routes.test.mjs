import assert from "node:assert/strict";
import test from "node:test";
import {
  projectStableWindowsRelease,
  resolveStableWindowsRelease,
  StableReleaseUnavailableError,
} from "../api/_release.js";
import metadataHandler from "../api/v1/releases/stable/windows-x86_64.js";
import downloadHandler from "../api/download/windows.js";

const installerHash = "a".repeat(64);

function stableManifest(overrides = {}) {
  return {
    product: "Takokit",
    version: "0.1.0",
    channel: "stable",
    commit_sha: "abc123",
    os: "windows",
    architecture: "x86_64",
    signing_key_id: "takokit-release-v1",
    test_fixture: false,
    artifacts: [{
      role: "installer",
      name: "Takokit-v0.1.0-windows-x86_64-installer.exe",
      sha256: installerHash,
      size: 1234,
      url: "https://downloads.example.test/Takokit-v0.1.0-windows-x86_64-installer.exe",
    }],
    ...overrides,
  };
}

function responseForJson(value, status = 200) {
  return {
    ok: status >= 200 && status < 300,
    status,
    async json() { return value; },
  };
}

function mockVercelResponse() {
  return {
    statusCode: null,
    headers: new Map(),
    body: null,
    redirectStatus: null,
    redirectUrl: null,
    setHeader(name, value) { this.headers.set(name.toLowerCase(), value); },
    status(code) { this.statusCode = code; return this; },
    send(value) { this.body = value; return this; },
    json(value) { this.body = value; return this; },
    redirect(code, url) { this.redirectStatus = code; this.redirectUrl = url; return this; },
  };
}

test("stable Windows release projection selects the canonical installer", () => {
  const projected = projectStableWindowsRelease(stableManifest());
  assert.equal(projected.schema_version, 1);
  assert.equal(projected.channel, "stable");
  assert.equal(projected.platform, "windows");
  assert.equal(projected.architecture, "x86_64");
  assert.equal(projected.test_fixture, false);
  assert.equal(projected.installer.sha256, installerHash);
});

test("stable projection rejects test fixture signing identity", () => {
  assert.throws(
    () => projectStableWindowsRelease(stableManifest({
      channel: "test",
      test_fixture: true,
      signing_key_id: "takokit-test-fixture-v1",
    })),
    StableReleaseUnavailableError,
  );
});

test("stable projection rejects malformed or missing installer metadata", () => {
  assert.throws(
    () => projectStableWindowsRelease(stableManifest({ artifacts: [] })),
    /installer metadata is missing/,
  );
  assert.throws(
    () => projectStableWindowsRelease(stableManifest({
      artifacts: [{ role: "installer", name: "bad.exe", sha256: "bad" }],
    })),
    /installer name is invalid/,
  );
});

test("resolver reports upstream failure without manufacturing a stable build", async () => {
  await assert.rejects(
    resolveStableWindowsRelease({
      manifestUrl: "https://example.test/release-manifest.json",
      fetchImpl: async () => responseForJson({}, 404),
    }),
    /returned 404/,
  );
});

test("metadata route exposes projected stable metadata", async () => {
  const originalFetch = globalThis.fetch;
  globalThis.fetch = async () => responseForJson(stableManifest());
  try {
    const response = mockVercelResponse();
    await metadataHandler({}, response);
    assert.equal(response.statusCode, 200);
    const body = JSON.parse(response.body);
    assert.equal(body.installer.sha256, installerHash);
    assert.equal(body.test_fixture, false);
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test("metadata and download routes fail closed for test fixtures", async () => {
  const originalFetch = globalThis.fetch;
  globalThis.fetch = async () => responseForJson(stableManifest({
    channel: "test",
    test_fixture: true,
    signing_key_id: "takokit-test-fixture-v1",
  }));
  try {
    const metadataResponse = mockVercelResponse();
    await metadataHandler({}, metadataResponse);
    assert.equal(metadataResponse.statusCode, 503);
    assert.equal(metadataResponse.body.error, "stable_release_unavailable");

    const downloadResponse = mockVercelResponse();
    await downloadHandler({}, downloadResponse);
    assert.equal(downloadResponse.statusCode, 503);
    assert.equal(downloadResponse.redirectUrl, null);
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test("Windows direct download redirects only to the projected stable installer", async () => {
  const originalFetch = globalThis.fetch;
  globalThis.fetch = async () => responseForJson(stableManifest());
  try {
    const response = mockVercelResponse();
    await downloadHandler({}, response);
    assert.equal(response.redirectStatus, 307);
    assert.equal(
      response.redirectUrl,
      "https://downloads.example.test/Takokit-v0.1.0-windows-x86_64-installer.exe",
    );
  } finally {
    globalThis.fetch = originalFetch;
  }
});
