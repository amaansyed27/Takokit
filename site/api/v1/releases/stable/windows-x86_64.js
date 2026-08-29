import { resolveStableWindowsRelease } from "../../../_release.js";

export default async function handler(_request, response) {
  try {
    const release = await resolveStableWindowsRelease();
    response.setHeader("Access-Control-Allow-Origin", "*");
    response.setHeader("Cache-Control", "public, s-maxage=60, stale-while-revalidate=300");
    response.setHeader("Content-Type", "application/json; charset=utf-8");
    response.status(200).send(JSON.stringify(release));
  } catch (error) {
    response.setHeader("Cache-Control", "no-store");
    response.status(503).json({
      error: "stable_release_unavailable",
      message: error instanceof Error ? error.message : String(error),
    });
  }
}
