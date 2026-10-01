# repomap brand

## Shared generator

CI uses the shared brand action pinned to `a6c81b4bcda66bf624f0a68e25e63b3c0ed043eb`.
The masters, tokens, pixel grids and provenance remain in this repository;
existing assets are unchanged by moving the generator. To regenerate locally,
prepare the script from the same pin (run from the repository root):

```sh
BRAND_SCRIPT="$(mktemp)"
curl --fail --location --output "$BRAND_SCRIPT" \
  "https://raw.githubusercontent.com/SylphxAI/.github/a6c81b4bcda66bf624f0a68e25e63b3c0ed043eb/.github/actions/brand/build.py"
python3 -m pip install pillow numpy resvg-py
python3 "$BRAND_SCRIPT" --brand-dir "$PWD/brand" --root "$PWD"
python3 "$BRAND_SCRIPT" --brand-dir "$PWD/brand" --root "$PWD" --check
rm "$BRAND_SCRIPT"
```

Add `--resnap` only when intentionally redrawing the small favicon grids.
Check mode needs only Python 3 and does not regenerate files. The commands below
assume `BRAND_SCRIPT` points to this pinned script. Generated-file comments that
name `brand/build.py` describe the historical generator; they are preserved to
keep the asset bytes and hashes unchanged.

This folder is the source of truth for the repomap mark, its icons, its colours and its
type. Every surface copies from here; nothing redraws the mark. Rebuild every derived
file with:

```bash
python3 "$BRAND_SCRIPT" --brand-dir "$PWD/brand" --root "$PWD"            # needs pillow, numpy and resvg-py
python3 "$BRAND_SCRIPT" --brand-dir "$PWD/brand" --root "$PWD" --resnap   # also redraw the 16 and 32 px grids
python3 "$BRAND_SCRIPT" --brand-dir "$PWD/brand" --root "$PWD" --check    # verify the hashes and the surface copies (CI runs this)
```

## Name

