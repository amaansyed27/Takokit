const TASK_FILTERS = {
  all: [],
  speech: ["tts"],
  transcription: ["stt", "live-transcription"],
  cloning: ["voice-cloning"],
  conversion: ["voice-conversion"],
};

export const DEFAULT_FILTERS = Object.freeze({
  task: "all",
  cpuFriendly: false,
  gpuSupported: false,
  gpuRequired: false,
  maxVram: "",
  maxSize: "",
  status: "",
  commercial: "",
  platform: "",
  runner: "",
});

export function filtersFromSearch(params) {
  return {
    task: params.get("task") || "all",
    query: params.get("q") || "",
    cpuFriendly: params.get("cpu") === "1",
    gpuSupported: params.get("gpu") === "1",
    gpuRequired: params.get("gpu_required") === "1",
    maxVram: params.get("max_vram") || "",
    maxSize: params.get("max_size") || "",
    status: params.get("status") || "",
    commercial: params.get("commercial") || "",
    platform: params.get("platform") || "",
    runner: params.get("runner") || "",
    sort: params.get("sort") || "recommended",
  };
}

export function searchFromFilters(filters) {
  const params = new URLSearchParams();
  if (filters.query) params.set("q", filters.query);
  if (filters.task && filters.task !== "all") params.set("task", filters.task);
  if (filters.cpuFriendly) params.set("cpu", "1");
  if (filters.gpuSupported) params.set("gpu", "1");
  if (filters.gpuRequired) params.set("gpu_required", "1");
  if (filters.maxVram) params.set("max_vram", filters.maxVram);
  if (filters.maxSize) params.set("max_size", filters.maxSize);
  if (filters.status) params.set("status", filters.status);
  if (filters.commercial) params.set("commercial", filters.commercial);
  if (filters.platform) params.set("platform", filters.platform);
  if (filters.runner) params.set("runner", filters.runner);
  if (filters.sort && filters.sort !== "recommended") params.set("sort", filters.sort);
  const value = params.toString();
  return value ? `?${value}` : "";
}

function commercialStatus(license) {
  const value = (license || "").toLowerCase();
  if (!value || value.includes("check-required") || value.includes("research")) return "unknown";
  if (value.includes("non-commercial") || value.includes("nc-")) return "no";
  return "yes";
}

function matchesTask(model, task) {
  const accepted = TASK_FILTERS[task] || [];
  return !accepted.length || accepted.some((value) => model.tasks.includes(value));
}

function matchesSearch(model, query) {
  if (!query) return true;
  const release = model.release;
  const haystack = [
    model.name,
    model.display_name,
    model.summary,
    model.shortSummary,
    model.primaryTask,
    model.status,
    release.runner,
    release.adapter,
    release.backend,
    release.license,
    ...model.aliases,
    ...model.tasks,
    ...model.languages,
    ...model.platforms,
  ].filter(Boolean).join(" ").toLowerCase();
  return query.toLowerCase().trim().split(/\s+/).every((part) => haystack.includes(part));
}

export function modelMatches(model, filters) {
  if (!matchesSearch(model, filters.query)) return false;
  if (!matchesTask(model, filters.task)) return false;
  if (filters.cpuFriendly && !model.hardware.cpu) return false;
  if (filters.gpuSupported && !model.hardware.gpu) return false;
  if (filters.gpuRequired && !model.hardware.gpuRequired) return false;

  const maxVram = Number(filters.maxVram);
  if (maxVram) {
    if (!model.hardware.minVramMb || model.hardware.minVramMb > maxVram * 1024) return false;
  }

  const maxSize = Number(filters.maxSize);
  if (maxSize) {
    if (!model.sizeBytes || model.sizeBytes > maxSize * 1_000_000) return false;
  }

  if (filters.status && model.status !== filters.status) return false;
  if (filters.commercial && commercialStatus(model.release.license) !== filters.commercial) {
    return false;
  }
  if (
    filters.platform &&
    !model.platforms.some((platform) =>
      platform.toLowerCase() === filters.platform.toLowerCase()
    )
  ) return false;
  if (filters.runner && model.release.runner !== filters.runner) return false;
  return true;
}

const hardwareRank = (model) => {
  if (model.hardware.cpu && !model.hardware.gpuRequired) return 0;
  if (model.hardware.gpu && !model.hardware.gpuRequired) return 1;
  return 2;
};

export function sortModels(models, sort) {
  const copy = [...models];
  const byName = (a, b) => a.display_name.localeCompare(b.display_name);
  switch (sort) {
    case "name":
      return copy.sort(byName);
    case "smallest":
      return copy.sort((a, b) =>
        (a.sizeBytes ?? Number.MAX_SAFE_INTEGER) -
          (b.sizeBytes ?? Number.MAX_SAFE_INTEGER) || byName(a, b)
      );
    case "hardware":
      return copy.sort((a, b) =>
        hardwareRank(a) - hardwareRank(b) ||
        (a.hardware.minVramMb ?? 0) - (b.hardware.minVramMb ?? 0) ||
        byName(a, b)
      );
    case "verified":
      return copy.sort((a, b) =>
        Number(b.status === "verified") - Number(a.status === "verified") ||
        byName(a, b)
      );
    default:
      return copy.sort((a, b) =>
        Number(b.recommended) - Number(a.recommended) ||
        Number(b.status === "verified") - Number(a.status === "verified") ||
        hardwareRank(a) - hardwareRank(b) ||
        byName(a, b)
      );
  }
}

export function filterModels(models, filters) {
  return sortModels(models.filter((model) => modelMatches(model, filters)), filters.sort);
}
