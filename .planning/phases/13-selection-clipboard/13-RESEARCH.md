# Phase 13: Selection + Clipboard - Research

**Researched:** 2026-03-26
**Domain:** Text editor selection model, mouse-to-text hit testing, OS clipboard integration
**Confidence:** HIGH — all findings verified against actual codebase. No external library
uncertainty; arboard is already in use and confirmed working.

## Summary

Phase 13 wires together existing infrastructure that is already largely present. The
`SelectionSet`, `Selection`, `EditorView.move_caret_to(offset, extend_selection)`, and
clipboard (`arboard::Clipboard`) are all implemented in `core_editor`. The keyboard path
(Shift+arrow, Ctrl+A, Ctrl+C/X/V) is already translated by `translate_editor_command` and
dispatched through `CommandDispatcher`. What is **missing** is the mouse path: no code
converts pixel coordinates to (line, column), no code registers a mouse hitbox on the
text area, and no code dispatches `MoveCursor`/selection commands on mouse click or drag.

The second gap is clipboard semantic richness: the context decisions require full-line
copy/cut when there is no selection, paste-above for line-clipboard content, and
auto-indent on paste. These behaviors need new logic in `CommandDispatcher.handle_copy`,
`handle_cut`, and `handle_paste`, plus a way to tag clipboard payloads as "line copies"
(VS Code uses a metadata convention described below).

The third gap is focus-driven selection dimming: the rendering layer needs to know whether
the editor has focus in order to render selections at full vs. reduced opacity.

**Primary recommendation:** Implement in three sequential sub-tasks: (1) mouse hit testing
and click-to-cursor in the text area, (2) drag and shift-click selection, (3) clipboard
semantic enrichment (full-line copy/cut/paste, auto-indent).

## Standard Stack

### Core (already in Cargo.toml — no new deps needed)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| arboard | 3.6.1 | OS clipboard read/write | Already used in `CommandDispatcher`; cross-platform |
| ropey | 1.6.1 | Rope text buffer for offset/line mapping | All coordinate conversion already works |
| winit | 0.30 | Mouse events + modifiers | Already wired in `event_loop.rs` |

### No New Dependencies

All required capabilities are present in the workspace. Avoid adding dependencies.

**Installation:** none required

## Architecture Patterns

### Recommended File Layout

The changes touch four layers:

```
core_editor/src/
├── commands/
│   ├── dispatcher.rs        # handle_copy, handle_cut, handle_paste enrichment
│   └── editor_command.rs    # add ClickAt { line, col, extend } + DragTo { line, col }
ora/src/
├── views/
│   └── text_area.rs         # register hitbox, on_mouse_down, on_mouse_move, on_mouse_up
├── editor_adapter/
│   └── types.rs             # add ClickAt / DragTo to EditorCommand enum
wgpu_client/src/
│   └── adapter.rs           # convert ClickAt/DragTo to core_editor equivalents
```

### Pattern 1: Pixel-to-Document Coordinate Mapping

**What:** Convert a raw pixel point (relative to the text area origin) into a
`(line_idx, col_idx)` pair in document coordinates.

**When to use:** In every mouse handler that must position the cursor.

**Formula:**

```rust
// Source: TextAreaView constants + event_loop.rs measurement
// LINE_HEIGHT = 21.0 (text_area.rs constant)
// char_width = crate::rendering::measured_char_width() (set at GPU init)
// scroll_y = adapter.scroll_y()  (logical first-visible-line index)
// scroll_y_offset_px = shared_scroll_offset.get()  (sub-line pixel offset)

fn pixel_to_doc(
    px: Point,          // position relative to text area origin
    text_area_origin: Point,
    char_width: f32,
    scroll_y: usize,
    scroll_y_offset_px: f32,
    scroll_x: usize,
) -> (usize, usize) {   // (line_idx, col_idx) — both 0-indexed
    const LINE_HEIGHT: f32 = 21.0;
    let local_y = px.y - text_area_origin.y + scroll_y_offset_px;
    let local_x = px.x - text_area_origin.x;

    let line_in_viewport = (local_y / LINE_HEIGHT).floor() as usize;
    let line_idx = scroll_y + line_in_viewport;

    // Mid-character snapping: round to nearest boundary
    let col_raw = ((local_x / char_width) + 0.5).floor() as usize;
    let col_idx = scroll_x + col_raw;

    (line_idx, col_idx)
}
```

