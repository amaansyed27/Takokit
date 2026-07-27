import { useEffect, useMemo, useState } from "react";
import { Footer, Header, CopyCommand } from "./components/Chrome";
import { RouteLink, useRoute } from "./router";
import { defaultRelease, formatBytes, getRegistry } from "./registry";

function ModelList({ models }) {
  if (!models.length) return <div className="empty">No models match this search.</div>;
  return <div className="model-list">{models.map((model) => {
    const release = defaultRelease(model);
    return <RouteLink href={`/library/${encodeURIComponent(model.name)}`} className="model-row" key={model.name}>
      <div><strong>{model.display_name}</strong><p>{model.summary}</p></div>
      <div className="task-list">{model.tasks.slice(0, 3).map((task) => <span key={task}>{task}</span>)}</div>
      <div className="model-facts"><span>{model.tags.length} tags</span><span>{formatBytes(release.size_bytes)}</span></div>
      <b>→</b>
    </RouteLink>;
  })}</div>;
}

function Home() {
  const [models, setModels] = useState([]);
  useEffect(() => { getRegistry().then((r) => setModels(r.models.slice(0, 5))).catch(() => {}); }, []);
  return <>
    <section className="hero shell">
      <div className="hero-copy"><p className="kicker">Local voice runtime</p>
        <h1>Run open voice models on your machine.</h1>
        <p>Pull, run, clone, transcribe, and serve speech models without rebuilding a fragile Python stack for every project.</p>
        <CopyCommand>tako pull kokoro</CopyCommand>
        <div className="actions"><RouteLink href="/download" className="primary">Download</RouteLink><RouteLink href="/library">Browse models</RouteLink></div>
      </div>
      <div className="hero-art"><img src="/brand/takokit-mark.svg" alt="Takokit abstract mark" /><div className="signal"><i/><i/><i/><i/><i/><i/><i/></div></div>
    </section>
    <section className="shell strip"><span>Rust-first runtime</span><span>CLI · TUI · GUI · API</span><span>Local models and outputs</span></section>
    <section className="shell section"><div className="section-head"><div><p className="kicker">Model library</p><h2>Start with a model.</h2></div><RouteLink href="/library">View all →</RouteLink></div><ModelList models={models} /></section>
    <section className="shell split section"><div><p className="kicker">One runtime</p><h2>Different models. One predictable contract.</h2></div><div className="steps"><article><b>01</b><h3>Pull</h3><p>Resolve a tested tag and install its verified model, runner, and dependencies.</p></article><article><b>02</b><h3>Run</h3><p>Use the same model from the CLI, desktop UI, TUI, or local HTTP API.</p></article><article><b>03</b><h3>Reuse</h3><p>Share compatible runtimes and caches without duplicating multi-gigabyte packages.</p></article></div></section>
  </>;
}

function Library() {
  const [models, setModels] = useState([]); const [query, setQuery] = useState(""); const [task, setTask] = useState("");
  useEffect(() => { getRegistry().then((r) => setModels(r.models)).catch(() => {}); }, []);
  const filtered = useMemo(() => models.filter((model) => {
    const haystack = [model.name, model.display_name, model.summary, ...model.aliases, ...model.tasks].join(" ").toLowerCase();
    return haystack.includes(query.toLowerCase()) && (!task || model.tasks.includes(task));
  }), [models, query, task]);
  const tasks = ["", "tts", "stt", "voice-cloning", "voice-conversion", "voice-training", "live-audio"];
  return <main className="shell page"><header className="page-head"><p className="kicker">Takokit library</p><h1>Models</h1><p>Curated model families and immutable releases supported by the Takokit runtime.</p></header>
    <div className="library-tools"><input type="search" value={query} onChange={(e) => setQuery(e.target.value)} placeholder="Search models" aria-label="Search models" />
      <div className="filters">{tasks.map((item) => <button className={task === item ? "active" : ""} onClick={() => setTask(item)} key={item || "all"}>{item || "all"}</button>)}</div></div>
    <p className="count">{filtered.length} model families</p><ModelList models={filtered} /></main>;
}

