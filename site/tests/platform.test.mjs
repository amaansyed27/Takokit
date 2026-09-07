import test from "node:test";
import assert from "node:assert/strict";
import {
  detectPlatform,
  installCommand,
  PLATFORM_DETAILS,
  PUBLIC_SITE_ORIGIN,
  releaseEndpoint,
} from "../src/lib/platform.js";

test("detectPlatform identifies Windows", () => {
  assert.equal(detectPlatform({ platform: "Win32" }), "windows");
});

test("detectPlatform identifies macOS", () => {
  assert.equal(detectPlatform({ userAgentData: { platform: "macOS" } }), "macos");
});

test("detectPlatform identifies Linux", () => {
  assert.equal(detectPlatform({ userAgent: "Mozilla/5.0 (X11; Linux x86_64)" }), "linux");
});

test("all v0.3.0 public target platforms expose canonical bootstrap commands", () => {
  assert.equal(PUBLIC_SITE_ORIGIN, "https://takokit.dawnlightlabs.com");
  assert.equal(installCommand("windows"), "irm https://takokit.dawnlightlabs.com/install.ps1 | iex");
  assert.equal(installCommand("linux"), "curl -fsSL https://takokit.dawnlightlabs.com/install.sh | sh");
  assert.equal(installCommand("macos"), "curl -fsSL https://takokit.dawnlightlabs.com/install.sh | sh");

  assert.equal(PLATFORM_DETAILS.windows.available, true);
  assert.equal(PLATFORM_DETAILS.linux.available, true);
  assert.equal(PLATFORM_DETAILS.macos.available, true);
  assert.equal(PLATFORM_DETAILS.windows.architecture, "x86_64");
  assert.equal(PLATFORM_DETAILS.linux.architecture, "x86_64");
  assert.equal(PLATFORM_DETAILS.macos.architecture, "arm64");
});

test("stable release endpoints match each published target", () => {
  assert.equal(releaseEndpoint("windows"), "/v1/releases/stable/windows-x86_64.json");
  assert.equal(releaseEndpoint("linux"), "/v1/releases/stable/linux-x86_64.json");
  assert.equal(releaseEndpoint("macos"), "/v1/releases/stable/macos-arm64.json");
  assert.equal(releaseEndpoint("unknown"), null);
});
