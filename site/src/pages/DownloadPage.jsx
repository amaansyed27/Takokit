import { useEffect, useState } from "react";
import { CommandBar } from "../components/CommandBar";
import { PlatformInstall } from "../components/PlatformInstall";
import {
  installCommand,
  PLATFORM_DETAILS,
  PLATFORM_ORDER,
  releaseEndpoint,
} from "../lib/platform";

function useStableReleases() {
  const [releases, setReleases] = useState(() => Object.fromEntries(
    PLATFORM_ORDER.map((platform) => [platform, { status: "checking", version: null }]),
  ));

  useEffect(() => {
    let cancelled = false;
    const load = async (platform) => {
      const details = PLATFORM_DETAILS[platform];
      try {
        const response = await fetch(releaseEndpoint(platform), { headers: { accept: "application/json" } });
        if (!response.ok) throw new Error("stable release unavailable");
        const metadata = await response.json();
        if (
          metadata?.channel !== "stable" ||
          metadata?.platform !== platform ||
          metadata?.architecture !== details.architecture ||
          metadata?.test_fixture !== false
        ) {
          throw new Error("stable release metadata is invalid");
        }
        if (!cancelled) {
          setReleases((current) => ({
            ...current,
            [platform]: { status: "ready", version: metadata.version },
          }));
        }
      } catch {
        if (!cancelled) {
          setReleases((current) => ({
            ...current,
            [platform]: { status: "unavailable", version: null },
          }));
        }
      }
    };

    Promise.all(PLATFORM_ORDER.map(load));
    return () => { cancelled = true; };
  }, []);

  return releases;
}

function PlatformReleaseCard({ platform, release }) {
  const details = PLATFORM_DETAILS[platform];
  const command = installCommand(platform);
  const directDownload = platform === "windows" ? "/download/windows" : null;

  return (
    <section className="platform-release-card" aria-labelledby={`${platform}-download-heading`}>
      <p className="eyebrow">{details.label} · {details.architecture}</p>
      <h2 id={`${platform}-download-heading`}>{details.shell} install</h2>
      <CommandBar label={`${details.label} installer command`}>{command}</CommandBar>
      <p className="platform-release-card__security">
        Stable metadata is resolved first and the selected artifact is checksum-verified before Takokit is installed.
      </p>

      {directDownload && release.status === "ready" ? (
        <>
          <div className="download-divider" aria-hidden="true"><span>or</span></div>
          <a className="download-primary" href={directDownload}>
            Download installer{release.version ? ` · v${release.version}` : ""}
          </a>
        </>
      ) : null}

      <p className="platform-release-card__status" role="status">
        {release.status === "checking" && "Checking stable release…"}
        {release.status === "ready" && `Stable beta available · v${release.version}`}
        {release.status === "unavailable" && "Stable package is not published for this platform yet."}
      </p>
      <p className="platform-release-card__note">{details.note}</p>
    </section>
  );
}

export function DownloadPage() {
  const releases = useStableReleases();

  return (
    <main className="shell page download-page">
      <header className="compact-page-head">
        <p className="eyebrow">Takokit beta</p>
        <h1>Install Takokit</h1>
        <p>
          Takokit 0.x releases are public beta builds. Windows x86_64, Linux x86_64, and macOS Apple Silicon use the same signed release metadata and local runtime contracts.
        </p>
      </header>

      <PlatformInstall heading="Install for your machine" />

      <div className="platform-release-grid">
        {PLATFORM_ORDER.map((platform) => (
          <PlatformReleaseCard key={platform} platform={platform} release={releases[platform]} />
        ))}
      </div>

      <aside className="truth-note">
        <h2>Beta support policy</h2>
        <p>
          Every 0.x release is a beta. Takokit 1.0.0 is the first stable release target. Linux ARM64 and Intel macOS are not advertised as stable v0.3.0 targets; model-level GPU and accelerator support can also vary by upstream runtime.
        </p>
      </aside>
    </main>
  );
}
