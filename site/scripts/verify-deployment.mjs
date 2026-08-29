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

async function request(pathname, expectedType, { expectedStatus = 200, redirect = "follow" } = {}) {
  const url = new URL(pathname.replace(/^\//, ""), base);
  const response = await fetch(url, {
    redirect,
    headers: requestHeaders,
  });
  const body = await response.text();
  assert.equal(response.status, expectedStatus, `${url} returned ${response.status}: ${body.slice(0, 180)}`);
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

const installPowerShell = await request("/install.ps1", /text\/plain/i);
assert.doesNotMatch(installPowerShell.body, /<html|<div id="root"/i, "/install.ps1 returned the SPA shell");
assert.match(installPowerShell.body, /v1\/releases\/stable\/windows-x86_64\.json/);
assert.match(installPowerShell.body, /System\.Security\.Cryptography\.SHA256/);
assert.match(installPowerShell.body, /ComputeHash/);
assert.doesNotMatch(installPowerShell.body, /git clone|cargo build|npm ci/i);

const stableUrl = new URL("v1/releases/stable/windows-x86_64.json", base);
const stableResponse = await fetch(stableUrl, { headers: requestHeaders, redirect: "manual" });
const stableBody = await stableResponse.text();
if (stableResponse.status === 200) {
  assert.match(stableResponse.headers.get("content-type") || "", /application\/json/i);
  const stable = JSON.parse(stableBody);
  assert.equal(stable.channel, "stable");
  assert.equal(stable.platform, "windows");
  assert.equal(stable.architecture, "x86_64");
  assert.equal(stable.test_fixture, false);
  assert.match(stable.installer?.sha256 || "", /^[0-9a-f]{64}$/i);
} else {
  assert.equal(stableResponse.status, 503, `stable release route returned ${stableResponse.status}`);
  const unavailable = JSON.parse(stableBody);
  assert.equal(unavailable.error, "stable_release_unavailable");
}

const downloadUrl = new URL("download/windows", base);
const downloadResponse = await fetch(downloadUrl, { headers: requestHeaders, redirect: "manual" });
if (stableResponse.status === 200) {
  assert.equal(downloadResponse.status, 307, "/download/windows must redirect to the stable installer");
  assert.match(downloadResponse.headers.get("location") || "", /^https:\/\//i);
} else {
  assert.equal(downloadResponse.status, 503, "/download/windows must fail closed before stable release publication");
}

console.log(`Takokit deployment verified: ${base.origin}`);
console.log(`Routes: ${directRoutes.length}, hashed assets: ${assetPaths.length}, models: ${aliasValue.models.length}`);
