import test from "node:test";
import assert from "node:assert/strict";
import {
  detectPlatform,
  installCommand,
  PLATFORM_DETAILS,
  PUBLIC_SITE_ORIGIN,
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

test("Windows command is the stable Dawnlight PowerShell bootstrap", () => {
  assert.equal(PUBLIC_SITE_ORIGIN, "https://takokit.dawnlightlabs.com");
  assert.equal(
    installCommand("windows"),
    "irm https://takokit.dawnlightlabs.com/install.ps1 | iex",
  );
});

test("unpublished platforms do not expose fake install commands", () => {
  assert.equal(PLATFORM_DETAILS.windows.available, true);
  assert.equal(PLATFORM_DETAILS.linux.available, false);
  assert.equal(PLATFORM_DETAILS.macos.available, false);
  assert.equal(installCommand("linux"), null);
  assert.equal(installCommand("macos"), null);
});
