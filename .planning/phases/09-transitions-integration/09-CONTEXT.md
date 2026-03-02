# Phase 9: Transitions & Integration - Context

**Gathered:** 2026-03-02
**Status:** Ready for planning

<domain>
## Phase Boundary

Two-part phase:
1. **CSS-like animation system** — Elements can animate property changes (color, opacity, position) on state transitions with configurable easing and duration
2. **wgpu_client migration** — Reduce wgpu_client to a thin app shell that delegates all UI to ora; eliminate hardcoded values and duplicated styling logic

This phase does NOT add new UI components or new editor features. It polishes existing interactions and consolidates the rendering ownership.

</domain>

<decisions>
## Implementation Decisions

### Transition feel & timing
- Claude's Discretion — pick appropriate durations and easing per interaction type
- Guidance: code editors favor snappy micro-interactions (hover ~100-150ms) with slightly longer macro-transitions (overlay open/close ~200ms)
- Easing curves should match the interaction — ease-out for most state changes, ease-in-out for position animations

### What gets animated
- Claude's Discretion — determine which elements animate by default vs opt-in
- Guidance: hover/active color transitions on interactive elements (buttons, tabs, list items) are good defaults
- Caret blink should use smooth fade (VS Code style) rather than hard toggle
- Focus ring appearance: Claude's discretion on instant vs quick fade
- Theme switching: Claude's discretion on instant vs cross-fade

### Overlay animations
- Claude's Discretion — determine entrance/exit animation style per overlay type
- Guidance: command palette and dialog could benefit from subtle fade; toasts from slide-in; instant show/hide is also acceptable if simpler

### Animatable properties
- Claude's Discretion — determine which CSS-like properties to support
- Minimum: background-color, text color, opacity
- Stretch goal: border-color, position offset for slide animations

### Migration boundary
- Claude's Discretion — determine how thin the wgpu_client shell becomes
- Guidance: wgpu_client should own minimal app setup (config, keybindings, root view registration) and delegate all rendering/layout/styling to ora
- Old wgpu_client rendering code should be removed (not kept as dead code) — ora is the single source of truth
- Audit and eliminate all hardcoded float literals (heights, widths, padding, font sizes) in wgpu_client

### Editor data bridge
- Claude's Discretion — determine data flow pattern between core_editor and ora views
- Guidance: ora views need to consume RenderModel from core_editor's ViewModelBuilder; keyboard events need to translate to EditorCommand
- Research should investigate current wgpu_client data flow to determine simplest migration path
- Dependency direction (ora depends on core_editor vs bridge in wgpu_client) should be decided based on keeping ora as generic as practical

### Claude's Discretion
All areas were discussed but specific selections were not captured due to tool limitations. Claude has flexibility across all implementation decisions. Key constraints from the roadmap success criteria:
1. Elements MUST support animating opacity, color, background-color, position with easing
2. Hover/active transitions MUST smoothly fade between states
3. wgpu_client MUST be reduced to thin app shell
4. ora views MUST consume RenderModel from core_editor
5. Keyboard events MUST translate to EditorCommand
6. ALL hardcoded values MUST be eliminated from wgpu_client
7. ALL duplicated styling logic MUST be removed from wgpu_client

</decisions>

<specifics>
## Specific Ideas

No specific requirements captured — open to standard approaches within the roadmap success criteria.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 09-transitions-integration*
*Context gathered: 2026-03-02*
