const REPOSITORY = "amaansyed27/Takokit";
const DEFAULT_STABLE_MANIFEST_URL = `https://github.com/${REPOSITORY}/releases/latest/download/release-manifest.json`;
const TEST_SIGNING_KEY_ID = "takokit-test-fixture-v1";

export class StableReleaseUnavailableError extends Error {
  constructor(message) {
    super(message);
    this.name = "StableReleaseUnavailableError";
  }
}

function isHttpsUrl(value) {
  try {
    return new URL(value).protocol === "https:";
  } catch {
    return false;
  }
}

function fail(message) {
  throw new StableReleaseUnavailableError(message);
}

export function projectStableWindowsRelease(manifest, manifestUrl = DEFAULT_STABLE_MANIFEST_URL) {
  if (!manifest || typeof manifest !== "object") fail("release manifest is missing");
  if (manifest.product !== "Takokit") fail("release manifest product is invalid");
  if (manifest.channel !== "stable") fail("no stable release is published");
  if (manifest.test_fixture === true) fail("test fixtures cannot be served as stable releases");
  if (manifest.os !== "windows" || manifest.architecture !== "x86_64") {
    fail("stable release is not for Windows x86_64");
  }
  if (typeof manifest.version !== "string" || !/^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/.test(manifest.version)) {
    fail("stable release version is invalid");
  }
  if (
    typeof manifest.signing_key_id !== "string" ||
    manifest.signing_key_id.length === 0 ||
    manifest.signing_key_id === TEST_SIGNING_KEY_ID
  ) {
    fail("stable release does not have a production signing identity");
  }
  if (!Array.isArray(manifest.artifacts)) fail("release manifest artifacts are missing");

  const installer = manifest.artifacts.find((artifact) => artifact?.role === "installer");
  if (!installer) fail("stable release installer metadata is missing");
  if (typeof installer.name !== "string" || !/^Takokit-v.+-windows-x86_64-installer\.exe$/.test(installer.name)) {
    fail("stable release installer name is invalid");
  }
  if (typeof installer.sha256 !== "string" || !/^[0-9a-fA-F]{64}$/.test(installer.sha256)) {
    fail("stable release installer SHA-256 is invalid");
  }

  const installerUrl = installer.url ||
    `https://github.com/${REPOSITORY}/releases/latest/download/${encodeURIComponent(installer.name)}`;
  if (!isHttpsUrl(installerUrl)) fail("stable release installer URL must use HTTPS");
  if (!isHttpsUrl(manifestUrl)) fail("stable release manifest URL must use HTTPS");

  const signatureUrl = manifestUrl.replace(/release-manifest\.json(?:\?.*)?$/, "release-manifest.sig");
  return {
    schema_version: 1,
    product: "Takokit",
    version: manifest.version,
    channel: "stable",
    platform: "windows",
    architecture: "x86_64",
    signing_key_id: manifest.signing_key_id,
    test_fixture: false,
    release_manifest: {
      url: manifestUrl,
      signature_url: signatureUrl,
    },
    installer: {
      name: installer.name,
      url: installerUrl,
      sha256: installer.sha256.toLowerCase(),
      size: Number.isFinite(Number(installer.size)) ? Number(installer.size) : null,
    },
  };
}

export async function resolveStableWindowsRelease({
  fetchImpl = globalThis.fetch,
  manifestUrl = process.env.TAKOKIT_STABLE_RELEASE_MANIFEST_URL || DEFAULT_STABLE_MANIFEST_URL,
} = {}) {
  if (typeof fetchImpl !== "function") fail("release metadata fetch is unavailable");
  if (!isHttpsUrl(manifestUrl)) fail("stable release manifest URL must use HTTPS");

  let response;
  try {
    response = await fetchImpl(manifestUrl, {
      headers: { accept: "application/json" },
      redirect: "follow",
      cache: "no-store",
    });
  } catch (error) {
    fail(`stable release manifest is unavailable: ${error instanceof Error ? error.message : String(error)}`);
  }
  if (!response?.ok) fail(`stable release manifest returned ${response?.status ?? "an error"}`);

  let manifest;
  try {
    manifest = await response.json();
  } catch {
    fail("stable release manifest is malformed");
  }
  return projectStableWindowsRelease(manifest, manifestUrl);
}

export function projectStableUnixRelease(manifest, platform, architecture, manifestUrl) {
  if (!manifest || typeof manifest !== "object") fail("release manifest is missing");
  if (![["linux", "x86_64"], ["macos", "arm64"], ["macos", "x86_64"]]
    .some(([os, arch]) => os === platform && arch === architecture)) {
    fail("requested platform is not supported");
  }
  if (manifest.product !== "Takokit" || manifest.channel !== "stable" || manifest.test_fixture !== false) {
    fail("no production stable release is published");
  }
  if (manifest.os !== platform || manifest.architecture !== architecture) {
    fail("stable release platform does not match the requested target");
  }
  if (manifest.signing_key_id !== "takokit-release-v1") fail("stable release does not have the production signing identity");
  if (typeof manifest.version !== "string" || !/^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/.test(manifest.version)) {
    fail("stable release version is invalid");
  }
  const expectedName = `Takokit-v${manifest.version}-${platform}-${architecture}.tar.gz`;
  const artifact = Array.isArray(manifest.artifacts)
    ? manifest.artifacts.find((candidate) => candidate?.role === "portable" && candidate?.name === expectedName)
    : null;
  if (!artifact || typeof artifact.sha256 !== "string" || !/^[0-9a-fA-F]{64}$/.test(artifact.sha256)) {
    fail("stable release portable artifact metadata is invalid");
  }
  const artifactUrl = artifact.url || `https://github.com/${REPOSITORY}/releases/latest/download/${encodeURIComponent(expectedName)}`;
  if (!isHttpsUrl(artifactUrl) || !isHttpsUrl(manifestUrl)) fail("stable release URLs must use HTTPS");
  return {
    schema_version: 1,
    product: "Takokit",
    version: manifest.version,
    channel: "stable",
    platform,
    architecture,
    signing_key_id: manifest.signing_key_id,
    test_fixture: false,
    manifest_url: manifestUrl,
    signature_url: manifestUrl.replace(/\.json(?:\?.*)?$/, ".sig"),
    artifact_name: expectedName,
    artifact_url: artifactUrl,
    artifact_sha256: artifact.sha256.toLowerCase(),
    artifact_size: Number.isFinite(Number(artifact.size)) ? Number(artifact.size) : null,
  };
}

export async function resolveStableUnixRelease(platform, architecture, {
  fetchImpl = globalThis.fetch,
  manifestUrl = `https://github.com/${REPOSITORY}/releases/latest/download/release-manifest-${platform}-${architecture}.json`,
} = {}) {
  if (typeof fetchImpl !== "function" || !isHttpsUrl(manifestUrl)) fail("stable release manifest URL must use HTTPS");
  let response;
  try {
    response = await fetchImpl(manifestUrl, { headers: { accept: "application/json" }, redirect: "follow", cache: "no-store" });
  } catch (error) {
    fail(`stable release manifest is unavailable: ${error instanceof Error ? error.message : String(error)}`);
  }
  if (!response?.ok) fail(`stable release manifest returned ${response?.status ?? "an error"}`);
  let manifest;
  try { manifest = await response.json(); } catch { fail("stable release manifest is malformed"); }
  return projectStableUnixRelease(manifest, platform, architecture, manifestUrl);
}
