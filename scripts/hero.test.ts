import { test, expect } from "bun:test";
import { readFileSync, mkdtempSync, mkdirSync, writeFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { readHeroData, heroSvg, heroCopy, heroCss } from "./hero";

const root = join(import.meta.dirname, "..");
const data = readHeroData(root);

test("chart metrics agree with all raw Verified gold-file ranks", () => {
  expect(data.count).toBe(500);
  expect(data.tuningCount).toBe(300);
  for (const [method, values] of Object.entries({
    repomap: [48.8, 74.8, 83.0], semble: [34.0, 61.6, 71.0], bm25: [20.8, 45.4, 55.8],
  })) {
    expect([1, 5, 10].map(k => data.scores[method][`acc@${k}`])).toEqual(values);
  }
  expect(data.ndcg).toEqual({ repomap: 0.845, semble: 0.851 });
});

test("SVG labels, legend and tooltip values do not depend on color", () => {
  const svg = heroSvg(data);
  expect(svg).toContain('aria-labelledby="localization-title localization-desc"');
  expect(svg.match(/class="bar"/g)).toHaveLength(9);
  expect(svg.match(/tabindex="0"/g)).toHaveLength(9);
  expect(svg.match(/class="value"/g)).toHaveLength(3);
  expect(svg).toContain(".bar:focus .tooltip");
  expect(svg).toContain("forced-colors:active");
  expect(svg).toContain("rotate(45)");
  expect(svg).toContain("rotate(135)");
  expect(svg).not.toContain("<script");
  for (const method of ["repomap", "semble", "bm25"]) {
    for (const k of [1, 5, 10]) expect(svg).toContain(`Acc@${k}: ${data.scores[method][`acc@${k}`].toFixed(1)}%`);
  }
});

test("both copy surfaces include table, held-out protocol and NDCG limitation", () => {
  for (const docs of [false, true]) {
    const copy = heroCopy(data, docs);
    expect(copy).toContain("<caption>");
    expect(copy).toContain("scope=\"row\"");
    expect(copy).toContain("300 disjoint SWE-bench test instances");
    expect(copy).toContain("one evaluation on all 500 Verified instances");
    expect(copy).toContain("File retrieval, not issue resolution");
    expect(copy).toContain("0.845");
    expect(copy).toContain("0.851");
    expect(copy).toContain("semble wins there");
    expect(copy).toContain("results.json");
  }
});

test("committed chart and copy are byte-for-byte generated from their sources", () => {
  expect(readFileSync(join(root, "docs/public/img/localization.svg"), "utf8")).toBe(heroSvg(data));
  for (const file of ["README.md", "docs/index.md"]) {
    const current = readFileSync(join(root, file), "utf8");
    const body = current.split("<!-- localization-hero:start -->\n")[1].split("\n<!-- localization-hero:end -->")[0];
    expect(body).toBe(heroCopy(data, file.startsWith("docs/")));
  }
});

test("docs import the shared generated CSS instead of Vue-stripped inline styles", () => {
  expect(heroSvg(data)).toContain(`<style>\n${heroCss}</style>`);
  expect(heroSvg(data, false)).not.toContain("<style>");
  expect(heroCopy(data, true)).not.toContain("<style>");
  expect(readFileSync(join(root, "docs/.vitepress/theme/localization.css"), "utf8")).toBe(heroCss);
  expect(readFileSync(join(root, "docs/.vitepress/theme/index.ts"), "utf8")).toContain("import './localization.css'");
  expect(heroCss).toContain(".dark .localization-chart");
});

// Fixtures live in this private checkout, not another session's temp tree.
function changedFixture(change: (files: Record<string, any>) => void) {
  const dir = mkdtempSync(join(root, ".hero-test-"));
  const names = ["bench/localization/results.json", "bench/localization/manifest.json", "bench/localization/manifest-tune.json"];
  const files = Object.fromEntries(names.map(name => [name, JSON.parse(readFileSync(join(root, name), "utf8"))]));
  try {
    change(files);
    mkdirSync(join(dir, "bench/localization"), { recursive: true });
    mkdirSync(join(dir, "docs"));
    for (const name of names) writeFileSync(join(dir, name), JSON.stringify(files[name]));
    writeFileSync(join(dir, "docs/benchmarks.md"), readFileSync(join(root, "docs/benchmarks.md")));
    expect(() => readHeroData(dir)).toThrow();
  } finally {
    rmSync(dir, { recursive: true });
  }
}

test("reject summary drift rather than drawing unsupported values", () => {
  changedFixture(files => { files["bench/localization/results.json"].summary.repomap["acc@10"] = 99; });
});

test("reject incomplete or duplicated Verified results", () => {
  changedFixture(files => { files["bench/localization/results.json"].results.pop(); });
  changedFixture(files => {
    const rows = files["bench/localization/results.json"].results;
    rows[1] = rows[0];
  });
});

test("reject overlapping tuning IDs and repeated Verified evaluations", () => {
  changedFixture(files => {
    files["bench/localization/manifest-tune.json"].items[0].id = files["bench/localization/manifest.json"].items[0].id;
  });
  changedFixture(files => { files["bench/localization/results.json"].evaluation.verified_evaluations = 2; });
});
