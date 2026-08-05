import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

const REPOSITORY = "amaansyed27/Takokit";
const registryPath = fileURLToPath(new URL("../registry/index.json", import.meta.url));
const brandAssets = new Map([
  ["/brand/takokit-mark.svg", {
    localPath: fileURLToPath(new URL("../assets/svg-transparent/512.svg", import.meta.url)),
    repositoryPath: "assets/svg-transparent/512.svg",
  }],
  ["/brand/takokit-mark-on-white.svg", {
    localPath: fileURLToPath(new URL("../assets/svg-white/512-white.svg", import.meta.url)),
    repositoryPath: "assets/svg-white/512-white.svg",
  }],
  ["/favicon.ico", {
    localPath: fileURLToPath(new URL("../assets/favicon/favicon.ico", import.meta.url)),
    repositoryPath: "assets/favicon/favicon.ico",
  }],
  ["/favicon-16x16.png", {
    localPath: fileURLToPath(new URL("../assets/favicon/favicon-16x16.png", import.meta.url)),
    repositoryPath: "assets/favicon/favicon-16x16.png",
  }],
  ["/favicon-32x32.png", {
    localPath: fileURLToPath(new URL("../assets/favicon/favicon-32x32.png", import.meta.url)),
    repositoryPath: "assets/favicon/favicon-32x32.png",
  }],
  ["/apple-touch-icon.png", {
    localPath: fileURLToPath(new URL("../assets/favicon/apple-touch-icon.png", import.meta.url)),
    repositoryPath: "assets/favicon/apple-touch-icon.png",
  }],
  ["/android-chrome-192x192.png", {
    localPath: fileURLToPath(new URL("../assets/favicon/android-chrome-192x192.png", import.meta.url)),
    repositoryPath: "assets/favicon/android-chrome-192x192.png",
  }],
  ["/android-chrome-512x512.png", {
    localPath: fileURLToPath(new URL("../assets/favicon/android-chrome-512x512.png", import.meta.url)),
    repositoryPath: "assets/favicon/android-chrome-512x512.png",
  }],
  ["/site.webmanifest", {
    localPath: fileURLToPath(new URL("../assets/favicon/site.webmanifest", import.meta.url)),
    repositoryPath: "assets/favicon/site.webmanifest",
  }],
]);

function repositoryRef() {
  return process.env.TAKOKIT_REPOSITORY_REF ||
    process.env.VERCEL_GIT_COMMIT_SHA ||
    "main";
}

async function readRepositoryFile(localPath, repositoryPath) {
  try {
    return await readFile(localPath);
  } catch (error) {
    if (error?.code !== "ENOENT") throw error;
  }

  const source = `https://raw.githubusercontent.com/${REPOSITORY}/${repositoryRef()}/${repositoryPath}`;
  const response = await fetch(source, { headers: { accept: "application/octet-stream" } });
  if (!response.ok) {
    throw new Error(`Unable to load ${repositoryPath} from ${repositoryRef()}: ${response.status}`);
  }
  return Buffer.from(await response.arrayBuffer());
}

function contentType(pathname) {
  if (pathname.endsWith(".svg")) return "image/svg+xml";
  if (pathname.endsWith(".png")) return "image/png";
  if (pathname.endsWith(".ico")) return "image/x-icon";
  if (pathname.endsWith(".webmanifest")) return "application/manifest+json";
  return "application/octet-stream";
}

function canonicalAssetsBuildPlugin() {
  return {
    name: "takokit-root-brand-assets-build",
    apply: "build",
    async buildStart() {
      for (const [publicPath, asset] of brandAssets) {
        this.emitFile({
          type: "asset",
          fileName: publicPath.slice(1),
          source: await readRepositoryFile(asset.localPath, asset.repositoryPath),
        });
      }
    },
  };
}

function canonicalAssetsServePlugin() {
  return {
    name: "takokit-root-brand-assets-serve",
    apply: "serve",
    configureServer(server) {
      server.middlewares.use(async (request, response, next) => {
        const pathname = request.url?.split("?", 1)[0] || "";
        const asset = brandAssets.get(pathname);
        if (!asset) {
          next();
          return;
        }
        try {
          response.setHeader("Content-Type", contentType(pathname));
          response.end(await readRepositoryFile(asset.localPath, asset.repositoryPath));
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
    apply: "serve",
    configureServer(server) {
      server.middlewares.use("/v1/registry.json", async (_request, response) => {
        try {
          response.setHeader("Content-Type", "application/json; charset=utf-8");
          response.end(await readRepositoryFile(registryPath, "registry/index.json"));
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
  plugins: [
    react(),
    canonicalAssetsBuildPlugin(),
    canonicalAssetsServePlugin(),
    localRegistryPlugin(),
  ],
  build: {
    outDir: "dist",
    emptyOutDir: true,
    sourcemap: true,
  },
});
