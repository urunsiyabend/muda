# Phase 9: Transitions & Integration - Context

**Gathered:** 2026-03-25
**Status:** Ready for planning

<domain>
## Phase Boundary

Add CSS-like transitions for UI polish (opacity, color, position, size animations with easing) and complete wgpu_client migration to a minimal ora shell. wgpu_client becomes just `main() + ora::run()`. All UI, rendering, input handling, and design tokens live in ora.

</domain>

<decisions>
## Implementation Decisions

### Transition Behavior
- All four property types supported in v1: colors (bg, text, border), opacity, position/transform, size (width, height)
- Default duration: balanced 150-250ms (standard desktop app feel, like VS Code/Zed)
- Interruption: reverse from current interpolated value (modern IDE behavior) — no visual jump on state change mid-animation
- Theme switching is instant swap, NOT animated — colors change immediately on toggle

### Animation API Surface
- Builder method per property: `.transition_bg(200.ms())`, `.transition_opacity(150.ms())` — explicit, granular control
- Easing: global default (ease-out) with per-transition override via chained method (e.g., `.transition_bg(200.ms()).easing(ease_in_out)`)
- Auto-drive redraws: transition system requests animation frames at ~60fps while any animation is in-flight
- Enter + exit transitions supported: elements can define appear/disappear animations (toast slide-in, dialog scale-up, fade-out on dismiss)

### Migration Boundary
- wgpu_client becomes minimal shell: only `main()` + `ora::run()` — all UI logic moves to ora
- Input translation (winit key events → EditorCommand) moves to ora — views handle input via ora's action system
- Adapter/trait boundary between ora and core_editor — ora views do NOT import core_editor types directly; a trait abstracts the data contract for RenderModel
- wgpu_client's design_system/ tokens deleted entirely — ora tokens are the single source of truth

### Migration Strategy
- Integration first, transitions second — ensure core functionality works before adding polish
- Incremental migration by component — one component at a time (e.g., TabBar, then Sidebar, then editor) with verification at each step
- Visual similarity target (not pixel-perfect) — ora's design tokens may produce slightly different results and that's acceptable
- Delete old code as we migrate — each migrated component's wgpu_client code gets removed immediately

### Claude's Discretion
- Specific easing function defaults per property type
- Adapter trait design for core_editor RenderModel abstraction
- Migration order of components (which component to migrate first)
- Animation frame scheduling mechanism within winit event loop
- Enter/exit animation implementation approach

</decisions>

<specifics>
## Specific Ideas

- Transition interruption should match modern IDE behavior (VS Code, Zed) — smooth reversal from current value, never a visual pop
- wgpu_client has existing animation primitives (Tween, Easing) in its codebase — can reference for easing function implementations
- The adapter trait boundary means ora could theoretically work with editors other than core_editor in the future

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 09-transitions-integration*
*Context gathered: 2026-03-25*
