const API = "/v1/registry.json";

const formatBytes = (bytes) => {
  if (!bytes) return "runtime managed";
  const units = ["B", "KiB", "MiB", "GiB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value.toFixed(unit > 1 ? 1 : 0)} ${units[unit]}`;
};

const escapeHtml = (value = "") =>
  value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");

async function registry() {
  const response = await fetch(API, { headers: { accept: "application/json" } });
  if (!response.ok) throw new Error(`Registry returned ${response.status}`);
  return response.json();
}

function modelCard(model) {
  const defaultRelease =
    model.tags.find((release) => release.tag === model.default_tag) ??
    model.tags[0];
  return `<a class="model-card" href="/library/${encodeURIComponent(model.name)}">
    <div class="card-meta">
      <span>${escapeHtml(defaultRelease.kind.toUpperCase())}</span>
      <span>${formatBytes(defaultRelease.size_bytes)}</span>
    </div>
    <h3>${escapeHtml(model.name)}</h3>
    <p>${escapeHtml(model.summary)}</p>
    <div class="tag-row">
      ${model.tasks
        .slice(0, 3)
        .map((task) => `<span class="tag">${escapeHtml(task)}</span>`)
        .join("")}
      <span class="tag gold">${model.tags.length} tag${
        model.tags.length === 1 ? "" : "s"
      }</span>
    </div>
  </a>`;
}

function bindCopyButtons(root = document) {
  root.querySelectorAll("[data-copy]").forEach((button) => {
    button.addEventListener("click", async () => {
      await navigator.clipboard.writeText(button.dataset.copy);
      const previous = button.textContent;
      button.textContent = "Copied";
      setTimeout(() => {
        button.textContent = previous;
      }, 1200);
    });
  });
}

async function renderHome() {
  const grid = document.querySelector("#featured-models");
  if (!grid) return;
  try {
    const index = await registry();
    const featured = ["kokoro", "whisper", "qwen3-tts", "chatterbox", "openvoice", "parakeet"]
      .map((name) => index.models.find((model) => model.name === name))
      .filter(Boolean);
    grid.innerHTML = featured.map(modelCard).join("");
    document.querySelector("#family-count").textContent = index.models.length;
    document.querySelector("#release-count").textContent = index.models.reduce(
      (sum, model) => sum + model.tags.length,
      0,
    );
  } catch (error) {
    grid.innerHTML = `<div class="empty">The registry is temporarily unavailable. ${escapeHtml(
      error.message,
    )}</div>`;
  }
}

async function renderLibrary() {
  const grid = document.querySelector("#library-grid");
  if (!grid) return;
  const search = document.querySelector("#library-search");
  const filter = document.querySelector("#task-filter");
  const resultLine = document.querySelector("#result-line");
  try {
    const index = await registry();
    const tasks = [...new Set(index.models.flatMap((model) => model.tasks))].sort();
    filter.insertAdjacentHTML(
      "beforeend",
      tasks.map((task) => `<option value="${task}">${task}</option>`).join(""),
    );
    const update = () => {
      const query = search.value.trim().toLowerCase();
      const task = filter.value;
      const models = index.models.filter(
        (model) =>
          (!query ||
            model.name.includes(query) ||
            model.display_name.toLowerCase().includes(query) ||
            model.summary.toLowerCase().includes(query) ||
            model.tags.some((release) => release.tag.includes(query))) &&
          (!task || model.tasks.includes(task)),
      );
      resultLine.textContent = `${models.length} model families · ${models.reduce(
        (sum, model) => sum + model.tags.length,
        0,
      )} releases`;
      grid.innerHTML = models.length
        ? models.map(modelCard).join("")
        : '<div class="empty">No models match this filter.</div>';
    };
    search.addEventListener("input", update);
    filter.addEventListener("change", update);
    update();
  } catch (error) {
    grid.innerHTML = `<div class="empty">Could not load the registry. ${escapeHtml(
      error.message,
    )}</div>`;
  }
}

function manifestRows(model, release) {
  const hardware = [
    release.hardware.cpu ? "CPU" : null,
    release.hardware.gpu ? "GPU" : null,
    release.hardware.min_ram ? `${release.hardware.min_ram} RAM` : null,
    release.hardware.min_vram ? `${release.hardware.min_vram} VRAM` : null,
  ]
    .filter(Boolean)
    .join(" · ");
  return [
    ["Default tag", model.default_tag],
    ["Runner", release.runner],
    ["Adapter", release.adapter ?? "none"],
    ["License", release.license],
    ["Hardware", hardware],
    ["Upstream", release.source.repository ?? release.source.provider],
  ]
    .map(
      ([label, value]) =>
        `<div class="manifest-row"><span>${escapeHtml(label)}</span><span>${escapeHtml(
          value,
        )}</span></div>`,
    )
    .join("");
}

async function renderModel() {
  const root = document.querySelector("#model-detail");
  if (!root) return;
  const name =
    new URLSearchParams(location.search).get("name") ||
    decodeURIComponent(location.pathname.split("/").filter(Boolean).at(-1) ?? "");
  try {
    const index = await registry();
    const model = index.models.find((candidate) => candidate.name === name);
    if (!model) {
      root.innerHTML = '<div class="empty">This model family was not found.</div>';
      return;
    }
    const defaultRelease =
      model.tags.find((release) => release.tag === model.default_tag) ??
      model.tags[0];
    document.title = `${model.name} · Takokit Library`;
    root.innerHTML = `
      <section class="model-hero">
        <div>
          <p class="eyebrow">Takokit Library / ${escapeHtml(defaultRelease.kind)}</p>
          <h1>${escapeHtml(model.name)}</h1>
          <p class="hero-copy">${escapeHtml(model.summary)}</p>
          <div class="copy-row">
            <code>tako pull ${escapeHtml(model.name)}</code>
            <button class="copy-button" data-copy="tako pull ${escapeHtml(
              model.name,
            )}">Copy</button>
          </div>
          <div class="tag-row" style="margin-top: 18px">
            ${model.tasks
              .map((task) => `<span class="tag">${escapeHtml(task)}</span>`)
              .join("")}
          </div>
        </div>
        <aside class="manifest-panel">${manifestRows(model, defaultRelease)}</aside>
      </section>
      <section class="section">
        <p class="eyebrow">Immutable releases</p>
        <h2>Available tags</h2>
        <table class="release-table">
          <thead><tr><th>Tag</th><th>Pull command</th><th>Model data</th><th>Manifest digest</th></tr></thead>
          <tbody>
            ${model.tags
              .map(
                (release) => `<tr>
                  <td><strong>${escapeHtml(release.tag)}</strong>${
                    release.tag === model.default_tag
                      ? '<br><span class="tag gold">default</span>'
                      : ""
                  }</td>
                  <td><code>tako pull ${escapeHtml(model.name)}:${escapeHtml(
                    release.tag,
                  )}</code></td>
                  <td>${formatBytes(release.size_bytes)}</td>
                  <td><span class="digest" title="${escapeHtml(
                    release.digest,
                  )}">${escapeHtml(release.digest)}</span></td>
                </tr>`,
              )
              .join("")}
          </tbody>
        </table>
      </section>`;
    bindCopyButtons(root);
  } catch (error) {
    root.innerHTML = `<div class="empty">Could not load the model manifest. ${escapeHtml(
      error.message,
    )}</div>`;
  }
}

bindCopyButtons();
renderHome();
renderLibrary();
renderModel();
