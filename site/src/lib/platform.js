export const PUBLIC_SITE_ORIGIN = "https://takokit.dawnlightlabs.com";
export const PLATFORM_ORDER = ["windows", "linux", "macos"];

export const PLATFORM_DETAILS = {
  windows: {
    label: "Windows",
    shell: "PowerShell",
    architecture: "x86_64",
    available: true,
    endpoint: "/v1/releases/stable/windows-x86_64.json",
    note: "Windows 10 or Windows 11 on x86_64. The installer adds the Takokit app, CLI/TUI, local server, browser GUI, updater, and notification-area resident controller.",
  },
  linux: {
    label: "Linux",
    shell: "Terminal",
    architecture: "x86_64",
    available: true,
    endpoint: "/v1/releases/stable/linux-x86_64.json",
    note: "Linux x86_64 uses a per-user install under ~/.local and includes the CLI/TUI, server, browser GUI, updater, and freedesktop launcher. Linux ARM64 is not published in v0.3.0.",
  },
  macos: {
    label: "macOS",
    shell: "Terminal",
    architecture: "arm64",
    available: true,
    endpoint: "/v1/releases/stable/macos-arm64.json",
    note: "macOS 12+ on Apple Silicon uses a per-user install with Takokit.app, the menu-bar resident controller, CLI/TUI, local server, browser GUI, and updater. Intel macOS remains experimental in v0.3.0.",
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
  if (platform === "windows") return `irm ${PUBLIC_SITE_ORIGIN}/install.ps1 | iex`;
  if (platform === "linux" || platform === "macos") {
    return `curl -fsSL ${PUBLIC_SITE_ORIGIN}/install.sh | sh`;
  }
  return null;
}

export function releaseEndpoint(platform) {
  return PLATFORM_DETAILS[platform]?.endpoint || null;
}
