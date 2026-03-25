# Phase 8: Advanced UI & Widgets - Research

**Researched:** 2026-03-02
**Domain:** GPUI-style advanced UI components and widget primitives in the ora framework
**Confidence:** HIGH

## Summary

Phase 8 builds on an already mature ora framework (Phases 1-7 complete) to add four advanced UI views (CommandPalette, FileTree, PanelManager, AppLayout) and eight widget primitives (Button-enhanced, Input, Checkbox, Toggle, ListItem, Tab, TreeItem, ContextMenu, Toast). Research reveals all these components can be composed from existing ora primitives (Div, TextElement, Button, Stack, Image) with no new external dependencies.

The existing `Button` element in `ora/src/elements/button.rs` already implements the variant-based styling pattern (ButtonVariant, ButtonState), size-less but functionally complete. Phase 8 must extend it with size tiers (sm/md/lg), add new widget types following the same three-phase lifecycle (request_layout, prepaint, paint), and compose them into more sophisticated views. The `SidebarView` placeholder in `ora/src/views/sidebar.rs` explicitly notes "Phase 8 will replace this with FileTree component" — a clear handoff point.

Key architectural insight: every component in this phase follows the same well-established pattern — stateless views consuming presentation data from Model<T>, producing Div/TextElement trees, using theme.color(ColorToken::*) for colors, and FocusHandle for keyboard focus. The only new complexity is drag-for-resize (PanelManager), fuzzy search matching (CommandPalette), and auto-dismiss timers (Toast). All three are achievable using existing `cx.capture_mouse()`, `cx.spawn()`, and the InteractionState APIs already in AppContext.

**Primary recommendation:** Build all Phase 8 components as ora Views/Elements in `ora/src/views/` and `ora/src/elements/`, using only existing infrastructure. No new dependencies. Follow the three-phase lifecycle and theme token pattern established in Phases 5-7.

## Standard Stack

### Core (Already Implemented — No New Dependencies)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| wgpu | 23.0 | GPU rendering | Already integrated, powers all rendering |
| glyphon | 0.7 | Text rendering | Already integrated, TextElement wraps it |
| winit | 0.30 | Windowing/events | Already integrated, owns event loop |
| async-executor | (existing) | Async tasks | cx.spawn() for Toast auto-dismiss timer |

### Supporting (Already in ora)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| generational-arena | 0.2 | Entity storage | Model<T> for component state |
| image | 0.24 | Image decoding | File-type icons if loading from bytes |

### New Dependencies Required
None. Phase 8 requires zero new Cargo.toml entries. All infrastructure exists.

**Fuzzy matching:** Hand-roll a simple substring/character-sequence scorer in Rust. The algorithm needed for CommandPalette is straightforward (20-30 lines) and doesn't warrant a dependency. Pattern: iterate query chars, find them in sequence in the target string, score by contiguity.

**Installation:**
```bash
# No new packages - all dependencies already in ora/Cargo.toml
```

## Architecture Patterns

### Recommended Project Structure
```
ora/src/
├── elements/
│   ├── button.rs           # Extend: add size tier support
│   ├── div.rs              # Unchanged
│   ├── text.rs             # Unchanged
│   ├── stack.rs            # Unchanged
│   └── mod.rs              # Add new widget re-exports
└── views/
    ├── sidebar.rs          # Extend: replace placeholder with FileTree content
    ├── tab_bar.rs          # Unchanged (Phase 7)
    ├── command_palette.rs  # NEW: UI-01 CommandPalette
    ├── file_tree.rs        # NEW: UI-02 FileTree
    ├── panel_manager.rs    # NEW: UI-03 PanelManager
    ├── app_layout.rs       # NEW: UI-04 AppLayout
    └── mod.rs              # Add new view re-exports

ora/src/widgets/            # NEW directory for widget primitives
    ├── input.rs            # WIDGET-02 Input
    ├── checkbox.rs         # WIDGET-03 Checkbox + Toggle
    ├── list_item.rs        # WIDGET-04 ListItem
    ├── tab.rs              # WIDGET-05 Tab
    ├── tree_item.rs        # WIDGET-06 TreeItem
    ├── context_menu.rs     # WIDGET-07 ContextMenu
    ├── toast.rs            # WIDGET-08 Toast
    └── mod.rs
```

Alternative: keep widgets in `elements/` if the new directory adds complexity. The existing Button lives in `elements/button.rs` — consistency suggests keeping new widgets there too. Both approaches work; the planner should pick one and stay consistent.

### Pattern 1: Three-Phase Lifecycle for New Widgets

**What:** All new widget elements implement `Element` trait with request_layout/prepaint/paint phases.
**When to use:** Every widget primitive (Input, Checkbox, ListItem, Tab, TreeItem, ContextMenu, Toast).
**Example — Input widget skeleton:**

