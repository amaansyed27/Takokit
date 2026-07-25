const SOURCE =
  "https://raw.githubusercontent.com/amaansyed27/Takokit/main/registry/index.json";

export default async function handler(_request, response) {
  try {
    const upstream = await fetch(SOURCE, {
      headers: { accept: "application/json" },
    });
    if (!upstream.ok) {
      throw new Error(`registry upstream returned ${upstream.status}`);
    }
    const registry = await upstream.json();
    if (
      registry.schema_version !== 1 ||
      registry.namespace !== "library" ||
      !Array.isArray(registry.models)
    ) {
      throw new Error("registry upstream returned an invalid schema");
    }
    response.setHeader(
      "Cache-Control",
      "public, s-maxage=300, stale-while-revalidate=86400",
    );
    response.setHeader("Access-Control-Allow-Origin", "*");
    response.setHeader("Content-Type", "application/json; charset=utf-8");
    response.status(200).send(JSON.stringify(registry));
  } catch (error) {
    response.status(502).json({
      error: "registry_unavailable",
      message: error instanceof Error ? error.message : String(error),
    });
  }
}
