const REGISTRY_URL = "/v1/registry.json";

const escapeHtml = (value = "") =>
  String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;");

const titleCase = (value = "") =>
  value
    .replaceAll("-", " ")
    .replace(/\b\w/g, (letter) => letter.toUpperCase());

function siteHeader() {
  const active = document.body.dataset.page || "";
  return `
    <header class="site-header">
      <div class="shell nav-inner">
        <a class="brand" href="/" aria-label="Takokit home">
          <img class="brand-mark" src="/assets/brand/takokit-mark.svg" alt="" />
          <img class="brand-word" src="/assets/brand/takokit-wordmark.svg" alt="Takokit" />
        </a>
        <nav class="nav-links" id="site-nav" aria-label="Primary navigation">
          <a ${active === "library" ? 'aria-current="page"' : ""} href="/library">Library</a>
          <a ${active === "docs" ? 'aria-current="page"' : ""} href="/docs">Docs</a>
          <a ${active === "download" ? 'aria-current="page"' : ""} href="/download">Download</a>
          <a href="https://github.com/amaansyed27/Takokit">GitHub</a>
        </nav>
        <a class="nav-action" href="/library">Explore models</a>
        <button class="nav-toggle" aria-controls="site-nav" aria-expanded="false" aria-label="Open menu"><span></span></button>
      </div>
    </header>`;
}

function siteFooter() {
  return `
    <footer class="site-footer">
      <div class="shell footer-main">
        <div class="footer-brand">
          <img src="/assets/brand/takokit-lockup.svg" alt="Takokit" />
          <p>A Rust-first local runtime for open speech and voice models, built by Dawnlight Labs.</p>
        </div>
        <div class="footer-group">
          <strong>Product</strong>
          <a href="/library">Model library</a>
          <a href="/download">Download</a>
          <a href="/docs">Documentation</a>
        </div>
        <div class="footer-group">
          <strong>Developers</strong>
          <a href="/v1/registry.json">Registry API</a>
          <a href="https://github.com/amaansyed27/Takokit">GitHub</a>
          <a href="https://github.com/amaansyed27/Takokit/blob/main/docs/registry.md">Registry protocol</a>
        </div>
        <div class="footer-group">
          <strong>Runtime</strong>
          <a href="/docs#storage">Storage</a>
          <a href="/docs#api">Local API</a>
          <a href="/docs#voice">Voice consent</a>
        </div>
      </div>
      <div class="shell footer-bottom">
        <span>© 2026 Dawnlight Labs</span>
        <span>Rust-first · local-first · open source</span>
      </div>
    </footer>`;
}

function mountChrome() {
  document.querySelectorAll("[data-site-header]").forEach((node) => {
    node.outerHTML = siteHeader();
  });
  document.querySelectorAll("[data-site-footer]").forEach((node) => {
    node.outerHTML = siteFooter();
  });
  const toggle = document.querySelector(".nav-toggle");
  const nav = document.querySelector(".nav-links");
  toggle?.addEventListener("click", () => {
    const open = nav.classList.toggle("is-open");
    toggle.setAttribute("aria-expanded", String(open));
  });
}

function mountCopyButtons() {
  document.querySelectorAll("[data-copy]").forEach((button) => {
    button.addEventListener("click", async () => {
      const original = button.textContent;
      await navigator.clipboard.writeText(button.dataset.copy);
      button.textContent = "Copied";
      window.setTimeout(() => (button.textContent = original), 1300);
    });
  });
}

function mountWaves() {
  document.querySelectorAll("[data-wave]").forEach((wave, waveIndex) => {
    wave.innerHTML = Array.from({ length: 24 }, (_, index) => {
      const height = 7 + ((index * 11 + waveIndex * 9) % 21);
      const delay = -((index % 7) * 0.14);
      return `<i style="--h:${height}px;--d:${delay}s"></i>`;
    }).join("");
  });
}

async function loadRegistry() {
  const response = await fetch(REGISTRY_URL, { headers: { accept: "application/json" } });
  if (!response.ok) throw new Error(`Registry returned ${response.status}`);
  const registry = await response.json();
  if (registry.schema_version !== 1 || !Array.isArray(registry.models)) {
    throw new Error("Registry schema is not supported");
  }
  return registry;
}

function defaultRelease(model) {
  return model.tags.find((release) => release.tag === model.default_tag) || model.tags[0];
}

function formatBytes(bytes) {
  if (!bytes) return "managed";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1000 && unit < units.length - 1) {
    value /= 1000;
    unit += 1;
  }
  return `${value >= 10 || unit === 0 ? value.toFixed(0) : value.toFixed(1)} ${units[unit]}`;
}

function glyph(model) {
  return model.name
    .split("-")
    .map((part) => part[0])
    .join("")
    .slice(0, 2)
    .toUpperCase();
}

function taskTags(tasks, limit = 3) {
  return tasks
    .slice(0, limit)
    .map((task) => `<span class="tag">${escapeHtml(titleCase(task))}</span>`)
    .join("");
}

