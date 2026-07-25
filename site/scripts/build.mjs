import { cp, mkdir, rm } from "node:fs/promises";

const output = new URL("../dist/", import.meta.url);
await rm(output, { recursive: true, force: true });
await mkdir(output, { recursive: true });
for (const entry of [
  "index.html",
  "model.html",
  "library",
  "docs",
  "download",
  "assets",
  "api",
  "vercel.json",
]) {
  await cp(new URL(`../${entry}`, import.meta.url), new URL(`../dist/${entry}`, import.meta.url), {
    recursive: true,
  });
}
console.log("Takokit Library static build ready in site/dist");
