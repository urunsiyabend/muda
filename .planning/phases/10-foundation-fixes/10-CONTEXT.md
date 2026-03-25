# Phase 10: Foundation Fixes - Context

**Gathered:** 2026-03-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Eliminate idle GPU waste, audit all v2 commands, redesign EditorDataSource for v2 scope, and fix the selection rendering bug before feature work begins. No new capabilities — this phase fixes and restructures what exists so v2 phases build on solid ground.

</domain>

<decisions>
## Implementation Decisions

### Redraw strategy
- Event-driven redraw only — zero GPU work when editor is idle (no heartbeat/polling)
- Caret blink uses hard toggle (visible/invisible snap) on a fixed interval, not smooth fade
- Caret blink timer resets on every input — caret stays solid while typing, only blinks during pauses
- Animations use a Zed-style global animation loop: any active animation enables continuous rendering, loop stops when all animations complete
- External async events (file watcher, etc.) request their own explicit redraw when they arrive

### EditorDataSource redesign
- Split monolithic trait into focused sub-traits grouped by logical domain (WorkspaceData, BufferData, UIStateData, etc.)
- Views receive separate trait objects (`&dyn WorkspaceDataSource`) — explicit dependencies, not a single adapter
- Only create sub-traits for current needs in Phase 10 — new sub-traits added by the phase that needs them (no upfront stubs)

### Command audit approach
- Register ALL v2 keybindings now in Phase 10 with stub handlers — prevents unknown-key panics
- Unimplemented commands show a status bar message: "Open File: not yet available" (or similar)
- Full audit of both v1 and v2 commands — verify existing arrow keys, typing, backspace still route correctly after trait redesign
- No command dispatch should ever panic — graceful handling always (log warning, status bar message, continue)

### Selection rendering fix
- Zed-style selection highlight: brighter than VS Code, preserves syntax highlighting, good contrast against editor background
- Selection stays visible but dimmed when editor loses focus (like VS Code unfocused selection)
- Multi-line selections extend to full line width (edge-to-edge), not just text characters
- Fix current-line highlight + selection highlight coexistence in this phase — both should render correctly together

### Claude's Discretion
- Exact selection highlight color values for dark/light themes
- Caret blink interval duration (500ms typical)
- Internal architecture of the dirty-flag / redraw-request system
- How the global animation loop integrates with ora's existing transition registry
- Sub-trait naming conventions and exact method signatures

</decisions>

<specifics>
## Specific Ideas

- "Do this like Zed does" — reference Zed's animation/redraw model for the global animation loop approach
- Selection highlight should have Zed-level contrast/brightness, not VS Code's subtle blue
- Status bar messages for unimplemented commands should be brief and informative

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 10-foundation-fixes*
*Context gathered: 2026-03-26*
