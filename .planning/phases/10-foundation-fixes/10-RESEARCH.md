# Phase 10: Foundation Fixes - Research

**Researched:** 2026-03-26
**Domain:** Winit event loop, GPU rendering, Rust trait design, selection rendering
**Confidence:** HIGH

## Summary

All five fixes in Phase 10 have been traced directly to specific lines in the
existing codebase. The root causes are concrete and the change surfaces are
small. No new libraries are needed. The work is primarily surgical edits to
`event_loop.rs`, `text_area.rs`, `editor_adapter/mod.rs`, and
`editor_adapter/types.rs`.

The idle GPU waste (FIX-01) and caret blink timer (FIX-05) are caused by the
same two lines: unconditional `request_redraw()` calls on lines 549 and 567 of
`event_loop.rs`, paired with the total absence of `ControlFlow::Wait`. Removing
those two calls and adding `ControlFlow::WaitUntil` in `about_to_wait` fixes
both. The selection rendering bug (FIX-02) is caused by `render_selection_rects`
existing but never being called in `TextAreaView::render()`, while selected
text spans are filtered out of the text layer. Command gaps (FIX-03) and the
monolithic trait (FIX-04) are additive changes with no hidden complexity.

**Primary recommendation:** Fix the two `request_redraw()` calls first — this
unblocks testing of every other fix by making the editor's frame cadence
observable.

## Standard Stack

### Core (already in use — no new dependencies)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| winit | current | Window + event loop | Already used; provides `ControlFlow::WaitUntil` |
| wgpu | current | GPU rendering | Already used |
| std::time::Instant | std | Timer for blink wakeup | Zero-cost, no deps |

### Key Winit APIs for this phase
| API | Location | Purpose |
|-----|----------|---------|
| `ControlFlow::Wait` | `winit::event_loop` | Sleep until next event |
| `ControlFlow::WaitUntil(Instant)` | `winit::event_loop` | Sleep until timed wakeup |
| `ControlFlow::Poll` | `winit::event_loop` | Busy loop (for active animations) |
| `event_loop.set_control_flow(...)` | `ActiveEventLoop` | Called in `about_to_wait` |

**No new crate dependencies required for any fix in this phase.**

## Architecture Patterns

### FIX-01 + FIX-05: Event-Driven Redraw with Timer Wakeup

**The bug (event_loop.rs lines 549, 567):**
```rust
// REMOVE THESE — they cause the infinite render loop:
gpu_state.window.request_redraw();  // line 549, inside RedrawRequested success
gpu_state.window.request_redraw();  // line 567, inside empty-frame path
```

**The fix — rewrite `about_to_wait`:**
```rust
fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
    while self.app_context.tick_executor() {}

    // Check if any transitions are still running (need continuous frames).
    let has_active_animations = self.app_context.has_active_transitions();

    if self.app_context.has_dirty_entities() || has_active_animations {
        // Need another frame immediately.
        event_loop.set_control_flow(ControlFlow::Poll);
        if let Some(gpu_state) = &self.gpu_state {
            gpu_state.window.request_redraw();
        }
    } else {
        // Sleep until caret blink time, or indefinitely if no caret.
        let next_wake = self.next_caret_wake_instant();
        match next_wake {
            Some(instant) => {
                event_loop.set_control_flow(ControlFlow::WaitUntil(instant));
            }
            None => {
                event_loop.set_control_flow(ControlFlow::Wait);
            }
        }
    }
}

fn next_caret_wake_instant(&self) -> Option<Instant> {
    // CaretElement::time_until_next_blink() already exists on CaretElement
    // but the instance lives only during paint. Use the global statics
    // BLINK_EPOCH and LAST_ACTIVITY (already in caret.rs) to compute
    // the wake time without needing a CaretElement instance.
    //
    // Simplest approach: compute directly using the same logic as
    // CaretElement::time_until_next_blink() but with the global statics.
    use crate::elements::{BLINK_RATE, ACTIVITY_TIMEOUT};
    // ... (implementation uses Instant::now() + computed delay)
    Some(Instant::now() + delay)
}
```

**Alternative (simpler):** Store a `next_blink_at: Option<Instant>` on `OraApp`
and update it each time `notify_caret_activity()` is called. The `about_to_wait`
reads it directly. This avoids coupling to the global statics.

**`about_to_wait` already has `_event_loop: &ActiveEventLoop`** — just remove
the underscore to use it.

