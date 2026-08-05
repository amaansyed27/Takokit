import assert from "node:assert/strict";
import test from "node:test";
import {
  defaultRelease,
  formatBytes,
  normalizeModel,
  resolveModel,
  resolveRelease,
  validateRegistry,
} from "../src/models/registry.js";

const registry = {
  schema_version: 1,
  namespace: "library",
  models: [{
    name: "whisper",
    display_name: "Whisper",
    default_tag: "base",
    summary: "Local transcription",
    tasks: ["stt"],
    aliases: ["whisper-tiny"],
    tags: [
      {
        tag: "base", target: "whisper-base", aliases: ["whisper-base"],
        size_bytes: 147951465, runner: "takokit-whispercpp", adapter: null,
        backend: "whispercpp", license: "mit", hardware: { cpu: true, gpu: false, min_ram: "4gb" },
        source: {}, version: "0.1.0", digest: "sha256:base", manifest_toml: "metadata_only = false",
      },
      {
        tag: "tiny", target: "whisper-tiny", aliases: ["whisper-tiny"],
        size_bytes: 77691713, runner: "takokit-whispercpp", adapter: null,
        backend: "whispercpp", license: "mit", hardware: { cpu: true, gpu: false, min_ram: "2gb" },
        source: {}, version: "0.1.0", digest: "sha256:tiny", manifest_toml: "metadata_only = false",
      },
    ],
  }],
};

test("registry validation accepts consistent data", () => {
  assert.deepEqual(validateRegistry(registry), []);
});

test("registry validation rejects contradictions", () => {
  const invalid = structuredClone(registry);
  invalid.models[0].tags[0].hardware = { cpu: false, gpu: false };
  assert.ok(validateRegistry(invalid).some((error) => error.includes("neither CPU nor GPU")));
});

test("unknown size is truthful", () => {
  assert.equal(formatBytes(0), "Not declared");
  assert.equal(formatBytes(null), "Not declared");
  assert.equal(formatBytes(77_691_713), "78 MB");
});

test("default and aliases resolve to one identity", () => {
  const normalized = { ...registry, models: registry.models.map(normalizeModel) };
  const model = resolveModel(normalized, "whisper-tiny");
  assert.equal(model.name, "whisper");
  assert.equal(defaultRelease(model).tag, "base");
  assert.equal(resolveRelease(model, "tiny").target, "whisper-tiny");
  assert.equal(resolveModel(normalized, "unknown"), null);
});

test("recommended variant remains the declared default", () => {
  const model = normalizeModel(registry.models[0]);
  assert.equal(model.release.tag, "base");
});
