import { useEffect, useState } from "react";
import { CommandBar } from "../components/CommandBar";
import { PlatformInstall } from "../components/PlatformInstall";
import { installCommand } from "../lib/platform";

const WINDOWS_RELEASE_ENDPOINT = "/v1/releases/stable/windows-x86_64.json";

export function DownloadPage() {
  const [release, setRelease] = useState({ status: "checking", version: null });

  useEffect(() => {
    let cancelled = false;
    const check = async () => {
      try {
        const response = await fetch(WINDOWS_RELEASE_ENDPOINT, {
          headers: { accept: "application/json" },
        });
        if (!response.ok) throw new Error("stable release unavailable");
        const metadata = await response.json();
        if (
          metadata?.channel !== "stable" ||
          metadata?.platform !== "windows" ||
          metadata?.architecture !== "x86_64" ||
          metadata?.test_fixture !== false
        ) {
          throw new Error("stable release metadata is invalid");
        }
        if (!cancelled) setRelease({ status: "ready", version: metadata.version });
      } catch {
        if (!cancelled) setRelease({ status: "unavailable", version: null });
      }
    };
    check();
    return () => { cancelled = true; };
  }, []);

  return (
    <main className="shell page download-page">
      <header className="compact-page-head">
        <p className="eyebrow">Windows distribution</p>
        <h1>Download Takokit</h1>
        <p>Install the same canonical Takokit application either from PowerShell or with the normal Windows installer.</p>
      </header>

      <PlatformInstall heading="Install for your machine" />

      <section className="windows-download" aria-labelledby="windows-download-heading">
        <p className="eyebrow">Windows x86_64</p>
        <h2 id="windows-download-heading">PowerShell</h2>
        <CommandBar label="Takokit PowerShell installer command">{installCommand("windows")}</CommandBar>
        <p className="windows-download__security">The bootstrap resolves Takokit stable release metadata, verifies the installer SHA-256, then runs the same Inno Setup installer used by the download button.</p>

        <div className="download-divider" aria-hidden="true"><span>or</span></div>

        {release.status === "ready" ? (
          <a className="download-primary" href="/download/windows">
            Download for Windows{release.version ? ` · v${release.version}` : ""}
          </a>
        ) : (
          <button className="download-primary" type="button" disabled aria-disabled="true">
            {release.status === "checking" ? "Checking Windows release…" : "Windows stable release not published yet"}
          </button>
        )}
        <p className="windows-download__requirement">Windows 10 or Windows 11 · x86_64 · Run <code>tako</code> for the TUI or <code>tako gui</code> for the local browser GUI.</p>
      </section>

      <aside className="truth-note">
        <h2>Release status</h2>
        <p>Linux and macOS installers are not published yet. The Windows stable endpoint also fails closed until an approved stable release with production release-signing identity is published.</p>
      </aside>
    </main>
  );
}