### Pattern: ControlFlow State Machine

```
Idle (no input, no animation, caret between blinks)
  → ControlFlow::WaitUntil(next_blink_instant)

Active animation running
  → ControlFlow::Poll (continuous frames until animation completes)

Input received (key, mouse)
  → ControlFlow::Poll for that frame → back to Wait/WaitUntil

Caret blink tick
  → about_to_wait fires, request_redraw(), then WaitUntil(next_blink)
```

### FIX-02: Selection Rendering

**The bug in `text_area.rs`:**

Three problems working together:

1. `render()` (line 260) builds `text_lines` and `caret` but **never calls
   `render_selection_rects()`**. The method is dead code.

2. `render_line()` (line 219) filters out Selection spans:
   ```rust
   .filter(|span| span.style != TextStyle::Selection)  // selection text vanishes
   ```
   This was intended so that selection text doesn't double-render, but since
   selection rects are never drawn, text in the selection range disappears
   entirely.

3. The Stack layers text and caret, but has no selection layer.

**The fix:**

The correct approach (per context: "preserves syntax highlighting") is:
- Selection background renders as a separate layer **below** text
- Text renders with its normal syntax colors regardless of selection
- Selected text should NOT be filtered out of the text layer

The core data model needs to be checked: do the `StyledSpan` sequences from
`core_editor` represent selection by replacing spans with `TextStyle::Selection`
spans (losing the syntax color), or by providing selection range information
separately? This is the key question for the data layer.

If `core_editor` replaces spans with Selection-styled spans (losing syntax info),
then FIX-02 requires a change to how `core_editor` sends selection data — the
selection range must come through the data pipe separately from styled spans.

If `core_editor` sends selection as a separate range (not embedded in spans),
then the fix is entirely in `text_area.rs`: add a selection rects layer.

**Research finding:** The `LinePresentation` type only carries `Vec<StyledSpan>`.
There is no separate selection range field. This means the current design mixes
selection state into the span list, which is why syntax highlighting cannot be
preserved with the current data model. The fix must either:

Option A: Add `selection_ranges: Vec<(usize, usize)>` to `LinePresentation` and
have `core_editor` populate it, then render selection rects from those ranges
(text renders normally with syntax colors, selection rects overlay from below).

Option B: Keep the span-based approach but also carry the original syntax style
alongside — i.e., `StyledSpan { text, syntax_style, is_selected }`.

**Recommendation: Option A.** It cleanly separates concerns. The render method
becomes:

```rust
// Stack layers in order (bottom to top):
// 1. Current-line highlight
// 2. Selection background rectangles (from selection_ranges, not span styles)
// 3. Text layer (all spans with normal syntax colors, no filtering)
// 4. Caret

fn render_selection_rects(&self, cx: &mut ViewContext) -> AnyElement {
    // Build from self.selection_ranges (new field on TextAreaView)
    // Each range: absolute pixel rect covering selected columns × line height
    // Full-width at line end if multi-line selection (edge-to-edge per context)
}
```

### FIX-03: v2 Command Variants

**Add to `EditorCommand` enum in `editor_adapter/types.rs`:**

```rust
// --- Tab management ---
/// Switch to a specific tab by view ID (Ctrl+Tab or Ctrl+1..9)
SwitchTab(u64),
/// Close the current tab (Ctrl+W)
CloseTab,

// --- File operations ---
/// Open a file (Ctrl+O) — shows file picker
OpenFile,
/// Save As (Ctrl+Shift+S) — shows save dialog
SaveAs,
/// New file (Ctrl+N)
New,

// --- Search ---
/// Find in file (Ctrl+F)
Find,
/// Replace in file (Ctrl+H)
Replace,
/// Replace All (no standard keybinding yet)
ReplaceAll,

// --- Navigation ---
/// Go to line (Ctrl+G)
GoToLine,
```

**Add to `translate_editor_command` in `events/editor_input.rs`:**

```rust
Key::Character(c) if ctrl => match c.as_str() {
    // ... existing ...
    "w" | "W" => Some(EditorCommand::CloseTab),
    "o" | "O" => Some(EditorCommand::OpenFile),
    "n" | "N" => Some(EditorCommand::New),
    "f" | "F" => Some(EditorCommand::Find),
    "h" | "H" => Some(EditorCommand::Replace),
    "g" | "G" => Some(EditorCommand::GoToLine),
    // SaveAs needs Ctrl+Shift+S — check shift modifier
    "s" | "S" if shift => Some(EditorCommand::SaveAs),
    "s" | "S" => Some(EditorCommand::Save),  // existing, but now ordered after shift check
    _ => None,
},
```