The result must then be clamped:
- `line_idx` clamped to `0..document.len_lines()-1`
- `col_idx` clamped to `0..document.line_len(line_idx)`

This produces the behavior specified in CONTEXT: past-end snaps to EOL, past-last-line
snaps to EOF, midpoint snapping gives nearest-boundary.

**Confidence:** HIGH — derived from actual `LINE_HEIGHT`, `char_width`, `scroll_y` values
in the codebase.

### Pattern 2: Mouse Hitbox Registration in TextAreaView

**What:** Register an opaque hitbox in `prepaint` that covers the entire text area,
then wire `on_mouse_down` / `on_mouse_move` / `on_mouse_up` handlers.

**When to use:** The handler receives raw pixel `position` and must convert to doc coords.

```rust
// In TextAreaView::prepaint (new method — currently View doesn't call prepaint)
// Alternative: wire the handler inside render() using a Div element callback.
// Preferred: add prepaint/paint lifecycle to View, OR use the Div element's
// existing on_mouse_down support (verify Div exposes this — see elements/div.rs).

// Pattern used by tab.rs (verified):
let hitbox_id = cx.register_hitbox(bounds, /*opaque=*/ true);
cx.on_mouse_down(hitbox_id, move |event, ctx| {
    if ctx.phase() != DispatchPhase::Bubble { return; }
    // Convert event.position to doc coords, dispatch ClickAt command
});
cx.on_mouse_move(hitbox_id, move |event, ctx| {
    if ctx.phase() != DispatchPhase::Bubble { return; }
    // If mouse captured (dragging), dispatch DragTo command
});
```

**Key issue:** `TextAreaView` currently implements `View` which only has `render()`.
It has no `prepaint`/`paint` lifecycle. Mouse handlers require `prepaint`. The pattern
must either:
a) Convert `TextAreaView` to a full element (has `prepaint`/`paint`/`layout`), or
b) Wrap it in a `Div` element that registers the hitbox and delegates rendering.

Option (b) is lower risk. Use a `Div` element wrapper in `views/text_area.rs` that is
opaque, registers the hitbox, and wraps the existing stack layout.

**Confidence:** HIGH — tab.rs proves the `on_mouse_down` pattern works. The `View`
vs. full-element tension is a real constraint observed in the code.

### Pattern 3: Mouse Capture for Drag Selection

**What:** Use `cx.capture_mouse(hitbox_id, button)` to keep receiving `MouseMove` events
even when the cursor leaves the text area element.

**When to use:** On `mouse_down` (start of drag), capture. On `mouse_up`, capture is
automatically released by the event loop (`release_mouse_capture` in event_loop.rs line 448).

```rust
// On mouse_down in text area handler:
cx.capture_mouse(hitbox_id, MouseButton::Left);
```

The event loop already routes `CursorMoved` events to the captured element
(event_loop.rs lines 359–386). The `on_mouse_move` handler fires with positions
outside the text area bounds during drag.

**Confidence:** HIGH — `capture_mouse` API and its event loop integration verified
in `context.rs` and `event_loop.rs`.

### Pattern 4: New EditorCommand Variants for Mouse Input

**What:** Add `ClickAt` and `DragTo` to `ora::editor_adapter::types::EditorCommand`,
and corresponding `core_editor::commands::EditorCommand` variants.

```rust
// In ora/src/editor_adapter/types.rs:
pub enum EditorCommand {
    // ... existing variants ...
    /// Position cursor at a document coordinate (from mouse click).
    /// extend_selection: true when Shift is held or during drag.
    ClickAt { line: usize, col: usize, extend_selection: bool, click_count: u32 },
    /// Extend selection during mouse drag (mouse move with button held).
    DragTo { line: usize, col: usize },
    /// Gutter click: select entire line at given line index.
    GutterClickAt { line: usize },
}

// In core_editor/src/commands/editor_command.rs:
pub enum EditorCommand {
    // ... existing variants ...
    ClickAt { line: usize, col: usize, extend_selection: bool, click_count: u32 },
    DragTo { line: usize, col: usize },
    GutterClickAt { line: usize },
}
```

