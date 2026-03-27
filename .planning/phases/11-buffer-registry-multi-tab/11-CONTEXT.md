# Phase 11: Buffer Registry + Multi-Tab - Context

**Gathered:** 2026-03-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Users can open multiple files in tabs and switch between them. Each buffer is deduplicated (same file path = same buffer) and preserves its own scroll, cursor, selection, and undo state. This phase builds the Buffer Registry as the central data structure and wires tab switching, closing, and dirty tracking. File I/O (open/save dialogs) and sidebar file browser are separate phases.

</domain>

<decisions>
## Implementation Decisions

### Tab close behavior
- Closing the last tab shows an empty state (no tabs, welcome/empty screen)
- After closing a tab, focus goes to the most recently used tab (JetBrains-style MRU)
- Middle-click on a tab closes it
- Right-click context menu on tabs with: Close, Close Others, Close All, Close to the Right

### Dirty state & save dialog
- Dirty indicator: dot before filename (VS Code style)
- Dot and close button (X) both visible simultaneously (no dot-replaces-X on hover)
- Save dialog on dirty close: Save / Discard / Cancel (three buttons)
- Closing multiple dirty tabs at once: follow IDE convention (single batch dialog listing all unsaved files)

### Tab overflow & ordering
- Overflow: horizontal scroll with left/right arrow buttons at edges (VS Code style)
- Ctrl+Tab cycles in visual order (left to right), not MRU
- No drag-to-reorder in this phase (deferred)
- New tabs open next to the active tab (inserted after current, not appended to end)

### Buffer switching UX
- All state preserved per-buffer: scroll position, cursor position, selection, undo history
- Tab switching is instant (no animation/transition)
- Re-opening an already-open file just switches to its existing tab (preserves current scroll/cursor state)
- Active tab: background highlight + accent-colored bottom border (JetBrains style)
- Border color changes based on editor focus: accent when focused, muted when unfocused

### Claude's Discretion
- Buffer Registry internal data structure design
- Empty state screen content and layout
- Tab scroll arrow button styling
- Context menu visual styling
- Exact tab width and truncation behavior for long filenames

</decisions>

<specifics>
## Specific Ideas

- "Make it like JetBrains" — tab close focus order follows MRU, active tab border changes color based on editor focus state
- Dot indicator + X button both visible (no toggle behavior)
- Instant tab switching — snappy, no transitions

</specifics>

<deferred>
## Deferred Ideas

- Tab drag-to-reorder — future enhancement
- Tab pinning — not in scope

</deferred>

---

*Phase: 11-buffer-registry-multi-tab*
*Context gathered: 2026-03-26*