**Stub handler pattern (in `wgpu_client` dispatch_command):**

```rust
EditorCommand::OpenFile => {
    // Not yet available — show status message
    self.set_status_message("Open File: not yet available");
}
```

The `StatusPresentation::message: Option<String>` field already exists for this.

### FIX-04: EditorDataSource Trait Split

**Current monolith (7 methods in `editor_adapter/mod.rs`):**
```
build_render_model, dispatch_command, resize_viewport,
viewport_lines, scroll_y, total_lines, window_title
```

**Proposed split:**

```rust
/// Read-only buffer/viewport data for rendering.
pub trait BufferDataSource {
    fn build_render_model(&self, viewport_lines: usize) -> RenderModel;
    fn resize_viewport(&mut self, width_chars: usize, height_lines: usize);
    fn viewport_lines(&self) -> usize;
    fn scroll_y(&self) -> usize;
    fn total_lines(&self) -> usize;
}

/// Command dispatch.
pub trait CommandDispatcher {
    fn dispatch_command(&mut self, cmd: EditorCommand);
}

/// Window/chrome metadata.
pub trait WindowDataSource {
    fn window_title(&self) -> String;
}

/// Convenience super-trait: required by run_with_editor entry point.
pub trait EditorDataSource: BufferDataSource + CommandDispatcher + WindowDataSource {}

/// Blanket impl so any type implementing all three sub-traits gets EditorDataSource.
impl<T> EditorDataSource for T
where
    T: BufferDataSource + CommandDispatcher + WindowDataSource {}
```

**Consumer update:** Views that only read data receive `&dyn BufferDataSource`.
The event loop uses the full `EditorDataSource` bound since it both reads and
dispatches. The `SharedAdapter` type alias in `views/mod.rs` needs to change
from `Rc<RefCell<Box<dyn EditorDataSource>>>` to stay or be split — keep it as
`EditorDataSource` for the shared adapter since both halves are needed.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Timed event loop wakeup | Custom timer thread | `ControlFlow::WaitUntil` (winit) | winit already manages OS-level sleep; a separate thread wastes a thread and adds synchronization |
| Caret blink timing | Per-instance state | Global `BLINK_EPOCH` static (already exists) + `WaitUntil` | The epoch static ensures blink phase survives element recreation; don't add per-frame state |
| Selection pixel rects | Custom rect compositor | Standard paint commands (existing `cx.paint_styled_rect`) | The painting layer already handles rect rendering |
| Trait object dispatch | Pattern matching on enum | `dyn Trait` (sub-traits) | Rust's trait object dispatch is correct for this boundary |

**Key insight:** All infrastructure for these fixes already exists. The work
is removing code (unconditional redraws), wiring up existing code (calling
`render_selection_rects`), and adding enum variants.

## Common Pitfalls

### Pitfall 1: Removing All `request_redraw()` Calls
**What goes wrong:** Some `request_redraw()` calls in `window_event` are correct
(mouse moves, keyboard input, scroll). Only the ones INSIDE `RedrawRequested`
success paths are wrong.
**Why it happens:** All redraw calls look the same; the context matters.
**How to avoid:** Only remove the two calls at lines 549 and 567 (the ones with
the "Request continuous redraw for now" comment). Keep all calls in event
handlers.
**Warning signs:** If mouse hover stops updating visually, a needed redraw call
was removed.

### Pitfall 2: Selection Highlight Z-Order
**What goes wrong:** Selection rects paint over text, making text invisible.
**Why it happens:** Stack paints layers in child order; if selection is on top
of text, it covers the text.
**How to avoid:** Stack order must be: `[text_layer, selection_layer, caret_layer]`
where selection goes above text but below caret, AND text must render with its
own syntax colors regardless. Actually, the correct Z order is:
`[current_line_bg, selection_bg, text_layer, caret]` — selection rects are
BEHIND text so syntax highlighting remains visible.

### Pitfall 3: SaveAs Keybinding Ordering
**What goes wrong:** `Ctrl+S` matches before `Ctrl+Shift+S` if not ordered correctly.
**Why it happens:** Match arms are checked in order; `"s" | "S"` matches both.
**How to avoid:** Check `shift` modifier first: `"s" | "S" if shift => SaveAs`
before `"s" | "S" => Save`.