```rust
// Source: ora/src/elements/button.rs (established pattern)
use crate::element::{Element, LayoutContext, LayoutId, PaintContext, PrepaintContext};
use crate::events::focus::FocusHandle;
use crate::events::mouse::HitboxId;
use crate::style::*;
use crate::theme::{ColorToken, Theme};

pub enum WidgetSize { Sm, Md, Lg }

pub struct Input {
    placeholder: String,
    value: String,
    size: WidgetSize,
    disabled: bool,
    focus_handle: Option<FocusHandle>,
    // Store child TextElement for lifecycle control (Pattern from 05-03 Button decision)
    text_element: Option<TextElement>,
}

pub struct InputState {
    layout_id: LayoutId,
    text_layout_id: LayoutId,
    text_state: TextState,
    hitbox_id: Option<HitboxId>,
    focus_id: Option<FocusId>,
}

impl Element for Input {
    type RequestLayoutState = InputState;

    fn request_layout(&mut self, cx: &mut LayoutContext) -> (LayoutId, InputState) {
        let style = self.build_style(); // compute Style from size tier
        let layout_id = cx.request_layout(&style);
        // Create text element, request its layout
        // Store in self.text_element for paint phase
        // ...
    }

    fn prepaint(&mut self, state: &mut InputState, cx: &mut PrepaintContext) {
        let bounds = cx.bounds(state.layout_id);
        let hitbox_id = cx.register_hitbox(bounds, true);
        state.hitbox_id = Some(hitbox_id);
        if let Some(fh) = &self.focus_handle {
            cx.register_focusable(fh.id);
            state.focus_id = Some(fh.id);
        }
        // prepaint text child
    }

    fn paint(&mut self, state: &mut InputState, cx: &mut PaintContext) {
        let bounds = cx.bounds(state.layout_id);
        let is_focused = state.focus_id.map(|id| cx.is_focused(id)).unwrap_or(false);
        let is_hovered = state.hitbox_id.map(|id| cx.is_hovered(id)).unwrap_or(false);

        // Borrow-safe pattern (STATE.md 07-01 decision): get all data from cx before theme reference
        let bg = cx.theme().color(if self.disabled { ColorToken::BgSecondary } else { ColorToken::BgPrimary });
        let border_color = cx.theme().color(if is_focused { ColorToken::Accent } else { ColorToken::Border });

        let mut style = Style::default();
        style.background = Background::Solid(bg);
        style.border.color = border_color;
        style.border.widths = Edges::all(1.0);
        style.border_radius = Corners::all(4.0);

        cx.paint_styled_rect(&style, &bounds);
        // paint text element
    }
}
```

### Pattern 2: Size Tier System (sm/md/lg)

**What:** Three named size configurations controlling padding, font-size, and min-height.
**When to use:** All widgets — Button (extended), Input, Checkbox, ListItem, Tab, TreeItem.
**Locked in CONTEXT.md:** sm=compact (tree items, tabs, status bar), md=standard (buttons, inputs, checkboxes), lg=prominent (dialog buttons, CTAs).

```rust
// Source: CONTEXT.md decisions + Button pattern in ora/src/elements/button.rs
#[derive(Clone, Copy, Debug, Default)]
pub enum WidgetSize {
    Sm,        // compact: height=24px, font=12px, padding_h=8px, padding_v=4px
    #[default]
    Md,        // standard: height=32px, font=13px, padding_h=12px, padding_v=6px
    Lg,        // prominent: height=40px, font=14px, padding_h=16px, padding_v=8px
}

impl WidgetSize {
    pub fn height(&self) -> f32 {
        match self { Sm => 24.0, Md => 32.0, Lg => 40.0 }
    }
    pub fn font_size(&self) -> f32 {
        match self { Sm => 12.0, Md => 13.0, Lg => 14.0 }
    }
    pub fn padding_h(&self) -> f32 {
        match self { Sm => 8.0, Md => 12.0, Lg => 16.0 }
    }
    pub fn padding_v(&self) -> f32 {
        match self { Sm => 4.0, Md => 6.0, Lg => 8.0 }
    }
}
```

Size tier heights align with existing constants: `TAB_BAR_HEIGHT=36px` (Md/Lg), `STATUS_BAR_HEIGHT=28px` (Sm+). The Md tier at 32px fits comfortably into both.

### Pattern 3: Fuzzy Match Algorithm for CommandPalette

**What:** Simple character-sequence matching with match-position highlighting.
**When to use:** CommandPalette search input processing.
**Locked in CONTEXT.md:** fuzzy matching, highlight matched chars in bold/accent color.

