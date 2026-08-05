import test from "node:test";
import assert from "node:assert/strict";
import { detectPlatform, installCommand } from "../src/lib/platform.js";

test("detectPlatform identifies Windows", () => {
  assert.equal(detectPlatform({ platform: "Win32" }), "windows");
});

test("detectPlatform identifies macOS", () => {
  assert.equal(detectPlatform({ userAgentData: { platform: "macOS" } }), "macos");
});

test("detectPlatform identifies Linux", () => {
  assert.equal(detectPlatform({ userAgent: "Mozilla/5.0 (X11; Linux x86_64)" }), "linux");
});

test("installCommand uses PowerShell on Windows and curl elsewhere", () => {
  assert.equal(
    installCommand("windows", "https://example.com/"),
    "irm https://example.com/install.ps1 | iex",
  );
  assert.equal(
    installCommand("linux", "https://example.com/"),
    "curl -fsSL https://example.com/install.sh | sh",
  );
});
