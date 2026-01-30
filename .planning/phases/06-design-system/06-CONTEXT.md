# Phase 6: Design System - Context

**Gathered:** 2026-01-30
**Status:** Ready for planning

<domain>
## Phase Boundary

Consolidate design tokens (colors, spacing, typography) as single source of truth with theme support. All visual tokens live in ora crate only. Theme system supports dark and light palettes, switchable at runtime.

</domain>

<decisions>
## Implementation Decisions

### Color token naming
- Hybrid approach: semantic tokens (bg_primary, fg_secondary) that reference functional palette (gray_900, blue_500)
- Standard set of 12 semantic categories: bg_primary, bg_secondary, bg_elevated, fg_primary, fg_secondary, fg_muted, accent, accent_hover, accent_active, border, error, success
- API: `theme.color(Token::BgPrimary)` — enum variants with single method
- Underlying palette publicly accessible: `theme.palette(Palette::Gray100)` as escape hatch for custom visuals
- Document semantic tokens as preferred path

### Spacing scale
- Tailwind-style numeric scale: 1=4px, 2=8px, 3=12px, 4=16px...
- Range 0-16: 0=0, 1=4, 2=8, 3=12, 4=16, 5=20, 6=24, 7=28, 8=32... 16=64
- Both raw pixels and token function supported: `div().padding(8)` or `div().padding(sp(2))`
- Negative spacing supported: `margin(-2)` or `sp(-2)` for overlap/pull effects

### Typography system
- Both semantic and scale names: semantic preferred (TextSize::Body), scale available (TextSize::Sm)
- Base body font size: 14px
- Two font families: ui_font (system sans) + code_font (monospace)
- Text size tokens bundle font size + line height together for complete typographic rhythm
- Optional line height override available

### Theme switching
- Theme can be set at startup AND changed at runtime
- Two built-in themes: dark + light
- Custom themes via overrides only (override specific tokens in built-in themes, not full custom palettes)
- Theme accessed via context: `cx.theme().color(Token::BgPrimary)`

### Claude's Discretion
- Exact gray/color shade values for palettes
- Default line height ratios (1.2, 1.4, 1.5)
- Theme persistence mechanism (if any)
- Internal theme storage architecture

</decisions>

<specifics>
## Specific Ideas

- Palette should feel modern — not too contrasty, comfortable for long coding sessions
- Semantic tokens should cover 90%+ of UI needs; palette access is escape hatch, not primary API

</specifics>

<deferred>
## Deferred Ideas

- High contrast accessibility theme — future phase
- Full custom theme definitions — currently only overrides supported

</deferred>

---

*Phase: 06-design-system*
*Context gathered: 2026-01-30*