```rust
// Source: Standard fuzzy-search algorithm pattern (verified in multiple editors)
// No external crate needed — the algorithm is ~30 lines

pub struct FuzzyMatch {
    pub score: i32,
    pub match_positions: Vec<usize>, // character indices that matched
}

pub fn fuzzy_match(query: &str, target: &str) -> Option<FuzzyMatch> {
    if query.is_empty() {
        return Some(FuzzyMatch { score: 0, match_positions: vec![] });
    }

    let query_chars: Vec<char> = query.to_lowercase().chars().collect();
    let target_chars: Vec<char> = target.to_lowercase().chars().collect();

    let mut match_positions = Vec::new();
    let mut qi = 0; // query index
    let mut last_match = 0usize;
    let mut score = 0i32;

    for (ti, &tc) in target_chars.iter().enumerate() {
        if qi < query_chars.len() && tc == query_chars[qi] {
            match_positions.push(ti);
            // Bonus for consecutive matches (contiguous sequence scores higher)
            if !match_positions.is_empty() && ti == last_match + 1 {
                score += 10;
            } else {
                score += 1;
            }
            // Bonus for match at word boundary (space, _, -, uppercase after lowercase)
            if ti == 0 || target_chars[ti - 1] == ' ' || target_chars[ti - 1] == '_' {
                score += 5;
            }
            last_match = ti;
            qi += 1;
        }
    }

    if qi == query_chars.len() {
        Some(FuzzyMatch { score, match_positions })
    } else {
        None // Not all query chars found
    }
}

// Render highlighted text using match_positions:
// Characters at match_positions get ColorToken::Accent, others get ColorToken::FgPrimary
// This requires splitting TextElement into multiple spans (alternating matched/unmatched)
```

### Pattern 4: Drag-for-Resize (PanelManager)

**What:** Mouse drag on a thin resize handle Div adjusts panel height, double-click cycles presets.
**When to use:** PanelManager bottom panel resize handle (UI-03).
**Locked in CONTEXT.md:** drag handle + presets, double-click toggles collapsed/half/full, panel sizes persist.

The drag pattern uses the existing `cx.capture_mouse()` API in `AppContext` (already implemented in `ora/src/events/interaction.rs`).

```rust
// Source: ora/src/events/interaction.rs (MouseCapture), AppContext::capture_mouse()
// Drag resize pattern — state lives in Model<PanelState>

pub struct PanelState {
    pub height: f32,          // current panel height
    pub is_collapsed: bool,
    pub preferred_height: f32, // restored height when expanding
}

const PANEL_HEIGHT_COLLAPSED: f32 = 0.0;
const PANEL_HEIGHT_HALF: f32 = 200.0;  // ~half screen
const PANEL_HEIGHT_FULL: f32 = 400.0;  // full panel

// In the resize handle's prepaint:
// cx.on_mouse_down(hitbox_id, |evt, _| {
//     app_context.capture_mouse(hitbox_id, MouseButton::Left);
//     drag_start_y = evt.position.y;
//     drag_start_height = panel_height;
// });
//
// In window event loop:
// on MouseMove while captured: panel_height = drag_start_height + (drag_start_y - current_y)
// on MouseUp: release_mouse_capture(), persist height
// on DoubleClick handle: cycle between COLLAPSED / HALF / FULL
```

Panel size persistence: use `std::fs` to read/write a simple JSON/TOML file. Since this is Phase 8 (no explicit persistence infrastructure), store in a file at a predictable path or pass through AppContext. Simplest approach: serialize PanelState to a file via `serde_json` (already likely in workspace) or plain string format. If serde_json is not available, write a simple key=value format.

### Pattern 5: Toast Auto-Dismiss with cx.spawn()

**What:** Info/success toasts auto-dismiss after 3-5 seconds via async timer; error/warning persist.
**When to use:** WIDGET-08 Toast.
**Locked in CONTEXT.md:** severity-based (info/success = auto-dismiss, warning/error = persist), position Claude's discretion.

```rust
// Source: ora/src/context.rs (AppContext::spawn), async-executor LocalExecutor
// Toast component uses cx.spawn() for timer (Phase 3 pattern)

pub enum ToastSeverity { Info, Success, Warning, Error }

pub struct ToastState {
    pub message: String,
    pub severity: ToastSeverity,
    pub visible: bool,
}

// In the view that manages toasts:
fn show_toast(&self, msg: String, severity: ToastSeverity, cx: &mut ViewContext) {
    let model = self.toast_model.clone();
    cx.update_model(&model, |state, mcx| {
        state.message = msg;
        state.severity = severity;
        state.visible = true;
        mcx.notify();
    });

    // Auto-dismiss for info/success only
    match severity {
        ToastSeverity::Info | ToastSeverity::Success => {
            let model_clone = model.clone();
            cx.spawn(async move {
                // Wait 3 seconds using async sleep (async-std or tokio not needed,
                // just poll-based delay using Instant)
                let dismiss_at = std::time::Instant::now() + std::time::Duration::from_secs(3);
                while std::time::Instant::now() < dismiss_at {
                    // Yield to executor - in a real async executor, use Timer/sleep
                    // Simplest: check in about_to_wait hook, not in spawn
                }
                // Note: access to AppContext from spawned task is limited
                // Alternative: track dismiss_at: Option<Instant> in ToastState,
                // check in event loop's about_to_wait handler
            });
        }
        _ => {} // Warning/error: user must dismiss manually
    }
}
```

