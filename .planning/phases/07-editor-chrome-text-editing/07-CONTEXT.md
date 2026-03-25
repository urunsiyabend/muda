# Phase 7: Editor Chrome & Text Editing - Context

**Gathered:** 2026-01-30
**Status:** Ready for planning

<domain>
## Phase Boundary

Migrate editor chrome components (TabBar, StatusBar, Sidebar, Gutter, Dialog) and text editing core (TextArea, Caret, Selection) to ora Views. This phase builds the editor shell UI and text rendering infrastructure. File tree navigation and command palette are separate phases.

</domain>

<decisions>
## Implementation Decisions

### Tab behavior
- Overflow handling: dropdown menu at end for tabs that don't fit
- Drag-and-drop reordering supported within the tab bar
- Close unsaved tab: modal dialog with Save/Don't Save/Cancel
- File type icons shown before filename (language-specific icons like Rust, JS, etc.)

### Sidebar layout
- Position: left side only
- Toggle: dedicated button/icon to collapse/expand (not header click or drag threshold)
- Resize: draggable edge to adjust width
- Collapsed state: narrow icon rail (not completely hidden)

### Text rendering
- Line wrapping: no wrap, horizontal scroll for long lines
- Selection: solid background highlight (not semi-transparent)
- Caret: thin line (beam cursor style)
- Current line: full background highlight spanning both gutter and editor area

### Dialog presentation
- Backdrop: semi-transparent dim overlay behind modal
- Button placement: right-aligned in dialog footer
- Escape key: does NOT dismiss (must use Cancel button)
- Backdrop click: does NOT dismiss (must use buttons)

### Claude's Discretion
- Caret blink rate and animation
- Exact tab dropdown menu styling
- Resize handle visual appearance (hover states, cursor)
- Dialog animation (fade/scale) if any
- StatusBar content layout and spacing
- Gutter width calculation

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

*Phase: 07-editor-chrome-text-editing*
*Context gathered: 2026-01-30*
