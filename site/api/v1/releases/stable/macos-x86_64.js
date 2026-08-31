import { serveUnixRelease } from "./_unix.js";
export default async function handler(_request, response) {
  return serveUnixRelease(response, "macos", "x86_64");
}
