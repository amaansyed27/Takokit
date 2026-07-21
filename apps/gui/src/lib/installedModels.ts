import { apiConfig } from "./api";
import type { RuntimeSnapshot } from "./types";

type InstalledModelEntry = {
  name: string;
};

type InstalledModelsResponse = {
  kind: "installed-models";
  data: InstalledModelEntry[];
};

export async function withVerifiedInstalledModels(runtime: RuntimeSnapshot): Promise<RuntimeSnapshot> {
  if (runtime.server.status !== "online") {
    return { ...runtime, models: [] };
  }

  const response = await fetch(`${apiConfig.localBaseUrl}/v1/models/installed`);
  if (!response.ok) {
    throw new Error(`Installed model inventory failed with ${response.status}`);
  }

  const inventory = (await response.json()) as InstalledModelsResponse;
  const installedIds = new Set(inventory.data.map((model) => model.name));

  return {
    ...runtime,
    models: runtime.models.filter((model) => installedIds.has(model.id)),
    modeNote: "Only models installed and verified on this machine are shown."
  };
}