**Practical recommendation for Toast timer:** Instead of cx.spawn() (which lacks easy AppContext access from async context), track `dismiss_at: Option<Instant>` in `ToastState`. In the `about_to_wait` event loop hook (already used for caret blink), check if `Instant::now() >= dismiss_at` and update the model. This avoids async complexity entirely.

### Pattern 6: FileTree Hierarchical Rendering

**What:** Tree of file entries with indentation, expand/collapse toggles, indent guides, colored icons.
**When to use:** UI-02 FileTree view (replaces SidebarView placeholder content).
**Locked in CONTEXT.md:** arrow-click OR double-click to expand/collapse, single-click selects, multi-select (Ctrl/Shift), indent guides, colored file-type icons (Seti/Material style).

```rust
// Source: core_editor view_model (FileEntryPresentation), ora Div builder API

pub struct FileTreeEntry {
    pub name: String,
    pub is_dir: bool,
    pub is_expanded: bool,
    pub is_selected: bool,
    pub depth: usize,           // indentation level
    pub children: Vec<FileTreeEntry>,
}

pub struct FileTreeView {
    entries: Vec<FileTreeEntry>,
    selected: Vec<usize>,       // multi-select: indices
    focus_handle: FocusHandle,
}

// Render one tree row:
fn render_entry(&self, entry: &FileTreeEntry, cx: &mut ViewContext) -> Div {
    let theme = cx.theme();
    let indent_px = entry.depth as f32 * 16.0; // 16px per indent level
    let row_height = 24.0; // sm tier

    let bg = if entry.is_selected {
        theme.color(ColorToken::Accent)  // selection
    } else {
        theme.color(ColorToken::BgSecondary) // default
    };

    Div::new()
        .flex_row()
        .align_center()
        .h(px(row_height))
        .bg(bg)
        .hover_bg(theme.color(ColorToken::BgElevated))
        // Indent guides: draw vertical lines for each depth level
        // Each level adds a thin left-border Div
        .child(self.render_indent_guides(entry.depth, cx))
        // Expand/collapse arrow (only for dirs)
        .child(self.render_expand_arrow(entry, cx))
        // File icon (colored by extension)
        .child(self.render_file_icon(entry, cx))
        // File name text
        .child(TextElement::new(&entry.name)
            .size(12.0)
            .color(theme.color(ColorToken::FgPrimary)))
}

// Indent guides: stack of thin vertical lines
fn render_indent_guides(&self, depth: usize, cx: &mut ViewContext) -> Div {
    let theme = cx.theme();
    let mut row = Div::new().flex_row().h(pct(100.0));
    for _ in 0..depth {
        row = row.child(
            Div::new()
                .w(px(16.0))
                .h(pct(100.0))
                .flex_row()
                .child(
                    Div::new()
                        .w(px(1.0))
                        .h(pct(100.0))
                        .bg(theme.color(ColorToken::Border))
                        .m(7.5) // center in 16px slot
                )
        );
    }
    row
}
```

**File-type icon colors:** Map extension to a color using a lookup table. Don't use external icon fonts (not available). Use colored Unicode characters or simple colored squares as icon proxies. The Seti/Material icon mapping is a static table (`.rs` = orange, `.js` = yellow, `.ts` = blue, etc.) — hand-roll a `fn icon_color_for_extension(ext: &str) -> Color` function with ~20-30 extension cases.

### Pattern 7: AppLayout Bounds Orchestration

**What:** A top-level View that computes pixel bounds for sidebar, tab bar, editor, panel, status bar, and renders them as positioned Div children.
**When to use:** UI-04 AppLayout.
**Locked in CONTEXT.md:** instant toggle (no animation), panel sizes persist, TAB_BAR_HEIGHT=36px, STATUS_BAR_HEIGHT=28px, SIDEBAR_DEFAULT_WIDTH=480px (from sidebar.rs).

```rust
// Source: ora/src/views/sidebar.rs (constants), tab_bar.rs (TAB_BAR_HEIGHT)
// AppLayout computes bounds and delegates to child views

pub struct AppLayout {
    sidebar: SidebarView,
    tab_bar: TabBarView,
    text_area: TextAreaView,
    panel_manager: PanelManagerView,
    status_bar: StatusBarView,
    command_palette: Option<CommandPaletteView>,
    // layout state
    sidebar_width: f32,
    panel_height: f32,
    sidebar_visible: bool,
    panel_visible: bool,
}

impl View for AppLayout {
    fn render(&self, cx: &mut ViewContext) -> AnyElement {
        // Main structure: Row(Sidebar, Column(TabBar, Row(Editor, Panel?), StatusBar))
        // CONTEXT: instant toggle means conditional include/exclude in element tree
        let theme = cx.theme();

        let mut main_col = Div::new()
            .flex_col()
            .grow(1.0);

        main_col = main_col.child(self.tab_bar.render(cx)); // 36px
        main_col = main_col.child(
            Div::new()
                .flex_row()
                .grow(1.0)
                .child(self.text_area.render(cx))
        );

        if self.panel_visible {
            main_col = main_col.child(self.panel_manager.render(cx));
        }
        main_col = main_col.child(self.status_bar.render(cx)); // 28px

        let mut root = Div::new()
            .flex_row()
            .w(pct(100.0))
            .h(pct(100.0))
            .bg(theme.color(ColorToken::BgPrimary));

        if self.sidebar_visible {
            root = root.child(self.sidebar.render(cx));
        }
        root = root.child(main_col);

        // CommandPalette is Stack overlay if open
        if let Some(palette) = &self.command_palette {
            return stack()
                .w(pct(100.0))
                .h(pct(100.0))
                .child(root)
                .child(palette.render(cx))
                .into();
        }

        root.into()
    }
}
```

