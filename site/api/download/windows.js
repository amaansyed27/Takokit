import { resolveStableWindowsRelease } from "../_release.js";

export default async function handler(_request, response) {
  try {
    const release = await resolveStableWindowsRelease();
    response.setHeader("Cache-Control", "no-store");
    response.redirect(307, release.installer.url);
  } catch (error) {
    response.setHeader("Cache-Control", "no-store");
    response.status(503).json({
      error: "stable_release_unavailable",
      message: error instanceof Error ? error.message : String(error),
    });
  }
}
