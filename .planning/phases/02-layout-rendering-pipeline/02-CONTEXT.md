# Phase 2: Layout & Rendering Pipeline - Context

**Gathered:** 2026-01-28
**Status:** Ready for planning

<domain>
## Phase Boundary

Implement simple stack/flex layout engine, GPU rendering pipeline with batching, and glyphon text rendering. This phase delivers the rendering backbone — elements appear on screen with correct positioning. Div and Text primitives are introduced. Creating styled components and design tokens are separate phases.

</domain>

<decisions>
## Implementation Decisions

### Layout model semantics
- CSS flexbox subset: row/column with justify-content, align-items, gap, flex-grow/shrink/basis
- Child wins on size conflicts — children keep their requested size, overflow handled by clipping
- Percentage sizing relative to parent element (standard CSS behavior)
- Full min/max constraints: min-width, max-width, min-height, max-height
- Core alignment subset: justify-content (start/center/end/space-between) + align-items (start/center/end/stretch)
- Absolute + relative positioning supported — elements can be taken out of flow with top/left/right/bottom
- No flex-wrap — items stay on single axis, overflow or shrink
- align-self supported — individual children can override parent's cross-axis alignment

### Div/rectangle rendering
- Per-corner border-radius — each corner can have a different radius
- Per-side border width — each side (top/right/bottom/left) can have different widths
- Box shadows supported — offset-x, offset-y, blur, spread, color
- Colors specified as RGBA floats plus linear gradient support (start color, end color, angle)

### Text rendering integration
- Text measurement happens during layout phase via glyphon — accurate dimensions, layout coupled to text shaping
- Both single-line and multi-line modes — text element has wrap mode (truncate or word wrap)
- Embedded default font in binary + runtime font loading API from files
- Styled runs within a single Text element — segments with different fonts, sizes, colors for syntax highlighting and rich text

### Clipping & overflow
- Opt-in clipping — elements only clip children when overflow: hidden/scroll is set, default is visible (matches CSS)
- Nested clipping supported — clip regions intersect for complex layouts
- Basic scroll support included — scrollable containers with scroll offset
- Both vertical and horizontal scroll axes supported independently

### Claude's Discretion
- GPU batching strategy and draw call organization
- Specific shader implementation for rounded corners and shadows
- Glyphon text atlas management and cache strategy
- Exact instanced rendering approach for rectangles
- Scroll input handling details (wheel events, momentum)
- Linear gradient shader implementation details

</decisions>

<specifics>
## Specific Ideas

- Layout model should feel like a CSS flexbox subset — familiar to anyone who knows flexbox, but without the full spec complexity (no wrap, no order, no flex-direction reverse)
- Per-corner radius and per-side border needed for tab underlines, card styles, and separator lines without extra wrapper elements
- Styled text runs are critical — this is a code editor, syntax highlighting is a core use case
- Scroll support in Phase 2 ensures file trees and code viewport can work without waiting for later phases

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 02-layout-rendering-pipeline*
*Context gathered: 2026-01-28*