### Anti-Patterns to Avoid

- **Storing fuzzy search results in the View struct during render:** Compute in render(), don't cache in struct — violates stateless view model. Store query and visible commands in Model<CommandPaletteState>.
- **Creating a new FocusHandle every render for tree items:** FileTree items with focus will create unbounded focus handle IDs. Pre-allocate or use a pool; only create handles for visible items.
- **Using absolute pixel positioning for tree indentation:** Use flex_row with spacer Divs for indent guides, not `left: px(depth * 16)` absolute positioning — the layout engine handles it more cleanly.
- **Polling in cx.spawn() for timers:** Use `dismiss_at: Option<Instant>` in model state, check in event loop's `about_to_wait` instead of busy-wait in spawned async tasks.
- **Triggering layout from within render():** Never call `cx.notify()` inside `render()`. State mutations (from drag resize, selection changes) must happen in event handlers, then notify triggers re-render.
- **Wrapping each tree row in its own TextElement with a new FocusHandle:** Creates O(n) focus handles per frame. Reuse or avoid per-row focus; use list-level focus with arrow-key navigation instead.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Z-layer overlays (CommandPalette, ContextMenu, Toast) | Custom absolute positioning | `stack()` element | Stack already handles z-layering via paint order, absolute positioned children (Phase 5) |
| Focus management for tab navigation in palette/tree | Custom tab index tracking | `cx.register_focusable()` + FocusHandle | Phase 4 event system already implements tab order via PrepaintContext::focusable_elements |
| Mouse drag tracking for resize | Manual cursor tracking | `cx.capture_mouse()` + InteractionState | Phase 4 already has MouseCapture, routes events outside bounds while captured |
| Theme-aware colors | Hardcoded hex values | `theme.color(ColorToken::*)` | Phase 6 token system, supports dark/light switching |
| Styled rectangle rendering | Custom wgpu draw calls | `cx.paint_styled_rect()` / `Div` | Framework batches all rects in one GPU draw call (Phase 2) |
| Text rendering | Calling glyphon directly | `TextElement` | Framework owns FontSystem/TextAtlas, elements don't (Phase 2) |
| Multi-select range calculation | Custom index tracking | `BTreeSet<usize>` for selection set | Standard library, no custom data structure needed |
| Text truncation with ellipsis | Custom overflow detection | Div with `overflow_hidden()` + fixed width | Framework handles clipping; ellipsis requires post-Phase-8 work if needed |

**Key insight:** Phase 8 is composition work, not infrastructure work. All building blocks exist. The risk is over-engineering new abstractions instead of composing existing ones.

## Common Pitfalls

### Pitfall 1: FocusHandle Proliferation in FileTree
**What goes wrong:** Each tree item rendered as a focusable element creates a new FocusHandle every render frame (if created in render()). With 1000 files, this generates 1000 new Arc-counted handles per frame.
**Why it happens:** Following Button's FocusHandle pattern directly, where button creates one handle during construction. FileTree has N dynamic rows.
**How to avoid:** Use a single FocusHandle for the tree container. Implement arrow-key navigation by tracking `selected_index` in Model<FileTreeState> and dispatching arrow key actions at the container level. Only individual items that need tab-stop focus (not arrow-key navigation) need their own FocusHandle.
**Warning signs:** Memory usage growing with file count, or many Arc<FocusInner> allocations per second.

### Pitfall 2: CommandPalette Layout Shifts on Input
**What goes wrong:** CommandPalette input is at top; result list is below it. As results filter, list height changes, causing the input to shift up/down if palette size is not fixed.
**Why it happens:** Using flex_col with grow(1.0) on result list means it expands to fill available space, changing total palette height based on result count.
**How to avoid:** Fixed palette height (input + 8-10 result rows height = fixed px value). Scroll within fixed-height result list using overflow_hidden. Do not use grow() on the result list — use h(px(RESULT_LIST_HEIGHT)) where RESULT_LIST_HEIGHT = 8 * ROW_HEIGHT.
**Warning signs:** Palette jumps position as user types, especially noticeable when going from 8 to fewer results.

