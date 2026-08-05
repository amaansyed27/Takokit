import assert from "node:assert/strict";
import test from "node:test";
import {
  filterModels,
  filtersFromSearch,
  searchFromFilters,
} from "../src/models/filtering.js";

const models = [
  {
    name: "kokoro",
    display_name: "Kokoro",
    summary: "Speech",
    shortSummary: "Fast speech",
    primaryTask: "speech",
    status: "executable",
    aliases: [],
    tasks: ["tts"],
    languages: ["English"],
    platforms: [],
    recommended: true,
    sizeBytes: 120_000_000,
    release: { runner: "takokit-onnx", adapter: null, backend: "onnx", license: "apache-2.0" },
    hardware: { cpu: true, gpu: false, gpuRequired: false, minVramMb: null },
  },
  {
    name: "whisper",
    display_name: "Whisper Tiny",
    summary: "Transcription",
    shortSummary: "Small transcription",
    primaryTask: "transcription",
    status: "verified",
    aliases: ["whisper-tiny"],
    tasks: ["stt"],
    languages: ["Multilingual"],
    platforms: ["Windows", "Linux"],
    recommended: true,
    sizeBytes: 77_000_000,
    release: { runner: "takokit-whispercpp", adapter: null, backend: "whispercpp", license: "mit" },
    hardware: { cpu: true, gpu: false, gpuRequired: false, minVramMb: null },
  },
  {
    name: "fish-speech",
    display_name: "Fish Speech",
    summary: "GPU cloning",
    shortSummary: "GPU cloning",
    primaryTask: "cloning",
    status: "executable",
    aliases: [],
    tasks: ["tts", "voice-cloning"],
    languages: [],
    platforms: [],
    recommended: false,
    sizeBytes: null,
    release: { runner: "takokit-python-managed", adapter: "fish", backend: "python-managed", license: "research-license" },
    hardware: { cpu: false, gpu: true, gpuRequired: true, minVramMb: 12 * 1024 },
  },
];

const base = {
  query: "", task: "all", cpuFriendly: false, gpuSupported: false,
  gpuRequired: false, maxVram: "", maxSize: "", status: "",
  commercial: "", platform: "", runner: "", sort: "recommended",
};

test("search matches aliases, tasks, languages, and runner", () => {
  assert.deepEqual(filterModels(models, { ...base, query: "whisper-tiny" }).map((m) => m.name), ["whisper"]);
  assert.deepEqual(filterModels(models, { ...base, query: "Multilingual" }).map((m) => m.name), ["whisper"]);
  assert.deepEqual(filterModels(models, { ...base, query: "python-managed" }).map((m) => m.name), ["fish-speech"]);
});

test("primary and advanced filters work", () => {
  assert.deepEqual(filterModels(models, { ...base, task: "transcription" }).map((m) => m.name), ["whisper"]);
  assert.deepEqual(filterModels(models, { ...base, gpuRequired: true }).map((m) => m.name), ["fish-speech"]);
  assert.deepEqual(filterModels(models, { ...base, maxVram: "8" }).map((m) => m.name), []);
  assert.deepEqual(filterModels(models, { ...base, platform: "Windows" }).map((m) => m.name), ["whisper"]);
});

test("sorting handles unknown size last", () => {
  assert.deepEqual(filterModels(models, { ...base, sort: "smallest" }).map((m) => m.name), [
    "whisper", "kokoro", "fish-speech",
  ]);
});

test("URL state round-trips", () => {
  const filters = { ...base, query: "voice", task: "cloning", cpuFriendly: true, sort: "name" };
  const query = searchFromFilters(filters);
  const parsed = filtersFromSearch(new URLSearchParams(query));
  assert.equal(parsed.query, "voice");
  assert.equal(parsed.task, "cloning");
  assert.equal(parsed.cpuFriendly, true);
  assert.equal(parsed.sort, "name");
});
