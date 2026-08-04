export const PLATFORM_ORDER = ["windows", "macos", "linux"];

export const PLATFORM_DETAILS = {
  windows: {
    label: "Windows",
    shell: "PowerShell",
    note: "Installs to your user account. Git, Rust, Node.js, and npm are required while signed binaries are still in progress.",
  },
  macos: {
    label: "macOS",
    shell: "Terminal",
    note: "Installs to ~/.local/bin. Git, Rust, Node.js, and npm are required while signed binaries are still in progress.",
  },
  linux: {
    label: "Linux",
    shell: "Terminal",
    note: "Installs to ~/.local/bin. Git, Rust, Node.js, and npm are required while signed binaries are still in progress.",
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

export function installCommand(platform, origin) {
  const base = String(origin || "https://takokit-library.vercel.app").replace(/\/+$/, "");
  if (platform === "windows") return `irm ${base}/install.ps1 | iex`;
  return `curl -fsSL ${base}/install.sh | sh`;
}