### Pitfall 3: Drag Handle Hit Area Too Small
**What goes wrong:** Panel resize handle is 1px tall (matching the visual border), making it nearly impossible to click reliably.
**Why it happens:** Making the visual width equal to the hit area width.
**How to avoid:** Visual border = 1px, but register_hitbox for a 4-8px tall transparent Div covering the border area. The larger invisible hit area makes drag reliable. This is the standard "hot spot" pattern for resize handles.
**Warning signs:** Users report difficulty grabbing the resize handle, especially on high-DPI screens.

### Pitfall 4: Borrow Checker Error Getting Theme After Mutable cx Operations
**What goes wrong:** Getting `let theme = cx.theme()` reference, then calling `cx.register_hitbox()` or other `&mut cx` methods. Rust rejects this as aliased borrows.
**Why it happens:** Forgetting the "borrow-safe render pattern" from Phase 7 (STATE.md 07-01 decision).
**How to avoid:** Complete ALL mutable cx operations (register_hitbox, register_focusable, on_mouse_down) BEFORE getting the theme reference. OR copy needed theme colors into local variables before the mutable operations.
**Warning signs:** `cannot borrow 'cx' as mutable because it is also borrowed as immutable` compiler errors.

### Pitfall 5: CommandPalette Opens Without Capturing Keyboard Focus
**What goes wrong:** Ctrl+Shift+P opens CommandPalette overlay, but keyboard input still routes to the text area behind it. User types into code instead of filtering palette.
**Why it happens:** Opening the palette doesn't set focus to the palette's input field.
**How to avoid:** When CommandPalette becomes visible, immediately call `cx.focus(&palette_input_focus_handle)`. When palette closes, restore focus to previously focused element. Track `previous_focus: Option<FocusId>` before opening.
**Warning signs:** Typing after opening palette modifies text area instead of filtering results.