`repomap`, lower case, always, including at the start of a sentence, in the docs title,
the npm package (`@sylphx/repomap`), the MCP Registry name
(`io.github.SylphxAI/repomap`), the binary and the Docker image. Upper case `REPOMAP_`
appears only where an environment variable has to be upper case (`REPOMAP_ROOT`,
`REPOMAP_TOKEN`, `REPOMAP_CACHE_DIR`, `REPOMAP_EMBED`, and CI's `REPOMAP_BIN`).

There is no drawn wordmark and no local-script name: the product ships English only, and
the name is set in text beside the symbol, in the docs header, the graph UI and the
README. Capitals therefore appear nowhere in the mark.

Operated by Sylphx Limited. The GitHub organisation is SylphxAI, `LICENSE` reads
`Copyright (c) 2026 SylphxAI` and the docs footer reads `© Sylphx`. The operator line is
never part of the brand.

## Files

| Need | File |
|---|---|
| The mark on a dark ground (docs header, README, slides) | `svg/repomap-symbol-on-dark.svg` |
| The mark on a light ground | `svg/repomap-symbol.svg` |
| One ink: engraving, embroidery, a stamp, a single-colour print | `svg/repomap-symbol-black.svg`, `svg/repomap-symbol-white.svg` |
| App icon master | `svg/repomap-app-icon.svg` |
| Square icon for a surface that applies its own mask (apple-touch-icon) | `svg/repomap-app-icon-square.svg` |
| Ready-made app icons | `app-icon/icon-192.png`, `app-icon/icon-512.png`, `app-icon/icon-1024.png`, `app-icon/apple-touch-icon-180.png` |
| Browser tab and bookmarks | `favicon/favicon.svg` (the 32 px grid), `favicon/favicon.ico` (16, 32 and 48 px) |
| Small PNGs, and the editable grids they come from | `favicon/favicon-16.png`, `favicon/favicon-32.png`, `favicon/favicon-48.png`, `favicon/grid-16.txt`, `favicon/grid-32.txt` |
| Colours and type as data | `tokens.json` |
| Colours and type as CSS | `tokens.css` |
| Where every file came from | `provenance.json` |
| The construction spec and the generator | `brand.json`, [shared generator](https://github.com/SylphxAI/.github/tree/a6c81b4bcda66bf624f0a68e25e63b3c0ed043eb/.github/actions/brand) |

`brand.json` at the repository root is a different file: it holds the public-copy
oneliner that `scripts/copy.ts` syncs into the README, the docs tagline, `package.json`
and `server.json`.

There is no maskable icon yet: the product ships no web manifest to point one at. Add
both when it does.

## Colours

| Token | Hex | Use |
|---|---|---|
| `ground` | `#06080C` | the dark ground: page, graph canvas, app-icon background |
| `ground-alt` | `#0B0E14` | a raised section on the ground |
| `ground-elv` | `#0E1118` | an elevated panel |
| `ground-soft` | `#11151D` | a soft well inside a panel |
| `accent` | `#8AA4FF` | the primary accent: links, buttons, focus |
| `accent-hover` | `#A3B8FF` | the accent on hover |
| `accent-active` | `#6D86E8` | the accent when active |
| `node-blue` | `#8AA4FF` | the blue node of the mark |
| `node-pink` | `#FF7EB6` | the pink node of the mark |
| `node-green` | `#42D6A4` | the green node of the mark |
| `node-amber` | `#FFB454` | the amber node of the mark |

The four node colours are the mark, and the hero gradient on the docs home runs
`node-blue` → `node-pink` → `node-amber`. The edges between the nodes are not a token:
they are the ground, 35% of the opposite colour, so `rgba(255,255,255,.35)` on a dark
ground and `rgba(6,8,12,.35)` on a light one. Over `ground` they composite to `#5D5E61`,
which is the grey in the app icon and in the favicon grids.

## Type

| Token | Value | Where |
|---|---|---|
| `sans` | `ui-sans-serif, system-ui, -apple-system, Segoe UI, Inter, Roboto, Helvetica Neue, Arial, sans-serif` | the graph UI, declared in `ui/src/style.css` |
| `mono` | `ui-monospace, SF Mono, JetBrains Mono, Cascadia Code, Menlo, Consolas, monospace` | code, file paths and line numbers |

No font files are hosted here or in the repository. The product UI uses the reader's own
system fonts; the docs site uses the type that ships inside VitePress's default theme
(Inter, over a system-sans fallback).

## Small sizes

16 and 32 px are pixel-snapped from `svg/repomap-app-icon.svg`. The master is rendered at
8× and every sample takes the nearest colour in `brand.json`'s palette; a pixel stays
empty when fewer than half of its samples are filled (`snap_threshold` 0.5).

- The grids are `favicon/grid-16.txt` and `favicon/grid-32.txt`. Each row is one pixel
  row and each letter indexes the palette in the file's header, so a person can edit a
  pixel by hand.
- Hand edits are kept: [shared generator](https://github.com/SylphxAI/.github/tree/a6c81b4bcda66bf624f0a68e25e63b3c0ed043eb/.github/actions/brand) draws the PNGs, the ICO and `favicon.svg` from the grid
  until `--resnap` redraws it from the master.
- `favicon.svg` is the 32 px grid drawn as pixel rectangles. `favicon.ico` holds the 16
  and 32 px grids and a 48 px render of the vector. From 48 px up, every file is the
  vector.

## Clear space and minimum size

Not yet specified: this repository has no design document. What the files carry on their
own: the symbol masters are tight to the artwork, so clear space is added where the mark
is placed, and the app icon holds a 15% margin on each side (the symbol is 70% of the
tile). Below 16 px, use the favicon files rather than the symbol.

## Do

- Use these files as they are, scaled evenly. Nothing here needs to be redrawn or
  retyped.
- On a dark ground use `repomap-symbol-on-dark.svg`; on a light ground use
  `repomap-symbol.svg`. Each one's edges are invisible on the other's ground.
- On one-ink work — a stamp, an engraving, a single-colour print, a photo — use
  `-black` or `-white`.
- Set the name beside the symbol in text, lower case: `repomap`.

## Don't

- Don't recolour a node or swap two node colours. The four hues are the mark.
- Don't stretch, squash, rotate or skew a file.
- Don't add shadows, glows, bevels, outlines or textures.
- Don't put the on-dark symbol on a white or light background, or the light-ground symbol
  on the dark ground: the edges disappear and the mark reads as loose dots.
- Don't set the name in capitals. There is no drawn wordmark, and `repomap` carries none.

## Surfaces

| Surface | Copies from |
|---|---|
| Docs header logo (`docs/.vitepress/config.ts`, `themeConfig.logo`) | `docs/public/logo.svg` ← `svg/repomap-symbol-on-dark.svg` |
| The `.mcpb` bundle icon (`.github/workflows/release.yml`, `mcpb-icon`) | `docs/public/icon.png` ← `app-icon/icon-512.png` |
| Docs favicon | `docs/public/favicon.ico` ← `favicon/favicon.ico`; `docs/public/favicon.svg` ← `favicon/favicon.svg` |
| Docs theme colours | `docs/.vitepress/theme/custom.css` imports `brand/tokens.css` and reads `--brand-color-*` |
| Docs browser-chrome colour | `docs/.vitepress/config.ts` reads `brand/tokens.json` |

Surfaces still to move, each of which still draws or hard-codes the brand itself:

| Surface | What it still carries |
|---|---|
| `crates/repomap/assets/index.html` | the mark inline as `<svg class="mark">` in the `#brand` block, and a nodes-only data-URI favicon. Hand-maintained: `bun run build:ui` writes only `app.js` and `app.css` |
| `ui/src/style.css` | `--bg: #06080c`, `--accent: #8aa4ff`, and the kind colours `#8aa4ff`, `#ffb454`, `#42d6a4`, `#ff7eb6` |
| `crates/repomap/assets/app.css` | the committed copy of `ui/src/style.css`, so the same values |
| `ui/src/main.ts` | the 24-colour node palette (it starts `#8aa4ff`, `#ff7eb6`, `#42d6a4`, `#ffb454`) and the hover pair at line 228 |
| `crates/repomap/assets/app.js` | the built copy of `ui/src/main.ts`, so the same values |
| `README.md` | badge colours inside the `mark.sylphx.com` URLs (`8aa4ff`, `42d6a4`, `ffb454`) |
| `docs/public/img/*` | screenshots and the demo recording of the graph UI; they are re-recorded when that UI moves |

## Provenance

- `svg/repomap-symbol-on-dark.svg` is `docs/public/logo.svg`, moved unchanged. It arrived
  in 935c66e, 2026-09-25: "feat: repomap 1.0 — Spine and Locus merged into one codebase
  map for agents (#89)".
- `svg/repomap-symbol.svg`, `-black` and `-white` hold the same shapes as that master:
  the light-ground version recolours only the edges to `rgba(6,8,12,.35)`, and the
  one-colour versions set every fill and stroke to a single ink. No shape changed.
- `svg/repomap-app-icon.svg` was drawn to the lead's spec of 2026-09-28: the symbol on a
  `#06080C` rounded square, `rx` 22% of the side, 1024 viewBox, the symbol 70% of the
  width. It replaces the 512 px render that arrived with repomap 1.3 (a30295d, 2026-09-26)
  on a `#0F1320` ground, where the mark was 57.6% of the tile. The spec's 70% is the
  mark's own width, not the full 32-unit symbol box: the box is 97.4% of the tile.
- `svg/repomap-app-icon-square.svg` is that icon without the rounding, for a surface that
  applies its own mask.
- `tokens.json` was read from the repository's own stylesheets
  (`docs/.vitepress/theme/custom.css`, `ui/src/style.css`); no value changed.

Every file's SHA-256 is in `provenance.json`.

## Trademark

Not registered. Owner decision owner#781: no trademark filings before the product earns
money. Use ™ at most, never ®.

Checked 2026-09-28. **Descriptive name shared across the category.** "Repo map" is Aider's feature name, and at least five open-source tools use "repomap" or close variants (pdavis68/RepoMapper, joshfinnie/repomap, l0wigh/repomap-rs, ariadoss/repomap, agustinvillegas/repomap). The name is descriptive and not protectable; the identity is @sylphx/repomap and the graph UI. Mark (four graph nodes): generic node-graph motif.