Dispatch in `CommandDispatcher`:
- `ClickAt { extend_selection: false, click_count: 1 }` → position cursor, clear selection
- `ClickAt { extend_selection: true, click_count: 1 }` → Shift+click, extend selection
- `ClickAt { click_count: 2 }` → select word at position
- `ClickAt { click_count: 3 }` → select line at position
- `DragTo` → extend selection to position (anchor stays at click origin)
- `GutterClickAt` → select entire line including newline

**Confidence:** HIGH — `click_count` is already tracked in `event_loop.rs` and passed
in `MouseDownEvent.click_count`.

### Pattern 5: Full-Line Clipboard (VS Code behavior)

**What:** When `Copy` is dispatched with no selection, copy the entire current line
including its newline. Mark the clipboard payload with a metadata convention so paste
knows to insert above the current line.

**VS Code convention (MEDIUM confidence — from reading VS Code source patterns):**
VS Code stores a JSON metadata blob alongside the text: `{"version":1,"isFromEmptySelection":true}`.
Since we are writing our own editor, a simpler approach is valid:

```rust
// In CommandDispatcher: track whether the last clipboard write was a line-copy.
// A simple bool is sufficient since single-buffer clipboard ownership is exclusive.

struct CommandDispatcher {
    clipboard: Option<Clipboard>,
    clipboard_is_line_copy: bool,  // true if last Copy/Cut had no selection
}
```

`handle_copy` (no selection):
1. Get the full line text including newline: `document.line(caret_line) + "\n"`
2. Write to clipboard
3. Set `clipboard_is_line_copy = true`

`handle_cut` (no selection):
1. Copy full line to clipboard
2. Delete the entire line from the buffer
3. Set `clipboard_is_line_copy = true`

`handle_paste` when `clipboard_is_line_copy`:
1. Read clipboard text
2. Find the start of the current line
3. Insert text BEFORE the current line (not at caret position)
4. Leave the caret at the original line, now shifted down one line

**Confidence:** MEDIUM — the VS Code behavior is well-documented community knowledge
but the internal mechanism for tracking line-copy ownership is custom to this codebase.

### Pattern 6: Auto-Indent on Paste

**What:** When pasting multi-line text, adjust indentation of each pasted line to match
the indentation of the line at the cursor (or insertion point for line copies).

**Algorithm (Claude's Discretion — design recommendation):**

```
1. Determine reference indentation: leading whitespace of the line where text will be inserted.
2. Determine source indentation: leading whitespace of the FIRST line of the clipboard text.
3. For each line in clipboard text:
   a. Remove source indentation prefix (if line has at least that many spaces/tabs).
   b. Prepend reference indentation.
4. Handle mixed tabs/spaces: normalize to the document's detected indent style.
```

**Edge cases:**
- Single-line paste: no re-indentation needed (just insert at cursor).
- Blank lines in pasted text: leave blank (no indentation added).
- Lines with LESS indentation than source first line: keep relative de-indent.

**Confidence:** LOW for the exact algorithm — no official spec. The approach above is
the most common pattern seen in editors; test against VS Code paste behavior.

### Anti-Patterns to Avoid

- **Storing pixel coords in core_editor:** core_editor must only receive document
  coordinates (line, col), never raw pixels. The pixel-to-doc conversion belongs in
  the `TextAreaView` (or its wrapper) before dispatching the command.

- **Dispatching `MoveCursor` for mouse:** Use `ClickAt`/`DragTo` commands for mouse
  input rather than `MoveCursor`. `MoveCursor` moves relative to current caret and has
  no concept of absolute (line, col) targeting. `ClickAt` is absolute.

- **Re-rendering on every `MouseMove` during drag:** Only call `request_redraw` when
  the selection actually changes (i.e., when `DragTo` moves to a different character
  position than before). Avoid per-pixel redraws.

- **Forgetting the dispatch phase check:** Every mouse handler in this codebase MUST
  guard `if ctx.phase() != DispatchPhase::Bubble { return; }`. Without this guard,
  handlers fire twice (capture + bubble). See tab.rs and tree_item.rs for reference.

- **Opacity via Selection ColorToken:** The current `ColorToken::Selection` is a fixed
  color. Focus-dimming requires either a second color token `SelectionInactive` or
  the text area view receiving a `has_focus: bool` flag from the render model and
  choosing opacity at render time. The `ColorToken` approach is cleaner.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| OS clipboard access | Custom clipboard | `arboard::Clipboard` (already dep) | Handles platform differences; already in use |
| Rope-based offset conversion | Manual offset arithmetic | `TextBuffer` methods (`line_to_offset`, `offset_to_position`, `position_to_offset`) | Already handles edge cases (last line, empty lines) |
| Multi-click detection | Custom timer | `event_loop.rs` `click_count` field | Already tracks consecutive click count in `MouseDownEvent.click_count` |
| Mouse capture | Custom state machine | `cx.capture_mouse()` + auto-release in event loop | Already implemented; event loop handles the release on `MouseUp` |
| Hit testing | Custom rect testing | `hit_test()` in `ora::events::mouse` | Already used everywhere |

**Key insight:** The entire selection state machine (`Selection`, `SelectionSet`,
`EditorView.move_caret_to`, `EditorView.begin_selection`) is already built. Phase 13 is
about wiring mouse events to the existing machine, not rebuilding it.

## Common Pitfalls

### Pitfall 1: Stale Text Area Origin

**What goes wrong:** The pixel-to-doc calculation uses the text area's origin (top-left
corner). If the origin is captured at render time but the layout shifts (e.g., sidebar
toggled), subsequent mouse events use the wrong origin.

