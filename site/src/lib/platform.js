export const PUBLIC_SITE_ORIGIN = "https://takokit.dawnlightlabs.com";
export const PLATFORM_ORDER = ["windows", "linux", "macos"];

export const PLATFORM_DETAILS = {
  windows: {
    label: "Windows",
    shell: "PowerShell",
    available: true,
    note: "Windows 10 or Windows 11 on x86_64. The desktop GUI requires Microsoft Edge WebView2 Runtime.",
  },
  linux: {
    label: "Linux",
    shell: "Terminal",
    available: false,
    note: "Packaging is not available yet. Linux distribution work remains a later slice.",
  },
  macos: {
    label: "macOS",
    shell: "Terminal",
    available: false,
    note: "Packaging is not available yet. macOS distribution work remains a later slice.",
  },
};

export function detectPlatform(source = globalThis.navigator) {
  const value = [
    source?.userAgentData?.platform,
    source?.platform,
    source?.userAgent,
  ].filter(Boolean).join(" ").toLowerCase();

  if (value.includes("win")) return "windows";
  if (value.includes("mac") || value.includes("darwin")) return "macos";
  if (value.includes("linux") || value.includes("x11")) return "linux";
  return "windows";
}

export function installCommand(platform) {
  if (platform !== "windows") return null;
  return `irm ${PUBLIC_SITE_ORIGIN}/install.ps1 | iex`;
}
