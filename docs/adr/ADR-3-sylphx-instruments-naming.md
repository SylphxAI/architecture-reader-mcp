# ADR-3: Sylphx Instruments product naming

Date: 2026-07-31  
Status: Accepted  
Slug: sylphx-instruments-naming

## Context

Portfolio packages were named as category SKUs (`pdf-reader-mcp`,
`architecture-reader-mcp`, `smart-reader-mcp`, …). Those names are searchable
but weak as brands. Category leaders in adjacent spaces use signature names
(e.g. Graphify, Serena, Docling, wigolo).

Agents, READMEs, CLIs, and npm need a consistent naming system that is
memorable without destroying existing Citra/PDF package SEO overnight.

## Decision

1. Adopt umbrella brand **Sylphx Instruments**.
2. Adopt product display names:

| Product | Brand | Transitional technical id |
| --- | --- | --- |
| PDF evidence | **Citra** | pdf-reader-mcp |
| Image evidence | **Iris** | image-reader-mcp |
| Video evidence | **Cue** | video-reader-mcp |
| Media router | **Prism** | smart-reader-mcp |
| Architecture engine | **Spine** | architecture-reader-mcp |
| Web access | **Lookout** | new product |

3. End-state packages/bins: `@sylphx/<brand-lowercase>` and bin `<brand-lowercase>`.
4. Migration order: display → CLI/MCP id → new package → repo rename.  
   PDF/Citra keeps the longest dual-name compatibility window.
5. Do not put `MCP`, `Smart`, `AI`, or `Reader` in the **brand** token.
   Category phrases remain in subtitles only.

## Consequences

- All new public copy leads with brand names.
- Portfolio SSOT and roadmaps use brand names primarily.
- Implementers may keep transitional repo folders until package cutover.
- Trademark/npm collisions on generic English words are mitigated by the
  `@sylphx/` scope and taglines, not by reverting to `*-reader-mcp` brands.

## Verification

- [ ] Portfolio SSOT lists the six brands
- [ ] Each active README hero can adopt brand + subtitle without schema change
- [ ] No archived product is assigned a new Instruments brand for relaunch
  without a new ADR

## References

- `docs/portfolio/sylphx-instruments-ssot.md`
