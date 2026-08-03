import assert from "node:assert/strict";
import test from "node:test";
import { matchRoute } from "../src/app/routes.js";

test("known routes resolve", () => {
  assert.equal(matchRoute("/").name, "home");
  assert.equal(matchRoute("/models").name, "models");
  assert.equal(matchRoute("/docs/install").params.slug, "install");
  assert.equal(matchRoute("/download").name, "download");
});

test("legacy library routes remain compatible", () => {
  assert.equal(matchRoute("/library").name, "models");
  assert.deepEqual(matchRoute("/library/whisper:tiny").params, {
    model: "whisper",
    tag: "tiny",
  });
  assert.deepEqual(matchRoute("/library/kokoro").params, {
    model: "kokoro",
    tag: undefined,
  });
});

test("new model tag route resolves", () => {
  assert.deepEqual(matchRoute("/models/qwen3-tts/0.6b-base").params, {
    model: "qwen3-tts",
    tag: "0.6b-base",
  });
});

test("unknown route returns not found", () => {
  assert.equal(matchRoute("/missing/page").name, "not-found");
});
