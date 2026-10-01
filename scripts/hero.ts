// Presentation only: committed benchmark results, never a new measurement.
import { readFileSync } from "node:fs";

export const methods = ["repomap", "semble", "bm25"] as const;
const labels = { repomap: "repomap", semble: "semble", bm25: "BM25" };
const ks = [1, 5, 10] as const;
export type HeroData = {
  count: number;
  tuningCount: number;
  scores: Record<string, Record<string, number>>;
  ndcg: { repomap: number; semble: number };
};

export function readHeroData(root = "."): HeroData {
  const read = (path: string) => readFileSync(`${root}/${path}`, "utf8");
  const result = JSON.parse(read("bench/localization/results.json"));
  const verified = JSON.parse(read("bench/localization/manifest.json"));
  const tuning = JSON.parse(read("bench/localization/manifest-tune.json"));
  const ids = new Set(verified.items.map((item: { id: string }) => item.id));
  if (ids.size !== verified.instances || tuning.items.length !== tuning.instances ||
      result.results.length !== verified.instances || result.revision !== verified.revision ||
      result.dataset !== verified.dataset || result.evaluation.verified_evaluations !== 1 ||
      tuning.items.some((item: { id: string }) => ids.has(item.id))) {
    throw new Error("Hero needs complete Verified results and a disjoint tuning split");
  }
  const seen = new Set<string>();
  for (const row of result.results) {
    if (!ids.has(row.id) || seen.has(row.id) || !row.gold.length) {
      throw new Error("Hero result IDs must match Verified exactly");
    }
    seen.add(row.id);
  }
  for (const method of methods) {
    const summary = result.summary[method];
    if (summary.scored !== verified.instances || summary.errors !== 0) {
      throw new Error(`Incomplete hero method: ${method}`);
    }
    for (const k of ks) {
      const correct = result.results.filter((row: { gold: string[]; methods: Record<string, { gold_ranks: Record<string, number | null> }> }) =>
        row.gold.every(file => {
          const rank = row.methods[method]?.gold_ranks[file];
          return typeof rank === "number" && rank >= 1 && rank <= k;
        })).length;
      const measured = Number((100 * correct / verified.instances).toFixed(1));
      if (summary[`acc@${k}`] !== measured) throw new Error(`Hero summary mismatch: ${method} Acc@${k}`);
    }
  }
  // The public search results are committed as a table, not a second JSON source.
  const search = read("docs/benchmarks.md").split("**Large repositories**")[0];
  const score = (label: string) => {
    const row = search.split("\n").find(line => line.startsWith(`| ${label} |`));
    const value = row ? Number(row.split("|")[2].trim()) : NaN;
    if (!Number.isFinite(value) || value < 0 || value > 1) throw new Error(`Missing public NDCG: ${label}`);
    return value;
  };
  return {
    count: verified.instances,
    tuningCount: tuning.instances,
    scores: result.summary,
    ndcg: { repomap: score("repomap, frozen defaults"), semble: score("semble, same runner") },
  };
}

const pct = (value: number) => `${value.toFixed(1)}%`;

