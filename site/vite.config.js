import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

const registryPath = fileURLToPath(new URL("../registry/index.json", import.meta.url));
const brandAssets = new Map([
  ["/brand/takokit-mark.svg", fileURLToPath(new URL("../assets/svg-transparent/512.svg", import.meta.url))],
  ["/brand/takokit-mark-on-white.svg", fileURLToPath(new URL("../assets/svg-white/512-white.svg", import.meta.url))],
  ["/favicon.ico", fileURLToPath(new URL("../assets/favicon/favicon.ico", import.meta.url))],
  ["/favicon-16x16.png", fileURLToPath(new URL("../assets/favicon/favicon-16x16.png", import.meta.url))],
  ["/favicon-32x32.png", fileURLToPath(new URL("../assets/favicon/favicon-32x32.png", import.meta.url))],
  ["/apple-touch-icon.png", fileURLToPath(new URL("../assets/favicon/apple-touch-icon.png", import.meta.url))],
  ["/android-chrome-192x192.png", fileURLToPath(new URL("../assets/favicon/android-chrome-192x192.png", import.meta.url))],
  ["/android-chrome-512x512.png", fileURLToPath(new URL("../assets/favicon/android-chrome-512x512.png", import.meta.url))],
  ["/site.webmanifest", fileURLToPath(new URL("../assets/favicon/site.webmanifest", import.meta.url))],
]);

function contentType(pathname) {
  if (pathname.endsWith(".svg")) return "image/svg+xml";
  if (pathname.endsWith(".png")) return "image/png";
  if (pathname.endsWith(".ico")) return "image/x-icon";
  if (pathname.endsWith(".webmanifest")) return "application/manifest+json";
  return "application/octet-stream";
}

function localBrandAssetsPlugin() {
  return {
    name: "takokit-root-brand-assets",
    async buildStart() {
      for (const [publicPath, sourcePath] of brandAssets) {
        this.emitFile({
          type: "asset",
          fileName: publicPath.slice(1),
          source: await readFile(sourcePath),
        });
      }
    },
    configureServer(server) {
      server.middlewares.use(async (request, response, next) => {
        const pathname = request.url?.split("?", 1)[0] || "";
        const sourcePath = brandAssets.get(pathname);
        if (!sourcePath) {
          next();
          return;
        }
        try {
          response.setHeader("Content-Type", contentType(pathname));
          response.end(await readFile(sourcePath));
        } catch (error) {
          response.statusCode = 404;
          response.end(error instanceof Error ? error.message : String(error));
        }
      });
    },
  };
}

function localRegistryPlugin() {
  return {
    name: "takokit-local-registry",
    configureServer(server) {
      server.middlewares.use("/v1/registry.json", async (_request, response) => {
        try {
          response.setHeader("Content-Type", "application/json; charset=utf-8");
          response.end(await readFile(registryPath, "utf8"));
        } catch (error) {
          response.statusCode = 503;
          response.end(JSON.stringify({
            error: "registry_unavailable",
            message: error instanceof Error ? error.message : String(error),
          }));
        }
      });
    },
  };
}

export default defineConfig({
  plugins: [react(), localBrandAssetsPlugin(), localRegistryPlugin()],
  build: {
    outDir: "dist",
    emptyOutDir: true,
    sourcemap: true,
  },
});
