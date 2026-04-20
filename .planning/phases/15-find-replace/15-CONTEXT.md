# Phase 15: Find / Replace - Context

**Gathered:** 2026-04-20
**Status:** Ready for planning

<domain>
## Phase Boundary

Inline find/replace overlay for the active buffer. Search, navigate between matches, replace one or all, and jump to a specific line. All operations scoped to the currently active buffer — no project-wide search, no multi-file operations.

Scope anchored by ROADMAP.md FIND-01..FIND-08 success criteria.

</domain>

<decisions>
## Implementation Decisions

### Find bar layout & controls
- **Position:** Top overlay inside editor area, floats above text (VS Code style). Placed below the tab bar so tab bar is never occluded.
- **Width:** Compact, right-aligned fixed-width panel (not full-width). Browser Ctrl+F style.
- **Inline controls:** Full toolbar — case-sensitive, whole-word, regex toggles as icon buttons + match count + prev/next arrows + close X.
- **Close affordance:** X icon in top-right of bar (in addition to Esc).
- **Drag:** Fixed position — user cannot drag the bar.
- **Toggle style:** Icon buttons with an active state (accent background when on, muted when off).
- **Toggle placement:** Embedded inside the find input on its right edge (like VS Code).
- **Replace row:** Ctrl+H adds a second row below the find row (find input on top, replace input + replace-one + replace-all icon buttons on bottom).
- **Replace buttons:** Two icon buttons (replace-one + replace-all) with tooltips, not text labels.
- **Open animation:** Fade-in + slide-down (150–200ms) using existing Phase 9 transition infrastructure.
- **Search history:** Session-only, ~20 entries, accessible with ↑/↓ inside the find input. Cleared on app close.
- **Match count format:** `"3 of 12"` (full words, not `"3/12"`).
- **Bar background:** Opaque, elevated — shadow + bg_panel token. Clearly separated from editor text beneath.
- **No placeholder keyboard hints** inside inputs (user is expected to know shortcuts).
- **Count update debounce:** ~100ms after last keystroke before recomputing matches (for large-file stability).

### Match highlighting & states
- **Current match:** Solid accent background fill.
- **Other matches:** Muted highlight (dimmer accent-based fill).
- **Theme tokens:** Two new color tokens added to the design system — `MatchCurrent` and `MatchOther`. Not reusing raw accent_primary with opacity.
- **No-matches state:** Find input gets a red-tinted background + count area reads `"No results"`. No modal/popup.
- **Invalid regex state:** Same red-tinted / "No results" state as zero-matches (no separate tooltip required).
- **Wrap-around navigation indicator:** Claude's discretion.
- **Scroll-to-match:** When navigating to a match outside the viewport, auto-scroll so the match lands near the viewport center.
- **Scrollbar marks:** Small accent ticks rendered in the scrollbar track, showing where every match sits in the file.
- **Active selection interaction:** When the find bar opens, any existing editor selection is cleared — match highlights take over. No layering conflict.
- **Dim other text:** No. Text stays at normal opacity; only match highlights differentiate matches.
- **Cursor position after navigation:** Cursor lands at the END of the matched range.
- **Multi-line regex matches:** Supported — a single highlight spans across line boundaries.
- **Single-character match highlight:** Uses the character's bounding box exactly — no extra horizontal padding.

### Keyboard & focus flow
- **Pre-populate:** Ctrl+F with an active editor selection always overwrites the find input with the selection text.
- **Ctrl+F while bar is already open:** Selects all text in the find input; if editor has a selection, replaces input content with it.
- **Enter in find input:** Next match. `Shift+Enter` = previous. `F3` and `Shift+F3` are alternates.
- **F3 / Shift+F3 while bar is closed:** Opens the bar and navigates using the last search query.
- **Tab cycle (replace row open):** find input → replace input → toggles → wraps back to find.
- **Escape (from any bar input):** Closes the bar, returns keyboard focus to the editor. Cursor stays at the last navigated match.
- **Enter in replace input:** Replace one match and advance to the next.
- **Replace All:** No confirmation dialog. Executes silently as a single undoable transaction — Ctrl+Z reverts everything.
- **Replace All shortcut:** `Ctrl+Alt+Enter` while focus is in the replace input.
- **Toggle shortcuts:** `Alt+C` (case-sensitive), `Alt+W` (whole-word), `Alt+R` (regex). Mnemonics match VS Code.
- **Find input is single-line:** Enter always means "next match". Literal newline searches go through regex mode with `\n`.
- **Focused input styling:** Accent border + subtle glow on whichever input currently holds focus.

### Go to Line
- **Trigger:** `Ctrl+G` opens a centered modal dialog (reuse the existing Dialog component).
- **Modal size:** Small, ~280×80 px — single input with placeholder like `"Go to line (or line:col)"`.
- **Backdrop:** Transparent — editor stays visible behind the modal so the live preview is readable.
- **Input format:** `"42"` (line only) or `"42:5"` (line + column).
- **Indexing:** 1-indexed for BOTH line and column.
- **Negative numbers:** Count from end of file. `-5` = 5th line from the last line (Python-style).
- **Non-numeric characters:** Filtered — only digits and `:` can enter the input at all.
- **Out of range:** Clamp to the last line. No error, no red state.
- **Live preview:** As the user types digits, the editor scrolls to the target line and briefly highlights it. Esc reverts to the scroll position and cursor location that existed when the modal opened.
- **Commit:** Enter commits — cursor stays at the previewed location, modal closes.
- **Jump feedback:** Cursor moves to target + the target line gets a brief highlight flash to draw the eye.
- **History:** None — modal always opens empty.

### Claude's Discretion
- Exact wrap-around indicator design (silent wrap is acceptable).
- Toggle icon glyphs, exact padding, bar height, close-button hover style.
- Regex error tooltip vs. inline message (simplest inline approach consistent with no-match state).
- Line-highlight flash duration/color for Go to Line (short and subtle).
- Exact math for centering a match in the viewport.
- Opacity/blending percentages for `MatchOther` token relative to `MatchCurrent`.
- Exact scroll behavior when match is only partially visible vs. fully off-screen.

</decisions>

<specifics>
## Specific Ideas

- **VS Code as primary reference** — top overlay bar, embedded toggles, Alt+C/W/R mnemonics, F3 navigation while closed, cursor-at-end-of-match after navigation, Ctrl+Alt+Enter for Replace All.
- **Browser Ctrl+F compact positioning** — right-aligned, non-full-width so the user still sees the left portion of the editor while searching.
- **Python-style negative indexing for Go to Line** — `-5` = 5th line from end. Power-user shortcut, uncommon in editors but explicitly requested.
- **Replace All as single transaction** — one Ctrl+Z reverts everything, which justifies the no-confirmation decision.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope. Project-wide search, regex capture groups in replacements, and find-in-files remain out of scope for v2.0 unless added as new phases.

</deferred>

---

*Phase: 15-find-replace*
*Context gathered: 2026-04-20*