function modelRow(model) {
  const release = defaultRelease(model);
  const runner = release.runner.replace("takokit-", "");
  return `
    <a class="model-row" href="/library/${encodeURIComponent(model.name)}">
      <span class="model-glyph">${escapeHtml(glyph(model))}</span>
      <div class="model-main">
        <strong>${escapeHtml(model.display_name)}</strong>
        <p>${escapeHtml(model.summary)}</p>
      </div>
      <div class="tag-list">${taskTags(model.tasks)}</div>
      <div class="model-meta">
        <strong>${escapeHtml(model.tags.length)} release${model.tags.length === 1 ? "" : "s"} · ${escapeHtml(formatBytes(release.size_bytes))}</strong>
        ${escapeHtml(runner)} · default ${escapeHtml(model.default_tag)}
      </div>
      <span class="row-arrow">→</span>
    </a>`;
}

async function mountHome() {
  const target = document.querySelector("#featured-models");
  if (!target) return;
  try {
    const registry = await loadRegistry();
    const preferred = ["kokoro", "whisper", "qwen3-tts", "chatterbox", "bark"];
    const featured = preferred
      .map((name) => registry.models.find((model) => model.name === name))
      .filter(Boolean);
    target.innerHTML = featured.map(modelRow).join("");
    document.querySelector("#family-count").textContent = registry.models.length;
    document.querySelector("#release-count").textContent = registry.models.reduce(
      (count, model) => count + model.tags.length,
      0,
    );
  } catch (error) {
    target.innerHTML = `<p class="empty-state">The registry is temporarily unavailable: ${escapeHtml(error.message)}</p>`;
  }
}

async function mountLibrary() {
  const target = document.querySelector("#library-results");
  if (!target) return;
  const search = document.querySelector("#library-search");
  const resultLine = document.querySelector("#result-line");
  let capability = "";
  let runtime = "";
  try {
    const registry = await loadRegistry();
    const render = () => {
      const query = search.value.trim().toLowerCase();
      const models = registry.models.filter((model) => {
        const releases = model.tags;
        const haystack = [
          model.name,
          model.display_name,
          model.summary,
          ...model.aliases,
          ...model.tasks,
          ...releases.flatMap((release) => [
            release.tag,
            release.runner,
            release.adapter || "",
            release.backend,
          ]),
        ]
          .join(" ")
          .toLowerCase();
        const capabilityMatch = !capability || model.tasks.includes(capability);
        const runtimeMatch =
          !runtime ||
          releases.some(
            (release) => release.backend.includes(runtime) || release.runner.includes(runtime),
          );
        return haystack.includes(query) && capabilityMatch && runtimeMatch;
      });
      resultLine.textContent = `${models.length} of ${registry.models.length} model families`;
      target.innerHTML = models.length
        ? models.map(modelRow).join("")
        : '<p class="empty-state">No model matches these filters.</p>';
    };
    search.addEventListener("input", render);
    document.querySelectorAll("[data-filter]").forEach((button) => {
      button.addEventListener("click", () => {
        document.querySelectorAll("[data-filter]").forEach((item) => item.classList.remove("is-active"));
        button.classList.add("is-active");
        capability = button.dataset.filter;
        render();
      });
    });
    document.querySelectorAll("[data-runner]").forEach((button) => {
      button.addEventListener("click", () => {
        document.querySelectorAll("[data-runner]").forEach((item) => item.classList.remove("is-active"));
        button.classList.add("is-active");
        runtime = button.dataset.runner;
        render();
      });
    });
    document.addEventListener("keydown", (event) => {
      if (event.key === "/" && document.activeElement !== search) {
        event.preventDefault();
        search.focus();
      }
    });
    render();
  } catch (error) {
    resultLine.textContent = "Registry unavailable";
    target.innerHTML = `<p class="empty-state">${escapeHtml(error.message)}</p>`;
  }
}

function releaseRows(model) {
  return model.tags
    .map(
      (release) => `
      <tr>
        <td><a href="?tag=${encodeURIComponent(release.tag)}">${escapeHtml(model.name)}:${escapeHtml(release.tag)}</a>${release.tag === model.default_tag ? ' <span class="tag tag-gold">default</span>' : ""}</td>
        <td>${escapeHtml(formatBytes(release.size_bytes))}</td>
        <td>${escapeHtml(release.runner.replace("takokit-", ""))}</td>
        <td>${escapeHtml(release.hardware.min_ram || "—")}</td>
        <td>${escapeHtml(release.digest.slice(0, 20))}…</td>
      </tr>`,
    )
    .join("");
}

