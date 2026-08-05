const REPOSITORY = "amaansyed27/Takokit";

function repositoryRef() {
  return process.env.TAKOKIT_REPOSITORY_REF ||
    process.env.VERCEL_GIT_COMMIT_SHA ||
    "main";
}

export default async function handler(_request, response) {
  const ref = repositoryRef();
  const source = `https://raw.githubusercontent.com/${REPOSITORY}/${ref}/registry/index.json`;

  try {
    const upstream = await fetch(source, {
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
    response.setHeader("Access-Control-Allow-Origin", "*");
    response.setHeader("Cache-Control", "public, s-maxage=300, stale-while-revalidate=86400");
    response.setHeader("Content-Type", "application/json; charset=utf-8");
    response.setHeader("X-Takokit-Registry-Ref", ref);
    response.status(200).send(JSON.stringify(registry));
  } catch (error) {
    response.setHeader("Cache-Control", "no-store");
    response.status(502).json({
      error: "registry_unavailable",
      message: error instanceof Error ? error.message : String(error),
      ref,
    });
  }
}