**Why it happens:** The text area origin must be read from the layout bounds at the
moment the hitbox is registered in `prepaint`, not stored in `TextAreaView` at `render`.

**How to avoid:** Capture `text_area_origin` from `cx.bounds(layout_id)` inside the
`prepaint` stage, not from a stored field.

**Warning signs:** Cursor jumps to wrong position after sidebar toggle or window resize.

### Pitfall 2: scroll_y_offset_px Missing from Pixel Calculation

**What goes wrong:** The text area shifts content up by `scroll_y_offset_px` (fractional
line offset for smooth scrolling). Ignoring this in pixel-to-doc mapping causes off-by-one
line errors when the user scrolled to a partial-line position.

**Why it happens:** `scroll_y_offset_px` is managed by the ora event loop's scroll
accumulator and is separate from the integer `scroll_y` line index.

**How to avoid:** The `pixel_to_doc` function must add `scroll_y_offset_px` to `local_y`
before dividing by `LINE_HEIGHT`.

**Warning signs:** Clicking on a line maps to the line above when the document is not
scrolled to a clean line boundary.

### Pitfall 3: Word Boundary VS Code Mismatch

**What goes wrong:** The existing `move_left_word`/`move_right_word` in `dispatcher.rs`
uses a simple whitespace-skip algorithm, not VS Code-style word boundaries. CONTEXT
specifies VS Code word boundaries: alphanumeric+underscore runs are words; punctuation
and whitespace are separate groups.

**Why it happens:** The existing implementation was a placeholder.

**How to avoid:** Rewrite word navigation to use the VS Code classification:
```rust
fn vs_code_char_class(c: char) -> u8 {
    if c.is_alphanumeric() || c == '_' { 0 }     // word
    else if c.is_whitespace() { 1 }               // whitespace
    else { 2 }                                     // punctuation
}
```
Move stops at class transitions, not just whitespace boundaries.

This affects both keyboard Ctrl+arrow AND double-click word selection. Both code paths
must use the same VS Code classification.

**Warning signs:** Ctrl+Right on `foo.bar` stops at `.` (correct), but current code
stops after the whole `foo.bar` chunk.

### Pitfall 4: Selection Not Cleared on Non-Extend Click

**What goes wrong:** Clicking somewhere without Shift leaves the old selection visible.

**Why it happens:** `ClickAt { extend_selection: false }` must call
`view.move_caret_to(offset, false)` which calls `selections.collapse_to(offset)`.
If someone dispatches `MoveCursor` instead, the collapse happens correctly — but
using a custom path might miss it.

**How to avoid:** Route through `EditorView.move_caret_to(offset, false)` for all
click-to-position operations. This automatically collapses the selection.

**Warning signs:** Old selection remains highlighted after clicking elsewhere.

### Pitfall 5: arboard Clipboard on Wayland Without Display

