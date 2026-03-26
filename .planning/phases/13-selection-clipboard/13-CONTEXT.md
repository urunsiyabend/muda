# Phase 13: Selection + Clipboard - Context

**Gathered:** 2026-03-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Users can select text with mouse and keyboard, and copy/cut/paste through the OS clipboard. Covers click-to-position, drag selection, shift+arrow, Ctrl+A, Ctrl+C/X/V. Multi-cursor and block selection are out of scope.

</domain>

<decisions>
## Implementation Decisions

### Click positioning
- Clicking past end of line snaps cursor to end of line content (no virtual space)
- Clicking below last line of file snaps cursor to last character of file
- Mid-character clicks use nearest-boundary snapping (past midpoint snaps right, otherwise left)
- Clicking in gutter (line number area) selects the entire line

### Drag & shift selection
- Double-click selects the whole word under cursor
- Triple-click selects the entire line including newline
- Double-click then drag expands selection word-by-word (word-snap drag)
- Word boundaries follow VS Code style: alphanumeric+underscore runs are words; punctuation and whitespace are separate groups

### Clipboard integration
- Ctrl+C with no selection copies the entire current line (including newline)
- Ctrl+X with no selection cuts the entire current line and removes it
- Pasting a full-line copy inserts above the current line (not inline at cursor)
- Ctrl+V auto-indents pasted text to match surrounding indentation level

### Selection visuals
- Selection highlight extends to full line width (fills to right edge of editor)
- Highlight color is semi-transparent accent color (~30% opacity overlay)
- Caret remains visible at the active end of selection, keeps blinking
- When editor loses focus, selection stays visible but dimmed (reduced opacity)

### Claude's Discretion
- Exact selection highlight opacity values and color derivation from theme
- Auto-indent algorithm details for paste
- Scroll-while-dragging speed and margins
- How Home/End interact with selection (Shift+Home/End)

</decisions>

<specifics>
## Specific Ideas

- VS Code is the primary reference for all selection and clipboard behavior
- Full-line copy/cut/paste cycle should feel identical to VS Code (copy line, paste above)
- Word boundary rules match VS Code (not CamelCase-aware)

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 13-selection-clipboard*
*Context gathered: 2026-03-26*
