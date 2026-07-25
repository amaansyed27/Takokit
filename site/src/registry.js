let cache;

export async function getRegistry() {
  if (cache) return cache;
  const response = await fetch("/v1/registry.json", { headers: { accept: "application/json" } });
  if (!response.ok) throw new Error(`Registry returned ${response.status}`);
  const value = await response.json();
  if (value.schema_version !== 1 || !Array.isArray(value.models)) {
    throw new Error("Unsupported registry schema");
  }
  cache = value;
  return value;
}

export function defaultRelease(model) {
  return model.tags.find((tag) => tag.tag === model.default_tag) || model.tags[0];
}

export function formatBytes(bytes) {
  if (!bytes) return "managed";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1000 && unit < units.length - 1) { value /= 1000; unit += 1; }
  return `${value >= 10 || unit === 0 ? value.toFixed(0) : value.toFixed(1)} ${units[unit]}`;
}
