import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

const registryPath = fileURLToPath(new URL("../registry/index.json", import.meta.url));

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
  plugins: [react(), localRegistryPlugin()],
  build: {
    outDir: "dist",
    emptyOutDir: true,
    sourcemap: true,
  },
});
