# Phase 5: Element Library - Context

**Gathered:** 2026-01-29
**Status:** Ready for planning

<domain>
## Phase Boundary

Build reusable styled primitives with Tailwind-style builder API and layout containers. This phase delivers the developer-facing API for constructing UI elements in code: fluent builder methods, layout containers (row/column/stack), basic interactive elements (Button), and Image element. Input and other complex widgets are deferred to Phase 8.

</domain>

<decisions>
## Implementation Decisions

### Builder API style
- Fluent chaining pattern: `div().flex().gap(px(4)).bg(red)` — everything returns Self
- Explicit units required: `.width(px(200))` or `.width(pct(50))` — verbose but clear about what's being specified
- Children added via chained `.child()` method: `div().child(a).child(b)` — one at a time
- Method naming uses mix approach: short names for very common properties (`.bg()`, `.w()`, `.h()`), verbose for others (`.padding()`, `.margin()`)

### Container semantics
- Row/column as Div methods: `div().row()` or `div().flex_row()` — direction configured on Div, not separate types
- Alignment via separate axes: `.justify(Center).items(Start)` — CSS flexbox style with main axis and cross axis
- Explicit `stack()` container for z-axis layering: `stack().child(bg).child(fg)` — children layer on top of each other
- Gap/spacing accepts both: `.gap(8)` for raw pixels and `.gap(Spacing::Md)` for design tokens — flexibility during development, tokens enforced later

### Interactive elements
- Button as separate element type: `button("Click").on_click(|_| {})` — dedicated element with semantic meaning
- Input deferred to Phase 8 — keep Phase 5 focused on display elements and basic interaction
- Button uses variant system: `button().primary()` or `.secondary()` — preset variants include hover/active states
- Button supports composable children: `button().child(icon).child(text)` — full flexibility for icon+text combos

### Image element
- Both file and data sources: `img(path)` for files, `img_from_bytes(data)` for in-memory — flexibility for different use cases
- Object-fit configurable: `.object_fit(Contain)` or `.object_fit(Cover)` — control how image scales in container
- Placeholder color while loading/on error — show background color until loaded, keep showing on error
- Async loading in background — load images non-blocking, show placeholder, re-render when ready

### Claude's Discretion
- Exact method names for padding/margin sides (e.g., `.pl()` vs `.padding_left()`)
- Internal texture caching strategy for images
- Default placeholder color value
- Stack z-ordering mechanics

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 05-element-library*
*Context gathered: 2026-01-29*