### Pitfall 4: Trait Split Breaks wgpu_client
**What goes wrong:** `wgpu_client`'s adapter struct implements `EditorDataSource`;
after the split it needs to implement all three sub-traits.
**Why it happens:** The blanket impl only works if all three traits are
implemented.
**How to avoid:** Implement all three sub-traits on the adapter struct in
`wgpu_client`, then the blanket `impl<T> EditorDataSource for T where T: ...`
handles the rest. No manual impl of `EditorDataSource` needed.

### Pitfall 5: CursorMoved Redraw on Every Mouse Move
**What goes wrong:** After FIX-01, the editor still redraws on every mouse
micro-movement because `CursorMoved` unconditionally calls `request_redraw()`.
**Why it happens:** That call is in the existing code and was not part of the
infinite loop, but it does cause many unnecessary redraws during mouse movement.
**How to avoid:** This is a secondary optimization; consider only requesting
redraw on actual hover state change (already tracked via `prev_hover != hit_id`).
Scoping it out of Phase 10 is acceptable — the goal is eliminating idle waste,
not optimizing mouse movement.

### Pitfall 6: Selection Color Too Dim
**What goes wrong:** Current `blue_900` (`#0D3870`, 5% / 22% / 44% RGB) is
very dark on a dark background (`gray_900` = `#212121`). The contrast ratio
is very low, making selection nearly invisible.
**Why it happens:** The color was chosen but never tested with real selection
rendering (since the rendering was broken).
**How to avoid:** Per context, "Zed-level contrast/brightness". Zed uses
approximately `#264F78` (VS Code) or a brighter blue with ~40% opacity. A
concrete starting point: try `Color::rgb(0.15, 0.31, 0.50)` for dark mode
selection — visibly brighter than `blue_900` while remaining distinct from
the caret/accent color.

## Code Examples

### Setting ControlFlow in about_to_wait

```rust
// Source: winit 0.30 ApplicationHandler docs
fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
    while self.app_context.tick_executor() {}

    if self.app_context.has_dirty_entities() {
        event_loop.set_control_flow(ControlFlow::Poll);
        if let Some(gpu_state) = &self.gpu_state {
            gpu_state.window.request_redraw();
        }
        return;
    }

    // Schedule next caret blink wakeup
    if let Some(next) = self.next_blink_instant {
        if next > Instant::now() {
            event_loop.set_control_flow(ControlFlow::WaitUntil(next));
        } else {
            // Blink instant passed — request redraw and schedule next
            if let Some(gpu_state) = &self.gpu_state {
                gpu_state.window.request_redraw();
            }
            self.next_blink_instant = Some(Instant::now() + BLINK_RATE);
            event_loop.set_control_flow(
                ControlFlow::WaitUntil(self.next_blink_instant.unwrap())
            );
        }
    } else {
        event_loop.set_control_flow(ControlFlow::Wait);
    }
}
```

### Tracking next blink instant on OraApp

```rust
// Add to OraApp struct:
next_blink_instant: Option<Instant>,

// In notify_caret_activity() call sites (after dispatching editor command):
crate::elements::notify_caret_activity();
self.next_blink_instant = Some(
    Instant::now() + ACTIVITY_TIMEOUT + BLINK_RATE
);
```

### Selection background render pattern

```rust
// In TextAreaView::render(), build the selection layer:
fn render_selection_bg_layer(&self, cx: &mut ViewContext) -> AnyElement {
    let theme = cx.theme();
    let selection_color = theme.color(ColorToken::Selection);

    // One rect per selected region per line
    let mut children: Vec<AnyElement> = Vec::new();
    for (line_index, line) in self.visible_lines.iter().enumerate() {
        if let Some((start_col, end_col)) = line.selection_range {
            let x = start_col as f32 * self.char_width;
            let width = (end_col - start_col) as f32 * self.char_width;
            let y = line_index as f32 * LINE_HEIGHT;
            // Use absolutely positioned Div — requires Position::Absolute support
            // (already exists in the layout system per stack.rs comments)
            let rect = Div::new()
                .absolute()
                .left(x)
                .top(y)
                .w(px(width))
                .h(px(LINE_HEIGHT))
                .bg(selection_color);
            children.push(rect.into());
        }
    }
    Div::new().w(pct(100.0)).h(pct(100.0)).children(children).into()
}
```

### Trait split — minimal impl in wgpu_client