function Model({ name, tag }) {
  const [model, setModel] = useState();
  useEffect(() => { getRegistry().then((r) => setModel(r.models.find((m) => m.name === name || m.aliases.includes(name)) || null)); }, [name]);
  if (model === undefined) return <div className="shell page">Loading model…</div>;
  if (model === null) return <div className="shell page"><h1>Model not found</h1><RouteLink href="/library">Back to models</RouteLink></div>;
  const release = model.tags.find((item) => item.tag === tag) || defaultRelease(model);
  const ref = `${model.name}:${release.tag}`;
  return <main className="shell page model-page"><header className="model-head"><div><p className="kicker">Library / {model.name}</p><h1>{model.display_name}<small>:{release.tag}</small></h1><p>{model.summary}</p><CopyCommand>tako pull {ref}</CopyCommand></div>
    <dl><div><dt>Size</dt><dd>{formatBytes(release.size_bytes)}</dd></div><div><dt>Runner</dt><dd>{release.runner}</dd></div><div><dt>License</dt><dd>{release.license}</dd></div><div><dt>Minimum RAM</dt><dd>{release.hardware.min_ram || "Not declared"}</dd></div></dl></header>
    <section className="section"><div className="section-head"><div><p className="kicker">Available variants</p><h2>Tags</h2></div></div>
      <div className="tags-table">{model.tags.map((item) => <RouteLink key={item.tag} href={`/library/${model.name}:${item.tag}`}><strong>{model.name}:{item.tag}</strong><span>{formatBytes(item.size_bytes)}</span><span>{item.runner.replace("takokit-", "")}</span><b>→</b></RouteLink>)}</div></section>
    <section className="prose"><h2>Run this model</h2><CopyCommand>tako plan {ref}</CopyCommand><CopyCommand>tako library show {ref}</CopyCommand><p>Takokit resolves this reference to the pinned install target <code>{release.target}</code> and verifies its runtime before reporting it ready.</p></section>
  </main>;
}

function Docs() {
  const sections = [["quickstart","Quickstart"],["models","Models"],["storage","Storage"],["api","Local API"]];
  return <main className="shell page docs"><aside>{sections.map(([id,title]) => <a href={`#${id}`} key={id}>{title}</a>)}</aside><article><header className="page-head"><p className="kicker">Documentation</p><h1>Build with local voice.</h1></header>
    <section id="quickstart"><h2>Quickstart</h2><p>Build the current public beta from source until signed packages are published.</p><CopyCommand>cargo build --release</CopyCommand><CopyCommand>tako pull kokoro</CopyCommand><CopyCommand>tako speak "Hello from Takokit" --model kokoro</CopyCommand></section>
    <section id="models"><h2>Model references</h2><p>Use a family default or select an immutable tag.</p><pre>kokoro{"\n"}whisper:small{"\n"}qwen3-tts:0.6b-base</pre></section>
    <section id="storage"><h2>Storage</h2><pre>~/.takokit/{"\n"}  models/{"\n"}  runners/{"\n"}  blobs/{"\n"}  cache/{"\n"}  manifests/{"\n\n"}project/.tako/sessions/</pre></section>
    <section id="api"><h2>Local API</h2><pre>GET  /health{"\n"}GET  /v1/models{"\n"}POST /v1/audio/speech{"\n"}POST /v1/audio/transcriptions</pre></section>
  </article></main>;
}

function Download() {
  return <main className="shell page"><header className="page-head"><p className="kicker">Public beta</p><h1>Download Takokit</h1><p>Takokit is still source-distributed. Installer commands will appear here only after signed GitHub release packages exist.</p></header>
    <section className="download-grid"><article><span>Windows</span><h2>PowerShell</h2><p>Build the Rust workspace and run the generated executable.</p><CopyCommand>cargo build --release</CopyCommand><CopyCommand>.\target\release\tako.exe doctor</CopyCommand></article>
    <article><span>macOS / Linux</span><h2>Terminal</h2><p>Build with the stable Rust toolchain.</p><CopyCommand>cargo build --release</CopyCommand><CopyCommand>./target/release/tako doctor</CopyCommand></article></section>
    <section className="notice"><strong>Why no curl or irm command yet?</strong><p>Those commands must install a real, versioned release safely. Publishing a decorative script before release artifacts exist would create a broken installation path.</p></section>
  </main>;
}

function resolve(route) {
  const path = route.split("?")[0].replace(/\/$/, "") || "/";
  if (path === "/") return <Home />;
  if (path === "/library") return <Library />;
  if (path === "/docs" || path.startsWith("/docs/")) return <Docs />;
  if (path === "/download") return <Download />;
  if (path.startsWith("/library/")) {
    const ref = decodeURIComponent(path.slice("/library/".length));
    const split = ref.indexOf(":");
    return <Model name={split === -1 ? ref : ref.slice(0, split)} tag={split === -1 ? undefined : ref.slice(split + 1)} />;
  }
  return <main className="shell page"><h1>Page not found</h1></main>;
}

export default function App() {
  const route = useRoute();
  return <><Header />{resolve(route)}<Footer /></>;
}
