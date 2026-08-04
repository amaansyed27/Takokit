import { CommandBar } from "../components/CommandBar";
import { PlatformInstall } from "../components/PlatformInstall";

export function DownloadPage() {
  return (
    <main className="shell page download-page">
      <header className="compact-page-head">
        <p className="eyebrow">Initial public testing</p>
        <h1>Install Takokit</h1>
        <p>Takokit detects your platform and shows the correct PowerShell or shell command. Until signed release packages are published, this installer performs the documented source build and installs the resulting binary to your user account.</p>
      </header>

      <PlatformInstall heading="Install for your machine" />

      <div className="platform-tabs" aria-label="Platform installation details">
        <section>
          <span>Windows</span>
          <h2>User installation</h2>
          <p>The PowerShell installer places <code>tako.exe</code> in <code>%LOCALAPPDATA%\Takokit\bin</code> and adds that directory to your user PATH.</p>
          <CommandBar>tako doctor</CommandBar>
        </section>
        <section>
          <span>Linux</span>
          <h2>User installation</h2>
          <p>The shell installer places <code>tako</code> in <code>~/.local/bin</code>. Add that directory to PATH when your distribution does not include it automatically.</p>
          <CommandBar>tako doctor</CommandBar>
        </section>
        <section>
          <span>macOS</span>
          <h2>User installation</h2>
          <p>The shell installer places <code>tako</code> in <code>~/.local/bin</code>. The current path is source-built and does not imply signed or notarized packaging.</p>
          <CommandBar>tako doctor</CommandBar>
        </section>
      </div>

      <aside className="truth-note">
        <h2>Current installer status</h2>
        <p>The <code>irm</code> and <code>curl</code> commands are real and platform-aware, but they currently require Git, Rust stable, Node.js LTS, and npm because signed prebuilt artifacts are not published yet. The scripts can later switch to release archives without changing the homepage commands.</p>
      </aside>
    </main>
  );
}
