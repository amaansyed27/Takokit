import assert from "node:assert/strict";

const input = process.argv[2] || process.env.TAKOKIT_DEPLOYMENT_URL;
if (!input) {
  console.error("Usage: npm run verify:deployment -- https://your-deployment.vercel.app");
  process.exit(2);
}

const base = new URL(input.endsWith("/") ? input : `${input}/`);
const bypass = process.env.VERCEL_AUTOMATION_BYPASS_SECRET;
const requestHeaders = bypass
  ? { "x-vercel-protection-bypass": bypass }
  : {};

async function request(pathname, expectedType) {
  const url = new URL(pathname.replace(/^\//, ""), base);
  const response = await fetch(url, {
    redirect: "follow",
    headers: requestHeaders,
  });
  const body = await response.text();
  assert.equal(response.status, 200, `${url} returned ${response.status}: ${body.slice(0, 180)}`);
  if (expectedType) {
    assert.match(
      response.headers.get("content-type") || "",
      expectedType,
      `${url} returned an unexpected content type`,
    );
  }
  return { url, response, body };
}

function validateRegistry(value, label) {
  assert.equal(value.schema_version, 1, `${label} has an unsupported schema`);
  assert.equal(value.namespace, "library", `${label} has an unexpected namespace`);
  assert.ok(Array.isArray(value.models) && value.models.length > 0, `${label} has no models`);
  for (const model of value.models) {
    assert.equal(typeof model.name, "string", `${label} contains a model without a name`);
    assert.ok(Array.isArray(model.tags), `${label} contains malformed tags for ${model.name}`);
  }
}

const directRoutes = [
  "/",
  "/models",
  "/library",
  "/models/kokoro",
  "/library/kokoro",
  "/docs",
  "/docs/install",
  "/download",
  "/this-route-must-render-the-spa",
];

let indexHtml = "";
for (const pathname of directRoutes) {
  const result = await request(pathname, /text\/html/i);
  assert.match(result.body, /<div id="root"><\/div>/, `${pathname} is not serving the React application`);
  if (pathname === "/") indexHtml = result.body;
}

assert.doesNotMatch(indexHtml, /scripts\/build\.mjs|assets\/base\.css/i, "legacy static source is still referenced");

const assetPaths = [...new Set(
  [...indexHtml.matchAll(/(?:src|href)="(\/assets\/[^"?#]+)(?:[?#][^"]*)?"/g)]
    .map((match) => match[1]),
)];
assert.ok(assetPaths.length > 0, "Vite did not emit hashed assets");
for (const assetPath of assetPaths) {
  const asset = await request(assetPath);
  assert.match(
    asset.response.headers.get("cache-control") || "",
    /immutable/i,
    `${assetPath} is missing immutable caching`,
  );
}

const registryAlias = await request("/v1/registry.json", /application\/json/i);
const registryApi = await request("/api/v1/registry", /application\/json/i);
const aliasValue = JSON.parse(registryAlias.body);
const apiValue = JSON.parse(registryApi.body);
validateRegistry(aliasValue, "/v1/registry.json");
validateRegistry(apiValue, "/api/v1/registry");
assert.equal(aliasValue.models.length, apiValue.models.length, "registry alias and API disagree");
assert.equal(
  registryAlias.response.headers.get("access-control-allow-origin"),
  "*",
  "registry CORS header is missing",
);

const mark = await request("/brand/takokit-mark.svg", /image\/svg\+xml/i);
assert.match(mark.body, /<svg[\s>]/i, "Takokit mark is not an SVG");
await request("/favicon.ico", /image\/(?:x-icon|vnd\.microsoft\.icon)/i);
const manifest = await request("/site.webmanifest", /application\/(?:manifest\+json|json)/i);
JSON.parse(manifest.body);

const installPowerShell = await request("/install.ps1", /(?:text\/plain|application\/octet-stream)/i);
assert.match(installPowerShell.body, /amaansyed27\/Takokit\.git/, "PowerShell installer does not target Takokit");
const installShell = await request("/install.sh", /(?:text\/plain|application\/octet-stream|text\/x-shellscript)/i);
assert.match(installShell.body, /amaansyed27\/Takokit\.git/, "shell installer does not target Takokit");

console.log(`Takokit deployment verified: ${base.origin}`);
console.log(`Routes: ${directRoutes.length}, hashed assets: ${assetPaths.length}, models: ${aliasValue.models.length}`);