**What goes wrong:** `arboard::Clipboard::new()` can fail on headless or misconfigured
Wayland setups. The existing code handles this with `.ok()` (stores `Option<Clipboard>`).

**Why it happens:** Platform clipboard requires a live display connection.

**How to avoid:** Already handled — `CommandDispatcher` stores `Option<Clipboard>`
and guards all clipboard ops with `if let Some(ref mut cb)`. No change needed.

**Warning signs:** Copy/paste silently does nothing on some Linux configurations.
This is already the behavior and is acceptable.

### Pitfall 6: Drag Selection Fires Before Click-to-Position

**What goes wrong:** A normal single-click triggers `MouseDown` → `MouseMove` (cursor
moves slightly) → `MouseUp`. If `DragTo` fires for tiny cursor movements, it creates
unwanted micro-selections.

**Why it happens:** Mouse doesn't hold perfectly still during a click.

**How to avoid:** Apply a minimum drag threshold (e.g., 3 pixels) before treating
movement as a drag selection. Only start drag mode after cursor moves > threshold from
the click position.

**Warning signs:** Clicking without moving creates a tiny 1-character selection.

## Code Examples

Verified patterns from actual codebase:

### Registering a Hitbox and Mouse Handler (from tab.rs)

```rust
// Source: ora/src/elements/tab.rs
let hitbox_id = cx.register_hitbox(bounds, true);
cx.on_mouse_down(hitbox_id, move |event, ctx| {
    if ctx.phase() != crate::events::dispatch::DispatchPhase::Bubble { return; }
    if matches!(event.button, MouseButton::Left) {
        // Handle click
    }
});
```

### Moving Caret and Managing Selection (from EditorView)

```rust
// Source: core_editor/src/view/editor_view.rs
// Collapse selection and move caret (single click, no extend):
view.move_caret_to(new_offset, false);  // collapses selection internally

// Extend selection (Shift+click or drag):
view.move_caret_to(new_offset, true);   // extends selection from anchor

// Begin selection at current position before extend:
view.begin_selection();                  // sets anchor = head = caret_offset
view.move_caret_to(new_offset, true);
```

### Existing Copy Handler (in CommandDispatcher — needs enrichment)

```rust
// Source: core_editor/src/commands/dispatcher.rs
fn handle_copy(&mut self, ctx: &CommandContext) {
    if let Some((start, end)) = ctx.view.selection_range() {
        let text = ctx.document.slice(TextRange::new(start, end));
        if let Some(ref mut cb) = self.clipboard {
            let _ = cb.set_text(text);
        }
    }
    // Currently: does nothing when no selection.
    // Phase 13 enrichment: copy current line when no selection.
}
```

### Offset Conversion (from TextBuffer)

```rust
// Source: core_editor/src/domain/text_buffer.rs
let line_idx = buffer.offset_to_line(char_offset);
let line_start = buffer.line_to_offset(line_idx);
let line_len = buffer.line_len(line_idx);
let pos = buffer.offset_to_position(char_offset);  // -> TextPosition { line, column }
let offset = buffer.position_to_offset(TextPosition { line, column });
```

### VS Code Word Boundary Classification

```rust
// Recommended pattern (not yet in codebase, to be added in dispatcher.rs)
fn vs_code_char_class(c: char) -> u8 {
    if c.is_alphanumeric() || c == '_' { 0 }  // word char
    else if c == '\n' { 3 }                    // line boundary — always stops
    else if c.is_whitespace() { 1 }            // whitespace
    else { 2 }                                 // punctuation
}
```

## State of the Art

| Old Approach | Current Approach | Notes |
|--------------|------------------|-------|
| Mouse selection not implemented | Phase 13 adds it | No existing mouse-to-cursor code exists |
| `handle_copy` copies selection only | Enrich: copy line if no selection | VS Code behavior specified in CONTEXT |
| Word movement uses whitespace-only | Rewrite with VS Code char classes | CONTEXT specifies VS Code word boundaries |
| Selection color is single `ColorToken::Selection` | Add `SelectionInactive` token | For focus-dimmed state |

**Deprecated/outdated:**
- Current `move_left_word`/`move_right_word`: replaces with VS Code class-based algorithm.

## Open Questions

