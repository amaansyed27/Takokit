import { resolveStableUnixRelease, StableReleaseUnavailableError } from "../../../_release.js";

export async function serveUnixRelease(response, platform, architecture) {
  try {
    const metadata = await resolveStableUnixRelease(platform, architecture);
    response.setHeader("Cache-Control", "public, s-maxage=60, stale-while-revalidate=300");
    response.setHeader("Access-Control-Allow-Origin", "*");
    return response.status(200).send(JSON.stringify(metadata));
  } catch (error) {
    const message = error instanceof StableReleaseUnavailableError ? error.message : "stable release unavailable";
    return response.status(503).json({ error: "stable_release_unavailable", message });
  }
}
