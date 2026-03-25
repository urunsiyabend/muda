# Phase 4: Event System - Context

**Gathered:** 2026-01-29
**Status:** Ready for planning

<domain>
## Phase Boundary

Mouse/keyboard input routing to elements, focus management for keyboard events, and two-phase event dispatch (capture/bubble). Elements can respond to user interaction, query hover/active states, and bind keyboard shortcuts to typed actions.

</domain>

<decisions>
## Implementation Decisions

### Hit testing behavior
- Topmost element receives event, then bubbles through parent chain
- Per-element opt-in for transparent region hit testing (hit_test_visible flag)
- Only visible portion of children hit-testable (scrolled-out regions don't receive hits)
- Mouse capture supported for drag operations (capture_mouse() on mousedown, release on mouseup)

### Focus model
- Interactive elements focusable by default (buttons, inputs); other elements opt-in with focusable(true)
- Tab navigation automatic: Tab moves to next focusable in tree order, Shift+Tab goes back
- Focus ring only appears when focus gained via keyboard (not mouse click)
- Focus scopes (trap/cycle) deferred to later phase when dialogs need them

### Action registry design
- Context-aware keybinding mappings: same key can mean different things in different contexts
- Chord keybindings (Ctrl+K Ctrl+C) deferred to later phase
- Conflict resolution: most specific wins (focused element beats parent, parent beats global)
- Actions are typed structs implementing Action trait (not string identifiers)

### Hover/active tracking
- Query via context: cx.is_hovered(element_id), cx.is_active(element_id)
- Full ancestry tracked: element AND all ancestors are "hovered" when cursor over element
- No MouseEnter/MouseLeave events; elements check is_hovered() during render
- Active state (pressed) auto-tracked by framework: mousedown→mouseup managed automatically

### Claude's Discretion
- Exact hitbox registration API
- Internal data structures for hover/active tracking
- Event struct layouts
- Focus ring styling (when keyboard-triggered)

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches following GPUI patterns.

</specifics>

<deferred>
## Deferred Ideas

- Chord keybindings (Ctrl+K Ctrl+C style) — add when editor needs them
- Focus scopes/traps — add when dialogs implemented in Phase 7
- MouseEnter/MouseLeave events — revisit if query-only proves limiting

</deferred>

---

*Phase: 04-event-system*
*Context gathered: 2026-01-29*
