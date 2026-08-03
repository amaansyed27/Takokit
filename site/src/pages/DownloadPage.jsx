import { CommandBar } from "../components/CommandBar";

export function DownloadPage() {
  return (
    <main className="shell page download-page">
      <header className="compact-page-head">
        <p className="eyebrow">Initial public testing</p>
        <h1>Download Takokit</h1>
        <p>Windows is the first packaging target. No installer or one-line install command is published until a real release artifact exists.</p>
      </header>
      <div className="platform-tabs" aria-label="Platform availability">
        <section>
          <span>Windows</span>
          <h2>Build from source</h2>
          <p>The <code>v0.0.1</code> installer layout is reserved here, but no nonexistent artifact is linked.</p>
          <CommandBar>cargo build --release</CommandBar>
          <CommandBar>.\target\release\tako.exe doctor</CommandBar>
          <strong className="availability">Packaging in progress · Windows-first</strong>
        </section>
        <section>
          <span>Linux</span>
          <h2>Build from source</h2>
          <p>A packaged Linux release is not currently declared. Heavy-model release testing is also not claimed.</p>
          <CommandBar>cargo build --release</CommandBar>
          <CommandBar>./target/release/tako doctor</CommandBar>
        </section>
        <section>
          <span>macOS</span>
          <h2>Build from source</h2>
          <p>A packaged macOS release is not currently declared. Release-tested packaging is not implied.</p>
          <CommandBar>cargo build --release</CommandBar>
          <CommandBar>./target/release/tako doctor</CommandBar>
        </section>
      </div>
      <aside className="truth-note">
        <h2>Why there is no <code>irm</code> or <code>curl</code> installer yet</h2>
        <p>Those commands must resolve to signed, versioned release artifacts. Publishing them before packages exist would create a fake installation path.</p>
      </aside>
    </main>
  );
}