1. **TextAreaView mouse handler attachment point**
   - What we know: `TextAreaView` implements `View` (only `render()`). Mouse handlers
     require `prepaint` context. The `tab.rs` pattern uses the `Element` trait lifecycle.
   - What's unclear: whether it's simpler to convert `TextAreaView` to an `Element`,
     or to create a new `TextAreaInteractiveWrapper` element that contains the existing
     view's render output.
   - Recommendation: Create a thin wrapper element (e.g., `TextAreaInteractive`) that
     registers hitboxes in `prepaint` and delegates to `TextAreaView::render` in `paint`.
     This avoids restructuring the existing view.

2. **Gutter click hitbox overlap**
   - What we know: gutter and text area are siblings in a `flex_row`. The gutter has its
     own bounds separate from the text area. A gutter click needs to be detected and
     treated as "select whole line".
   - What's unclear: whether gutter needs its own separate mouse handler or whether
     the text area handler should check whether the click is within gutter bounds.
   - Recommendation: Give the gutter view its own `on_mouse_down` handler that dispatches
     `GutterClickAt { line }`. Simpler than sharing logic with the text area handler.

3. **scroll_y_offset_px access from within the paint callback**
   - What we know: `scroll_y_offset_px` is a shared `Rc<Cell<f32>>` passed to
     `EditorRootView`. The text area view currently reads this from the `RenderModel`.
   - What's unclear: whether the callback (registered in prepaint) can still access
     the correct `scroll_y_offset_px` at the time `mouse_down` fires (it fires between
     frames, not at paint time).
   - Recommendation: Store the most recent `scroll_y_offset_px` in a shared `Rc<Cell<f32>>`
     accessible to the mouse handler closure, similar to how `SharedScrollOffset` works
     for rendering.

4. **Focus state for selection dimming**
   - What we know: `EditorView` in core_editor does not track "has OS keyboard focus".
     The `FocusState` enum in `view/mod.rs` tracks Editor vs. Sidebar focus.
   - What's unclear: whether editor-vs-sidebar focus (already in `FocusState`) is the
     correct signal, or whether a separate "window focus lost" signal is needed.
   - Recommendation: Use `FocusState::Editor` vs. anything else as the signal for
     full vs. dimmed selection rendering. Add a `has_focus: bool` field to `RenderModel`
     and use it in `TextAreaView` to select between `Selection` and `SelectionInactive`
     color tokens.

## Sources

### Primary (HIGH confidence)

- Direct codebase reading — all findings cited to specific files and line numbers
- `core_editor/src/view/selection.rs` — `Selection`, `SelectionSet` API
- `core_editor/src/view/editor_view.rs` — `move_caret_to`, `begin_selection`, `select_all`
- `core_editor/src/commands/dispatcher.rs` — all clipboard handlers, word movement
- `ora/src/platform/event_loop.rs` — click_count tracking, mouse capture, scroll
- `ora/src/elements/tab.rs` — `on_mouse_down` hitbox pattern
- `ora/src/views/text_area.rs` — rendering constants (LINE_HEIGHT=21.0)
- `ora/src/rendering/mod.rs` — `measured_char_width()` API
- `core_editor/src/domain/text_buffer.rs` — coordinate conversion API

### Secondary (MEDIUM confidence)

- VS Code full-line copy/cut behavior: widely documented community knowledge
  (paste-above for line copies, line selection on Ctrl+C with no selection)

### Tertiary (LOW confidence)

- Auto-indent paste algorithm: no official specification; described behavior
  matches common editor implementations but should be verified against VS Code.
- VS Code metadata convention for line-copy clipboard: not independently verified
  against VS Code source. The custom `clipboard_is_line_copy: bool` approach is
  more reliable for this codebase.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new deps; existing arboard confirmed working
- Architecture: HIGH — all patterns verified in actual codebase files
- Pitfalls: HIGH for mouse origin, scroll offset, dispatch phase guard, selection
  collapse; MEDIUM for word boundary mismatch (known gap but easy to fix)
- Clipboard semantics: MEDIUM — VS Code behavior well-known but auto-indent details LOW

**Research date:** 2026-03-26
**Valid until:** 2026-04-25 (stable; all relevant libs are already versioned in Cargo.toml)
