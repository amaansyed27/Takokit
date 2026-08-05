import {
  canonicalRef,
  isRecommended,
  knownLimitations,
  languages,
  primaryTask,
  shortSummary,
  supportedPlatforms,
  taskLabels,
  verificationStatus,
} from "./presentation.js";

let cache;

export function formatBytes(bytes) {
  if (!Number.isFinite(bytes) || bytes <= 0) return "Not declared";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1000 && unit < units.length - 1) {
    value /= 1000;
    unit += 1;
  }
  const digits = value >= 10 || unit === 0 ? 0 : 1;
  return `${value.toFixed(digits)} ${units[unit]}`;
}

export function parseMemory(value) {
  if (!value || typeof value !== "string") return null;
  const match = value.toLowerCase().match(/^([\d.]+)\s*(gb|mb)$/);
  if (!match) return null;
  const amount = Number(match[1]);
  return match[2] === "gb" ? amount * 1024 : amount;
}

export function defaultRelease(model) {
  return model.tags.find((release) => release.tag === model.default_tag) ||
    model.tags[0] ||
    null;
}

export function resolveModel(registry, name) {
  const normalized = decodeURIComponent(name || "").toLowerCase();
  return registry.models.find((model) =>
    model.name.toLowerCase() === normalized ||
    model.aliases.some((alias) => alias.toLowerCase() === normalized) ||
    model.tags.some((release) =>
      release.target?.toLowerCase() === normalized ||
      release.aliases?.some((alias) => alias.toLowerCase() === normalized)
    )
  ) || null;
}

export function resolveRelease(model, tag) {
  if (!model) return null;
  if (!tag) return defaultRelease(model);
  const normalized = decodeURIComponent(tag).toLowerCase();
  return model.tags.find((release) =>
    release.tag.toLowerCase() === normalized ||
    release.target?.toLowerCase() === normalized ||
    release.aliases?.some((alias) => alias.toLowerCase() === normalized)
  ) || null;
}

function validateRelease(model, release) {
  const errors = [];
  if (!release.tag || !release.target || !release.runner) {
    errors.push(`${model.name} contains an incomplete release`);
  }
  if (release.size_bytes != null && (!Number.isFinite(release.size_bytes) || release.size_bytes < 0)) {
    errors.push(`${model.name}:${release.tag} has an invalid size`);
  }
  if (release.hardware?.cpu === false && release.hardware?.gpu === false) {
    errors.push(`${model.name}:${release.tag} supports neither CPU nor GPU`);
  }
  if (release.hardware?.cpu === false && !release.hardware?.gpu) {
    errors.push(`${model.name}:${release.tag} requires undeclared hardware`);
  }
  return errors;
}

export function validateRegistry(value) {
  const errors = [];
  if (value?.schema_version !== 1 || value?.namespace !== "library") {
    errors.push("Unsupported registry schema");
  }
  if (!Array.isArray(value?.models)) {
    errors.push("Registry models must be an array");
    return errors;
  }
  const identities = new Set();
  for (const model of value.models) {
    if (!model.name || !model.display_name || !Array.isArray(model.tags)) {
      errors.push("Registry contains malformed model data");
      continue;
    }
    if (identities.has(model.name)) errors.push(`Duplicate model identity: ${model.name}`);
    identities.add(model.name);
    if (!model.tags.some((release) => release.tag === model.default_tag)) {
      errors.push(`${model.name} has no default release`);
    }
    for (const release of model.tags) errors.push(...validateRelease(model, release));
  }
  return errors;
}

export function normalizeModel(model) {
  const release = defaultRelease(model);
  if (!release) throw new Error(`${model.name} has no releases`);
  const status = verificationStatus(model, release);
  const hardware = release.hardware || {};
  return {
    ...model,
    release,
    ref: canonicalRef(model.name, release.tag, model.default_tag),
    primaryTask: primaryTask(model),
    taskLabels: taskLabels(model),
    shortSummary: shortSummary(model),
    languages: languages(model),
    platforms: supportedPlatforms(model),
    limitations: knownLimitations(model),
    status,
    recommended: isRecommended(model, release),
    sizeBytes: Number.isFinite(release.size_bytes) && release.size_bytes > 0
      ? release.size_bytes
      : null,
    hardware: {
      cpu: Boolean(hardware.cpu),
      gpu: Boolean(hardware.gpu),
      gpuRequired: hardware.cpu === false && hardware.gpu === true,
      minRam: hardware.min_ram || null,
      minVram: hardware.min_vram || null,
      minRamMb: parseMemory(hardware.min_ram),
      minVramMb: parseMemory(hardware.min_vram),
    },
  };
}

export async function getRegistry({ force = false } = {}) {
  if (cache && !force) return cache;
  const response = await fetch("/v1/registry.json", {
    headers: { accept: "application/json" },
  });
  if (!response.ok) throw new Error(`Registry returned ${response.status}`);
  const value = await response.json();
  const errors = validateRegistry(value);
  if (errors.length) {
    const error = new Error("Registry data is malformed");
    error.details = errors;
    throw error;
  }
  cache = {
    ...value,
    models: value.models.map(normalizeModel),
  };
  return cache;
}