```rust
impl BufferDataSource for CoreEditorAdapter {
    fn build_render_model(&self, viewport_lines: usize) -> RenderModel { ... }
    fn resize_viewport(&mut self, w: usize, h: usize) { ... }
    fn viewport_lines(&self) -> usize { ... }
    fn scroll_y(&self) -> usize { ... }
    fn total_lines(&self) -> usize { ... }
}

impl CommandDispatcher for CoreEditorAdapter {
    fn dispatch_command(&mut self, cmd: EditorCommand) { ... }
}

impl WindowDataSource for CoreEditorAdapter {
    fn window_title(&self) -> String { ... }
}
// EditorDataSource impl comes for free from the blanket impl
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Continuous redraw in game loop | Event-driven + WaitUntil timer | winit 0.28+ | Eliminates idle GPU/CPU usage |
| One mega-trait for editor boundary | Sub-traits by domain | Rust best practice | Explicit dependencies, easier testing |

**Deprecated/outdated patterns in this codebase:**
- `// Request continuous redraw for now (don't break existing demos)` comment on
  line 549 — this was a deliberate temporary hack, now to be removed.

## Open Questions

1. **Selection data format in core_editor**
   - What we know: `LinePresentation` only has `Vec<StyledSpan>`; Selection
     spans replace text spans, losing syntax style info
   - What's unclear: Whether `core_editor` can be changed to emit
     `selection_ranges: Vec<(usize, usize)>` per line alongside normal spans
   - Recommendation: Check `core_editor`'s view model — if it already computes
     selection ranges, expose them. If not, add `selection_ranges` to
     `LinePresentation` in `editor_adapter/types.rs` and update wgpu_client's
     conversion code.

2. **Absolute positioning in Div for selection rects**
   - What we know: `Stack` uses absolute positioning for its children (per
     stack.rs comment), and `Position::Absolute` exists in the layout system
   - What's unclear: Whether `Div` has a public `.absolute()` / `.left()` /
     `.top()` builder API yet
   - Recommendation: Check `elements/div.rs` for absolute positioning API
     before planning the selection rect render approach. If absent, use the
     Stack layer approach with `mt()` offsets per line instead.

3. **TransitionRegistry active-state query**
   - What we know: `TransitionRegistry` exists in `animation/transition.rs`;
     `AppContext` owns it
   - What's unclear: Whether `AppContext` exposes a `has_active_transitions()`
     query method
   - Recommendation: If the method doesn't exist, add it to `AppContext` in
     this phase. The planner should add this task.

## Sources

### Primary (HIGH confidence)
- Direct code inspection: `ora/src/platform/event_loop.rs` — lines 549, 567
  (the two unconditional request_redraw calls)
- Direct code inspection: `ora/src/views/text_area.rs` — render() method
  (render_selection_rects never called; Selection spans filtered from text)
- Direct code inspection: `ora/src/editor_adapter/types.rs` — EditorCommand
  enum (missing SwitchTab, CloseTab, OpenFile, SaveAs, New, Find, Replace,
  ReplaceAll, GoToLine)
- Direct code inspection: `ora/src/editor_adapter/mod.rs` — EditorDataSource
  monolithic trait
- Direct code inspection: `ora/src/elements/caret.rs` — BLINK_EPOCH, LAST_ACTIVITY
  globals, `time_until_next_blink()` method
- Direct code inspection: `ora/src/theme/mod.rs` — Selection = blue_900 (#0D3870)
- Direct code inspection: `ora/src/theme/color.rs` — color values

### Secondary (MEDIUM confidence)
- Winit ApplicationHandler docs: `ControlFlow::WaitUntil` is the standard
  timer-driven wakeup pattern for game loops / rendering apps that need
  idle sleep with periodic wakeup
- Context7/winit: `set_control_flow` called on `ActiveEventLoop` in
  `about_to_wait` is the documented pattern

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all code inspected directly; no new libraries needed
- Architecture (FIX-01/05): HIGH — root cause lines identified exactly
- Architecture (FIX-02): HIGH for the bug identification; MEDIUM for the
  data model fix (depends on core_editor capability not inspected)
- Architecture (FIX-03): HIGH — additive enum changes
- Architecture (FIX-04): HIGH — clean trait split with blanket impl
- Pitfalls: HIGH — derived from code inspection, not speculation

**Research date:** 2026-03-26
**Valid until:** 2026-04-25 (stable codebase, no external library churn expected)