function modelPage(model, selectedTag) {
  const release = model.tags.find((tag) => tag.tag === selectedTag) || defaultRelease(model);
  const reference = `${model.name}:${release.tag}`;
  const source = release.source.repository || release.source.provider;
  document.title = `${model.display_name}:${release.tag} · Takokit`;
  return `
    <section class="model-hero">
      <div class="shell model-head">
        <div>
          <p class="model-kicker">Takokit Library / ${escapeHtml(model.name)}</p>
          <h1>${escapeHtml(model.display_name)}<span style="color:var(--gold)">:${escapeHtml(release.tag)}</span></h1>
          <p class="model-summary">${escapeHtml(model.summary)}</p>
          <div class="tag-list">${taskTags(model.tasks, 8)}</div>
        </div>
        <div class="model-stats">
          <div class="model-stat"><span>Status</span><strong><i class="status-dot"></i>Published</strong></div>
          <div class="model-stat"><span>Default</span><strong>${escapeHtml(model.default_tag)}</strong></div>
          <div class="model-stat"><span>Size</span><strong>${escapeHtml(formatBytes(release.size_bytes))}</strong></div>
          <div class="model-stat"><span>Runner</span><strong>${escapeHtml(release.runner)}</strong></div>
          <div class="model-stat"><span>License</span><strong>${escapeHtml(release.license)}</strong></div>
        </div>
      </div>
    </section>
    <div class="shell model-content">
      <article>
        <div class="install-panel">
          <div class="command-tabs">
            <button class="command-tab is-active" data-command="tako pull ${escapeHtml(reference)}">Pull</button>
            <button class="command-tab" data-command="tako plan ${escapeHtml(reference)}">Plan</button>
            <button class="command-tab" data-command="tako library show ${escapeHtml(reference)}">Inspect</button>
          </div>
          <div class="install-command">
            <code id="model-command">tako pull ${escapeHtml(reference)}</code>
            <button class="copy-button" data-model-copy>Copy</button>
          </div>
        </div>
        <section class="content-section">
          <h2>Releases</h2>
          <table class="release-table">
            <thead><tr><th>Reference</th><th>Size</th><th>Runner</th><th>Minimum RAM</th><th>Manifest digest</th></tr></thead>
            <tbody>${releaseRows(model)}</tbody>
          </table>
        </section>
        <section class="content-section">
          <h2>About this family</h2>
          <p>${escapeHtml(model.summary)}</p>
          <p>
            The <code>${escapeHtml(reference)}</code> release resolves to stored install ID
            <code>${escapeHtml(release.target)}</code>. Takokit verifies the pinned manifest and
            required runtime before reporting it as ready.
          </p>
        </section>
        <section class="content-section">
          <h2>Runtime contract</h2>
          <p>
            Backend <code>${escapeHtml(release.backend)}</code> executes through
            <code>${escapeHtml(release.runner)}</code>${release.adapter ? ` with adapter <code>${escapeHtml(release.adapter)}</code>` : ""}.
            Hardware guidance: ${release.hardware.cpu ? "CPU supported" : "CPU not advertised"};
            ${release.hardware.gpu ? "GPU supported" : "GPU not required"}; minimum RAM
            ${escapeHtml(release.hardware.min_ram || "not declared")}.
          </p>
        </section>
      </article>
      <aside class="model-aside">
        <div class="aside-block">
          <strong>Canonical reference</strong>
          <p><code>library/${escapeHtml(reference)}</code></p>
        </div>
        <div class="aside-block">
          <strong>Legacy aliases</strong>
          ${(release.aliases.length ? release.aliases : model.aliases)
            .map((alias) => `<p><code>${escapeHtml(alias)}</code></p>`)
            .join("")}
        </div>
        <div class="aside-block">
          <strong>Source</strong>
          <p>${escapeHtml(source)}</p>
          <p>Revision-pinned manifest</p>
        </div>
        <div class="aside-block">
          <strong>Manifest</strong>
          <p>${escapeHtml(release.digest)}</p>
        </div>
      </aside>
    </div>`;
}

async function mountModel() {
  const target = document.querySelector("#model-page");
  if (!target) return;
  try {
    const registry = await loadRegistry();
    const route = decodeURIComponent(location.pathname.split("/").filter(Boolean).at(-1) || "");
    const [name, routeTag] = route.split(":");
    const selectedTag = new URLSearchParams(location.search).get("tag") || routeTag;
    const model = registry.models.find(
      (entry) => entry.name === name || entry.aliases.includes(name),
    );
    if (!model) throw new Error(`Model family “${name}” was not found`);
    target.innerHTML = modelPage(model, selectedTag);
    const command = document.querySelector("#model-command");
    document.querySelectorAll("[data-command]").forEach((button) => {
      button.addEventListener("click", () => {
        document.querySelectorAll("[data-command]").forEach((item) => item.classList.remove("is-active"));
        button.classList.add("is-active");
        command.textContent = button.dataset.command;
      });
    });
    document.querySelector("[data-model-copy]").addEventListener("click", async (event) => {
      await navigator.clipboard.writeText(command.textContent);
      event.currentTarget.textContent = "Copied";
      window.setTimeout(() => (event.currentTarget.textContent = "Copy"), 1300);
    });
  } catch (error) {
    target.innerHTML = `<section class="page-hero"><div class="shell"><p class="eyebrow">Library error</p><h1>Model unavailable.</h1><p class="lead">${escapeHtml(error.message)}</p><a class="button" href="/library">Back to library</a></div></section>`;
  }
}

mountChrome();
mountCopyButtons();
mountWaves();
mountHome();
mountLibrary();
mountModel();
