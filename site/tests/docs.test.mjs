import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import {
  DOC_GROUPS,
  DOC_ORDER,
  DOCS,
  adjacentDocs,
  findDocGroup,
} from "../src/docs/content.js";

const root = new URL("../", import.meta.url);

async function source(path) {
  return readFile(new URL(path, root), "utf8");
}

test("documentation uses sidebar, article, on-page navigation, and pagination", async () => {
  const page = await source("src/pages/DocsPage.jsx");
  for (const component of [
    "DocsSidebar",
    "DocsCodeBlock",
    "DocsTableOfContents",
    "DocsPager",
  ]) {
    assert.match(page, new RegExp(component));
  }
  assert.match(page, /docs-shell/);
  assert.match(page, /docs-content__section/);
  assert.doesNotMatch(page, /CommandBar/);
});

test("documentation navigation covers every document exactly once", () => {
  const ids = DOC_GROUPS.flatMap((group) => group.pages.map(([id]) => id));
  assert.equal(ids.length, Object.keys(DOCS).length);
  assert.equal(new Set(ids).size, ids.length);
  assert.deepEqual(ids, DOC_ORDER.map((item) => item.id));

  for (const id of ids) {
    assert.ok(DOCS[id], `missing document ${id}`);
    assert.ok(findDocGroup(id), `missing group for ${id}`);
    assert.ok(DOCS[id].sections.length > 0, `missing sections for ${id}`);
    const sectionIds = DOCS[id].sections.map((section) => section.id);
    assert.equal(new Set(sectionIds).size, sectionIds.length, `duplicate section id in ${id}`);
  }
});

test("documentation pagination follows the declared navigation order", () => {
  const first = adjacentDocs(DOC_ORDER[0].id);
  assert.equal(first.previous, null);
  assert.equal(first.next.id, DOC_ORDER[1].id);

  const last = adjacentDocs(DOC_ORDER.at(-1).id);
  assert.equal(last.next, null);
  assert.equal(last.previous.id, DOC_ORDER.at(-2).id);
});

test("documentation styles use a compact responsive three-column reading layout", async () => {
  const styles = await source("src/styles/docs.css");
  assert.match(styles, /grid-template-columns: 240px minmax\(0, 760px\) 190px/);
  assert.match(styles, /docs-toc/);
  assert.match(styles, /docs-sidebar__toggle/);
  assert.match(styles, /max-width: 1120px/);
  assert.match(styles, /max-width: 820px/);
  assert.match(styles, /white-space: pre-wrap/);
});

test("documentation code blocks provide contained copy feedback", async () => {
  const code = await source("src/components/docs/DocsCodeBlock.jsx");
  assert.match(code, /copyText/);
  assert.match(code, /Copied/);
  assert.match(code, /Copy failed/);
  assert.match(code, /<pre tabIndex=\{0\}>/);
});