### Pitfall 6: Toast Position Conflicts with Other Overlays
**What goes wrong:** Toast notifications overlap CommandPalette or ContextMenu, creating layering confusion.
**Why it happens:** Toast positioned in top-right, CommandPalette also near top. Both rendered in Stack.
**How to avoid:** Position toasts in **bottom-right** of viewport (Claude's discretion per CONTEXT.md). This separates them from CommandPalette (top-center) and ContextMenu (cursor-relative). In AppLayout's Stack, toasts are the topmost layer.
**Warning signs:** Toast partially covers CommandPalette or ContextMenu, blocking interactions.

### Pitfall 7: Context Menu Appearing Outside Viewport
**What goes wrong:** Right-click near bottom-right corner of window causes ContextMenu to render off-screen (partially or completely outside window bounds).
**Why it happens:** Positioning at cursor coordinates without checking viewport boundaries.
**How to avoid:** After computing cursor position, clamp: if menu_right > window_width, position menu left of cursor. If menu_bottom > window_height, position menu above cursor. Use `cx.window_size()` from LayoutContext or PaintContext.
**Warning signs:** Users report ContextMenu appearing with clipped/missing items near window edges.

## Code Examples

### CommandPalette View Structure

```rust
// Source: CONTEXT.md decisions + established View pattern (ora/src/views/dialog.rs)
// CommandPalette renders as a Stack overlay centered near top of screen

pub struct CommandPaletteState {
    pub visible: bool,
    pub query: String,
    pub commands: Vec<CommandItem>,
    pub filtered: Vec<(usize, FuzzyMatch)>, // (command_index, match_info)
    pub selected_index: usize,
}

pub struct CommandPaletteView {
    state: Model<CommandPaletteState>,
    input_focus: FocusHandle,  // created once in ::new(), not per render
}

impl View for CommandPaletteView {
    fn render(&self, cx: &mut ViewContext) -> AnyElement {
        let state = cx.read_model(&self.state);
        if !state.visible {
            return Div::new().w(px(0.0)).h(px(0.0)).into();
        }

        let theme = cx.theme();
        let palette_width = 600.0f32;
        let row_height = 32.0f32;
        let visible_rows = 8usize; // CONTEXT: fixed 8-10 visible
        let list_height = visible_rows as f32 * row_height;

        // Palette container: centered horizontally, near top (VS Code style)
        Div::new()
            .flex_col()
            .w(px(palette_width))
            .bg(theme.color(ColorToken::BgElevated))
            .border(1.0, theme.color(ColorToken::Border))
            .border_radius(6.0)
            .shadow(0.0, 8.0, 32.0, 0.0, Color::rgba(0.0, 0.0, 0.0, 0.5))
            // Input row at top
            .child(self.render_input(&state, cx))
            // Fixed-height result list
            .child(
                Div::new()
                    .flex_col()
                    .h(px(list_height))
                    .overflow_hidden()
                    .children(
                        state.filtered.iter().take(visible_rows)
                            .map(|(cmd_idx, fm)| self.render_result(&state.commands[*cmd_idx], fm, *cmd_idx == state.selected_index, cx))
                            .collect::<Vec<_>>()
                    )
            )
            .into()
    }
}
```

### Widget Size Tier Implementation

```rust
// Source: CONTEXT.md (3 size tiers), Button pattern (ora/src/elements/button.rs)

pub struct WidgetSizeConfig {
    pub height: f32,
    pub font_size: f32,
    pub padding_h: f32,
    pub padding_v: f32,
    pub border_radius: f32,
}

impl WidgetSize {
    pub fn config(&self) -> WidgetSizeConfig {
        match self {
            WidgetSize::Sm => WidgetSizeConfig { height: 24.0, font_size: 12.0, padding_h: 8.0,  padding_v: 4.0, border_radius: 3.0 },
            WidgetSize::Md => WidgetSizeConfig { height: 32.0, font_size: 13.0, padding_h: 12.0, padding_v: 6.0, border_radius: 4.0 },
            WidgetSize::Lg => WidgetSizeConfig { height: 40.0, font_size: 14.0, padding_h: 16.0, padding_v: 8.0, border_radius: 5.0 },
        }
    }
}
```

### Checkbox/Toggle Boolean Control Pattern

```rust
// Source: CONTEXT.md (checked/unchecked states), established interaction pattern

pub struct Checkbox {
    checked: bool,
    label: String,
    disabled: bool,
    size: WidgetSize,
    on_change: Option<Box<dyn Fn(bool) + 'static>>,
    focus_handle: Option<FocusHandle>,
    // Internal text element for label
    label_element: Option<TextElement>,
}

// Toggle is the same pattern with pill-shaped background instead of box
pub struct Toggle {
    checked: bool,
    disabled: bool,
    size: WidgetSize,
    on_change: Option<Box<dyn Fn(bool) + 'static>>,
    focus_handle: Option<FocusHandle>,
}

// Paint pattern for checkbox:
// - unchecked: bordered square (border 1px, Border color)
// - checked: filled square (Accent bg, white checkmark "✓")
// - disabled: 50% opacity on entire element
// - hover: BgElevated background around the checkbox+label container
```

### Panel Manager with Drag Resize

```rust
// Source: ora/src/events/interaction.rs (MouseCapture), CONTEXT.md decisions

pub struct PanelManagerView {
    state: Model<PanelState>,
    drag_focus: FocusHandle,
}

// In prepaint, register mouse handlers on the resize handle hitbox:
// cx.on_mouse_down(handle_hitbox, move |evt, event_cx| {
//     // Start drag: record start position and current height
// });
// cx.on_mouse_move(handle_hitbox, move |evt, event_cx| {
//     // While dragging: update panel height = start_height - (evt.y - drag_start_y)
// });
// cx.on_mouse_up(handle_hitbox, move |evt, event_cx| {
//     // End drag: persist panel height
//     event_cx.release_mouse_capture();
// });
// Double-click: cycle through COLLAPSED(0) / HALF(200) / FULL(400)

// Panel tabs (Output, Problems, etc.) render as Tab widgets:
// - drag to reorder: tracked via separate drag state in Model<PanelState>
// - tab order persists alongside panel height preference
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Sidebar content was placeholder text | Phase 8 replaces with FileTree View | Phase 8 | Real file navigation |
| No panel system | Phase 8 adds PanelManager | Phase 8 | Output/Problems tabs |
| Button only (no input, checkbox, etc.) | Full widget suite | Phase 8 | Complete UI toolkit |
| No command palette | Phase 8 CommandPalette | Phase 8 | VS Code-style UX |
| Hardcoded layout (no AppLayout) | AppLayout orchestrates all regions | Phase 8 | Unified layout control |

**Known Issues from Phase 7 to Address:**
- `SidebarView::render_content()` returns placeholder text — Phase 8 replaces with FileTree
- `SIDEBAR_DEFAULT_WIDTH` in `sidebar.rs` is currently 480.0px (was increased in 07-05) but CONTEXT says 260px was the decision (discrepancy in STATE.md shows 260px decision then 480px actual) — Phase 8 should consolidate to a single exported constant used by both SidebarView and AppLayout
- Scissor clipping infrastructure exists but not GPU-rendered (noted in STATE.md Known Issues) — overflow:hidden on FileTree rows won't clip GPU-rendered text; this is acceptable for Phase 8 (workaround: fixed-size rows never overflow horizontally for normal file names)

**Deprecated approaches avoided in Phase 8:**
- Direct glyphon Buffer creation (use TextElement)
- Per-component FontSystem (framework-owned via TextSystem)
- Hardcoded colors (use theme.color(ColorToken::*))
- Manual GPU draw calls (use cx.paint_styled_rect, TextElement)

## Open Questions

1. **Where to organize widget primitives**
   - What we know: Button lives in `elements/button.rs`, views live in `views/`
   - What's unclear: Should Input, Checkbox, etc. go in `elements/` (with Button) or a new `widgets/` submodule?
   - Recommendation: Put in `elements/` for consistency with Button. Create `elements/input.rs`, `elements/checkbox.rs`, etc. This keeps the element/view distinction clean: elements are low-level primitives, views are composed components.

2. **CommandPalette Positioning Mechanism**
   - What we know: CONTEXT says "centered horizontally near top of viewport (VS Code-style)". AppLayout renders it as a Stack overlay.
   - What's unclear: Exact pixel offset from top (VS Code uses ~20% from top). Is this a hardcoded constant or calculated?
   - Recommendation: Hardcode `TOP_OFFSET = 80.0px` for the centering container's padding-top. Use `Div::new().flex_col().align_center().pt(TOP_OFFSET)` as the positioning wrapper inside the Stack.

3. **Panel Tab Drag-to-Reorder Implementation Depth**
   - What we know: CONTEXT says drag to reorder panel tabs. Drag requires mouse capture + visual feedback.
   - What's unclear: Is ghost drag preview needed (like a semi-transparent floating tab)? Or is just reordering on drop sufficient?
   - Recommendation: Simple reorder on drop (no ghost). Track `drag_tab_index: Option<usize>` in PanelState. On mouse_up over a different tab, swap order. Visual feedback: dimmed text on dragged tab via hover state query.

4. **Persistence Storage for Panel/Sidebar Sizes**
   - What we know: CONTEXT says "panel sizes persist across sessions". No persistence API exists in ora yet.
   - What's unclear: Where to write the file? What format? Which crate?
   - Recommendation: Write to a hardcoded path (`~/.config/muda/layout.toml` or `./muda-layout.json` in working directory). Use a simple custom key=value text format to avoid new dependencies. Parse with 5-10 lines of Rust. This is the minimal viable approach for Phase 8; a proper settings API can be a future phase.

5. **FileTree Data Source**
   - What we know: `SidebarPresentation` has `entries: Vec<FileEntryPresentation>` (flat list). FileTree needs hierarchy with expand/collapse.
   - What's unclear: Should Phase 8 define new `FileTreePresentation` types in `core_editor::view_model`, or build hierarchy from existing flat data?
   - Recommendation: Define `FileTreePresentation` with `Vec<FileTreeNode>` (nested) in `core_editor/src/view_model/mod.rs`. This is the correct separation: `core_editor` owns data shape, `ora` owns rendering. Update `SidebarPresentation` or create alongside it.

## Sources

### Primary (HIGH confidence)
- `ora/src/elements/button.rs` — ButtonVariant state-based styling, three-phase lifecycle pattern, FocusHandle integration
- `ora/src/elements/div.rs` — Complete Div builder API, hover_bg/active_bg/focus_ring, overflow_hidden
- `ora/src/elements/stack.rs` — Stack z-layering with AbsoluteWrapper, paint order for z-index
- `ora/src/views/dialog.rs` — Modal overlay pattern, stack() for z-layering, button row right-aligned
- `ora/src/views/sidebar.rs` — SidebarPresentation consumption, collapsed state, SIDEBAR constants
- `ora/src/views/tab_bar.rs` — Tab rendering pattern, TAB_BAR_HEIGHT=36px, dirty indicators
- `ora/src/context.rs` — AppContext::capture_mouse(), spawn(), focus management APIs
- `ora/src/events/interaction.rs` — MouseCapture, InteractionState, is_hovered/is_active
- `ora/src/element/trait_def.rs` — Element trait, LayoutContext, PrepaintContext, PaintContext APIs
- `ora/src/theme/mod.rs` — ColorToken enum, Theme::color(), all token definitions
- `core_editor/src/view_model/mod.rs` — SidebarPresentation, FileEntryPresentation existing types
- `.planning/STATE.md` — All locked architectural decisions from Phases 1-7
- `.planning/phases/08-advanced-ui-widgets/08-CONTEXT.md` — User decisions for Phase 8

### Secondary (MEDIUM confidence)
- Phase 7 RESEARCH.md — Prior architecture analysis, verified patterns for views consuming presentation data
- ROADMAP.md — Success criteria for all Phase 8 requirements

### Tertiary (LOW confidence)
- Standard fuzzy search algorithm — Character-sequence matching is a well-known pattern; specific score formula is author's synthesis, not from external source. Needs validation against user expectations.
- File-type icon color mapping — Seti/Material icon colors are a community standard but not formally specified. The color choices are approximations; exact colors should match what users expect from VS Code.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — Verified: all required infrastructure exists in ora Phases 1-7, no new dependencies needed
- Architecture: HIGH — Verified: all patterns directly derived from existing codebase code (button.rs, dialog.rs, sidebar.rs, context.rs)
- Pitfalls: HIGH — Derived from STATE.md documented decisions, known issues, and Rust borrow checker constraints verified in existing code

**Research date:** 2026-03-02
**Valid until:** 2026-04-02 (30 days — stable desktop GUI patterns, ora framework API stable)