export function heroSvg(data: HeroData): string {
  const grid = [0, 25, 50, 75, 100].map(value => {
    const x = 168 + value * 4.6;
    return `<path class="grid" d="M${x} 124V454"/><text class="secondary tick" x="${x}" y="482" text-anchor="middle">${value}</text>`;
  }).join("\n");
  const legend = methods.map((method, i) => `<rect class="${method}" x="${40 + i * 190}" y="92" width="16" height="16" rx="2"/><text x="${66 + i * 190}" y="107">${labels[method]}</text>`).join("\n");
  const groups = ks.map((k, group) => {
    const top = 152 + group * 110;
    const bars = methods.map((method, row) => {
      const value = data.scores[method][`acc@${k}`];
      const width = value * 4.6;
      const y = top + row * 27;
      const label = `${labels[method]} Acc@${k}: ${pct(value)}`;
      // Square baseline, 4px rounded data end; a 27px hit area for a 16px mark.
      return `<g class="bar" tabindex="0" role="img" aria-label="${label}">
<title>${label}</title><rect x="168" y="${y - 5}" width="460" height="27" fill="transparent"/>
<text x="148" y="${y + 14}" text-anchor="end">${labels[method]}</text>
<path class="${method}" d="M168 ${y}h${width - 4}q4 0 4 4v8q0 4 -4 4H168Z"/>
<text class="${method === "repomap" ? "value" : "tooltip"}" x="${178 + width}" y="${y + 14}">${pct(value)}</text></g>`;
    }).join("\n");
    return `<text class="group" x="40" y="${top - 14}">Acc@${k}</text>\n${bars}`;
  }).join("\n");
  return `<svg class="localization-chart" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 720 548" role="img" aria-labelledby="localization-title localization-desc" style="width:100%;max-width:720px;height:auto">
<title id="localization-title">SWE-bench Verified file localization</title>
<desc id="localization-desc">${data.count} issues, one issue-text search each. Acc@k counts issues where every gold fix file is among the top k files. ${methods.map(method => `${labels[method]}: ${ks.map(k => `Acc@${k} ${pct(data.scores[method][`acc@${k}`])}`).join(", ")}`).join(". ")}. Higher is better. This measures file retrieval, not issue resolution.</desc>
<style>
.localization-chart{--surface:#fcfcfb;--ink:#0b0b0b;--secondary:#52514e;--grid:#e8e8e4;--repomap:#2a78d6;--semble:#eb6834;--bm25:#1baf7a;font-family:ui-sans-serif,system-ui,-apple-system,Segoe UI,sans-serif;font-size:19px}
@media(prefers-color-scheme:dark){.localization-chart{--surface:#1a1a19;--ink:#ffffff;--secondary:#c3c2b7;--grid:#333330;--repomap:#3987e5;--semble:#d95926;--bm25:#199e70}}
.localization-chart text{fill:var(--ink)}.localization-chart .secondary{fill:var(--secondary)}.localization-chart .grid{stroke:var(--grid);stroke-width:1;fill:none}.localization-chart .repomap{fill:var(--repomap)}.localization-chart .semble{fill:var(--semble)}.localization-chart .bm25{fill:var(--bm25)}.localization-chart .group,.localization-chart .value{font-weight:600}.localization-chart .tick{font-size:17px}.localization-chart .tooltip{opacity:0}.localization-chart .bar:hover .tooltip,.localization-chart .bar:focus .tooltip{opacity:1}.localization-chart .bar:focus{outline:none}.localization-chart .bar:hover path,.localization-chart .bar:focus path{filter:brightness(1.12)}
@media print,(forced-colors:active){.localization-chart .semble{fill:url(#texture-45)}.localization-chart .bm25{fill:url(#texture-135)}.localization-chart .repomap{fill:var(--ink)}.localization-chart .tooltip{opacity:1}}
</style>
<defs><pattern id="texture-45" width="6" height="6" patternUnits="userSpaceOnUse" patternTransform="rotate(45)"><rect width="6" height="6" fill="var(--surface)"/><path d="M0 0V6" stroke="var(--ink)" stroke-width="2"/></pattern><pattern id="texture-135" width="6" height="6" patternUnits="userSpaceOnUse" patternTransform="rotate(135)"><rect width="6" height="6" fill="var(--surface)"/><path d="M0 0V6" stroke="var(--ink)" stroke-width="2"/></pattern></defs>
<rect width="720" height="548" rx="16" fill="var(--surface)"/>
<text x="40" y="46" font-size="27" font-weight="600">Find the files behind an issue</text>
<text class="secondary" x="40" y="75">SWE-bench Verified · ${data.count} issues · one search</text>
${legend}\n${grid}\n${groups}
<text class="secondary" x="168" y="512" font-size="17">Issues with every fix file in the top k (%)</text>
</svg>\n`;
}

export function heroCopy(data: HeroData, docs = false): string {
  const benchmarks = "https://sylphxai.github.io/repomap/benchmarks";
  const table = methods.map(method => `<tr><th scope="row">${labels[method]}</th>${ks.map(k => `<td>${pct(data.scores[method][`acc@${k}`])}</td>`).join("")}</tr>`).join("\n");
  const image = docs ? heroSvg(data) : `<img src="docs/public/img/localization.svg" width="720" alt="SWE-bench Verified: repomap finds every fix file in the top 10 for ${pct(data.scores.repomap["acc@10"])} of ${data.count} issues; semble ${pct(data.scores.semble["acc@10"])}, BM25 ${pct(data.scores.bm25["acc@10"])}. Full Acc@1, 5 and 10 values follow in the table.">`;
  return `<p>One issue-text search finds every fix file in the top 10 for <strong>${pct(data.scores.repomap["acc@10"])} of SWE-bench Verified issues</strong> (semble: ${pct(data.scores.semble["acc@10"])}). File retrieval, not issue resolution.</p>
${image}
<details><summary>Chart values and evaluation protocol</summary>
<table><caption>SWE-bench Verified: every gold fix file in the top k files</caption><thead><tr><th scope="col">Method</th><th scope="col">Acc@1</th><th scope="col">Acc@5</th><th scope="col">Acc@10</th></tr></thead><tbody>${table}</tbody></table>
<p>Defaults selected on ${data.tuningCount} disjoint SWE-bench test instances, excluding every Verified ID, then frozen before one evaluation on all ${data.count} Verified instances. One search per issue, at its base commit; gold files are used only for scoring. All three methods scored ${data.count}/${data.count}, with zero errors.</p>
<p><a href="${benchmarks}#file-localization">Split definitions, pinned revisions and full results</a> · <a href="https://github.com/SylphxAI/repomap/blob/main/bench/localization/results.json">Committed chart data</a></p>
</details>
<p>Trade-off: on semble's public code-search set, NDCG@10 is <strong>${data.ndcg.repomap.toFixed(3)}</strong> versus semble's <strong>${data.ndcg.semble.toFixed(3)}</strong>; semble wins there. This is not a claim of better search on every task. <a href="${benchmarks}#search-quality">Measured search quality and limitations</a>.</p>`;
}
