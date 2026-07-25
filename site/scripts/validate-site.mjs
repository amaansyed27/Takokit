import { access, readFile } from "node:fs/promises";

const root = new URL("../", import.meta.url);
for (const path of ["api/v1/registry.js", "vercel.json"]) {
  await access(new URL(path, root));
}

const registryPath = new URL("../../registry/index.json", import.meta.url);
try {
  const registry = JSON.parse(await readFile(registryPath, "utf8"));
  const releases = registry.models.reduce((total, model) => total + model.tags.length, 0);
  if (registry.schema_version !== 1 || registry.models.length !== 24 || releases !== 31) {
    throw new Error(`unexpected registry shape: ${registry.models.length} families / ${releases} releases`);
  }
} catch (error) {
  if (error.code !== "ENOENT") throw error;
  console.warn("Registry file is outside this standalone checkout; skipped cross-tree validation.");
}

const handler = await readFile(new URL("api/v1/registry.js", root), "utf8");
if (!handler.includes("registry/index.json") || !handler.includes("schema_version")) {
  throw new Error("registry API is not wired to the validated repository index");
}

console.log("Takokit registry API validation passed");
