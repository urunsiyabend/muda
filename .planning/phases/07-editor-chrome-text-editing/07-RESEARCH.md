# Phase 7: Editor Chrome & Text Editing - Research

**Researched:** 2026-01-30
**Domain:** GPUI-style editor UI components with GPU-accelerated text rendering
**Confidence:** HIGH

## Summary

Phase 7 migrates eight critical editor components to ora Views: TabBar, StatusBar, Sidebar, Gutter, Dialog (chrome), and TextArea, Caret, Selection (text editing core). Research reveals these components follow a common pattern in modern GPU-based editors: Views own no rendering state (no FontSystem/TextAtlas), instead producing element trees that ora's framework manages. The existing wgpu_client components are tightly coupled to glyphon rendering (each owns FontSystem, SwashCache, TextAtlas, TextRenderer), which Phase 7 will eliminate.

Key architectural shift: Current components call glyphon prepare/render directly; ora Views build declarative element trees (Div, TextElement) and let the framework handle text shaping, layout, and batched rendering. This matches GPUI's proven model where views are pure functions of state, producing elements without side effects.

Critical decisions already locked in CONTEXT.md: no line wrapping (horizontal scroll), solid selection backgrounds, thin beam-style caret, full-width current line highlight, semi-transparent modal backdrop with no escape-key/backdrop-click dismiss. These align with modern code editor UX patterns and simplify implementation.

**Primary recommendation:** Build Views as thin wrappers over ora's existing element library (Div, TextElement, Button), consuming core_editor's RenderModel/Presentation types. Focus migration effort on state management (Model<T> for reactive updates) and event handling (FocusHandle, mouse/keyboard actions), not rendering primitives—those already exist from Phases 1-6.

## Standard Stack

### Core (Already Implemented in ora)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| wgpu | 23.0 | GPU abstraction | Industry standard, powers Zed/Bevy/egui |
| glyphon | 0.7 | Text rendering | Only production-ready GPU text renderer for wgpu 23 |
| winit | 0.30 | Windowing + events | Cross-platform standard, used by all GPU Rust projects |

### Supporting (Already Integrated)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| generational-arena | 0.2 | Entity storage | Already used for Model<T> storage in ora::AppContext |
| pollster | 0.3 | GPU async init | Already used for blocking wgpu::Instance creation |

### Not Needed (Phase 7 does not add dependencies)
Phase 7 uses only existing ora APIs from Phases 1-6. No new external dependencies required.

**Installation:**
No additional dependencies. All required infrastructure exists in `ora/Cargo.toml`.

## Architecture Patterns

### Recommended Project Structure
```
ora/src/
├── elements/           # Primitive elements (Phases 1-6)
│   ├── div.rs         # Layout container - use for all component backgrounds
│   ├── text.rs        # Text rendering - use for all text display
│   ├── button.rs      # Interactive button - use for tab close, dialog actions
│   ├── image.rs       # Icon rendering - use for file type icons
│   └── stack.rs       # Z-layering - use for dialog overlay
└── views/             # Phase 7 components live here
    ├── tab_bar.rs     # → TabBarView (CHROME-01)
    ├── status_bar.rs  # → StatusBarView (CHROME-02)
    ├── sidebar.rs     # → SidebarView (CHROME-03)
    ├── gutter.rs      # → GutterView (CHROME-04)
    ├── dialog.rs      # → DialogView (CHROME-05)
    ├── text_area.rs   # → TextAreaView (EDIT-01)
    ├── caret.rs       # → CaretElement (EDIT-02, not a View - just an element)
    └── selection.rs   # → SelectionElement (EDIT-03, element or integrated into TextArea)
```

### Pattern 1: Stateless View Consuming Presentation Data

**What:** View trait implementation that takes presentation data (from core_editor) and produces element tree
**When to use:** All chrome components (TabBar, StatusBar, Sidebar, Gutter, Dialog)
**Rationale:** Decouples rendering from domain logic, follows GPUI's View-as-function-of-state model

**Example TabBar:**
```rust
// Source: Existing wgpu_client/src/components/tab_bar.rs + ora element_library_demo.rs
use ora::{View, ViewContext, AnyElement, Div, TextElement, px, ColorToken};
use core_editor::view_model::TabBarPresentation;

pub struct TabBarView {
    // View owns no rendering state (no FontSystem, no TextAtlas)
    // All state comes from AppContext or passed-in presentation data
}

impl View for TabBarView {
    fn render(&self, cx: &mut ViewContext) -> AnyElement {
        // Get presentation data from Model or passed context
        let tab_bar: &TabBarPresentation = ...; // From Model<EditorState> or similar
        let theme = cx.theme();

        Div::new()
            .flex_row()
            .h(px(28.0)) // Or theme.height(ComponentSize::TabBar)
            .bg(theme.color(ColorToken::BgSecondary))
            .children(
                tab_bar.tabs.iter().map(|tab| {
                    self.render_tab(tab, cx)
                })
            )
            .into()
    }
}

impl TabBarView {
    fn render_tab(&self, tab: &TabPresentation, cx: &mut ViewContext) -> Div {
        let theme = cx.theme();
        let bg_color = if tab.is_active {
            theme.color(ColorToken::BgPrimary)
        } else {
            theme.color(ColorToken::BgSecondary)
        };

        Div::new()
            .flex_row()
            .gap(8.0)
            .p(12.0)
            .bg(bg_color)
            .child(TextElement::new(&tab.title).size(12.0))
            // Add close button, dirty indicator, etc.
    }
}
```

### Pattern 2: Blinking Caret with Activity Timeout

**What:** Caret blinks at configurable rate (default 500ms) but stays solid for 500ms after user activity
**When to use:** EDIT-02 Caret element
**Rationale:** Prevents distracting blink during typing, standard pattern in VSCode/Zed/Sublime

**Example:**
```rust
// Source: Existing wgpu_client/src/components/caret.rs + accessibility research
use std::time::{Duration, Instant};
use ora::{Element, Div, px, ColorToken};

pub struct CaretElement {
    blink_visible: bool,
    last_blink: Instant,
    last_activity: Instant,
    blink_rate: Duration,  // 500ms default (WCAG: must be < 3 blinks/sec = 333ms max)
}

impl CaretElement {
    const BLINK_RATE: Duration = Duration::from_millis(500); // Safe: 2 blinks/sec < 3/sec WCAG limit
    const ACTIVITY_TIMEOUT: Duration = Duration::from_millis(500);

    pub fn on_activity(&mut self) {
        self.last_activity = Instant::now();
        self.blink_visible = true; // Solid caret during typing
    }

    pub fn update(&mut self) -> bool {
        let now = Instant::now();

        // Don't blink if user was recently active
        if now.duration_since(self.last_activity) < Self::ACTIVITY_TIMEOUT {
            return false;
        }

        // Toggle blink state
        if now.duration_since(self.last_blink) >= self.blink_rate {
            self.blink_visible = !self.blink_visible;
            self.last_blink = now;
            return true; // Request redraw
        }
        false
    }
}

impl Element for CaretElement {
    fn paint(&mut self, cx: &mut PaintContext) {
        if !self.blink_visible { return; }

        // Render thin vertical line at caret position
        let caret_color = cx.theme().color(ColorToken::AccentPrimary);
        Div::new()
            .w(px(2.0))  // Thin beam style (CONTEXT decision)
            .h(px(line_height))
            .bg(caret_color)
            .paint(cx);
    }
}
```

### Pattern 3: Gutter Width Calculation for Monospace Fonts

**What:** Calculate gutter width as `padding + (digit_count × char_width)` where digit_count = max_line_number.to_string().len()
**When to use:** CHROME-04 Gutter view
**Rationale:** Prevents gutter resize on scroll, uses digit '0' width (not 'M') for accurate monospace width

**Example:**
```rust
// Source: Existing wgpu_client/src/components/gutter.rs + Zed issue #21860
use ora::{View, Div, TextElement, px, sp, TextSize};

impl GutterView {
    const LEFT_PADDING: f32 = 8.0;  // Theme token: sp(SpacingToken::Sm)

    fn calculate_width(&self, model: &RenderModel, cx: &ViewContext) -> f32 {
        let digit_count = model.gutter.line_number_width(); // Max line number digits
        let char_width = self.measure_digit_width(cx);

        Self::LEFT_PADDING + (digit_count as f32 * char_width) + 8.0 // Right padding for separator
    }

    fn measure_digit_width(&self, cx: &ViewContext) -> f32 {
        // Measure '0' width, not 'M' (Zed uses em for gutter, but digits are narrower)
        // ora's TextElement already handles font measurement during layout
        // For monospace fonts, all digits have same width
        let text_size = cx.theme().text_size(TextSize::Body);
        text_size.font_size * 0.6 // Monospace digit width approximation
    }
}
```

### Pattern 4: Modal Dialog with Backdrop and Button Focus

**What:** Dialog rendered as Stack with semi-transparent backdrop + centered dialog box + button row
**When to use:** CHROME-05 Dialog view
**Rationale:** Standard pattern from Material UI, Headless UI, Tailwind—backdrop dims background, buttons right-aligned

**Example:**
```rust
// Source: Existing wgpu_client/src/components/dialog.rs + Material UI/Tailwind patterns
use ora::{View, stack, Div, button, ButtonVariant, px, pct, ColorToken};

impl DialogView {
    fn render(&self, cx: &mut ViewContext) -> AnyElement {
        let theme = cx.theme();

        stack()
            // Layer 0: Semi-transparent backdrop (CONTEXT: no dismiss on click)
            .child(
                Div::new()
                    .absolute()
                    .w(pct(100.0))
                    .h(pct(100.0))
                    .bg(Color::from_rgba(0.0, 0.0, 0.0, 0.6)) // 60% opacity
                    // No on_click handler (CONTEXT decision: must use buttons)
            )
            // Layer 1: Centered dialog box
            .child(
                Div::new()
                    .absolute()
                    .w(px(420.0))
                    .flex_col()
                    .bg(theme.color(ColorToken::BgPrimary))
                    .border_radius(8.0)
                    .shadow_lg()
                    .child(self.render_dialog_content(cx))
                    .child(self.render_button_row(cx))
            )
            .into()
    }

    fn render_button_row(&self, cx: &mut ViewContext) -> Div {
        Div::new()
            .flex_row()
            .justify_end()  // Right-aligned (CONTEXT decision)
            .gap(12.0)
            .p(24.0)
            .child(button("Save (Y)", ButtonVariant::Primary, self.save_focus.clone()))
            .child(button("Don't Save (N)", ButtonVariant::Secondary, self.no_save_focus.clone()))
            .child(button("Cancel (Esc)", ButtonVariant::Ghost, self.cancel_focus.clone()))
            // Note: Esc key bound in keymap, not handled by backdrop (CONTEXT decision)
    }
}
```

### Pattern 5: Text Selection Rendering

**What:** Selection backgrounds rendered as Div rectangles behind text, layered between current line highlight and text
**When to use:** EDIT-03 Selection rendering (integrated into TextAreaView)
**Rationale:** Matches existing wgpu_client pattern, leverages ora's z-ordering via child order

**Example:**
```rust
// Source: Existing wgpu_client/src/components/text_area.rs
impl TextAreaView {
    fn render(&self, cx: &mut ViewContext) -> AnyElement {
        let model: &RenderModel = ...; // From Model<EditorState>
        let theme = cx.theme();

        Div::new()
            .relative()
            .w(pct(100.0))
            .h(pct(100.0))
            // Layer 0: Current line highlight (full width)
            .child(self.render_current_line_bg(model, theme))
            // Layer 1: Selection backgrounds (solid, per CONTEXT decision)
            .children(self.render_selection_rects(model, theme))
            // Layer 2: Text content (syntax highlighted)
            .children(self.render_text_lines(model, theme))
            // Layer 3: Caret (on top)
            .child(CaretElement::new(...))
            .into()
    }

    fn render_selection_rects(&self, model: &RenderModel, theme: &Theme) -> Vec<Div> {
        model.visible_lines.iter()
            .flat_map(|line| {
                line.spans.iter().filter_map(|span| {
                    if span.style == TextStyle::Selection {
                        Some(Div::new()
                            .absolute()
                            .x(px(span.x_offset))
                            .y(px(line.y_offset))
                            .w(px(span.width))
                            .h(px(line_height))
                            .bg(theme.color(ColorToken::Selection))) // Solid bg, not semi-transparent
                    } else {
                        None
                    }
                })
            })
            .collect()
    }
}
```

### Anti-Patterns to Avoid

- **Creating FontSystem per View:** Views must NOT own FontSystem/TextAtlas—these are framework-managed in ora's TextSystem (Phase 2). Use TextElement instead.
- **Manual glyphon prepare/render:** Views produce element trees, not GPU commands. No direct glyphon API calls in view code.
- **Hardcoded sizes:** Use theme tokens (sp(), TextSize) not raw float literals. Existing wgpu_client has 24.0/28.0/12.0 scattered—Phase 7 must eliminate these.
- **Component-specific Font metrics:** Don't call Buffer::new() or set_metrics() in views. TextElement handles this internally.
- **Premature focus management:** Phase 7 views render focus states (rings, highlights) but don't implement keyboard nav—that's Phase 8 (FileTree, CommandPalette).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Text rendering | Custom glyphon BufferLine iteration | ora::TextElement | Framework batches all text into single glyphon prepare/render pass (Phase 2) |
| Blinking animation | RequestRedraw timer loop | CaretElement with time_until_next_blink() | Already implemented in existing caret.rs, proven pattern |
| Modal backdrop | Custom overlay element | ora::stack() with Div layers | Stack handles z-ordering, absolute positioning built-in (Phase 5) |
| Tab close buttons | Custom clickable region | ora::button(ButtonVariant::Ghost) | Button handles hover/active/focus states, theme-aware (Phase 5) |
| Gutter digit width | String::len() × constant | Measure '0' glyph via TextElement layout | Handles variable-width fonts correctly (Zed learned this the hard way) |
| Selection highlight clipping | Manual scissor rect calculation | Div with overflow:hidden | Framework handles clipping (ELEM-06 API exists, GPU deferred but not blocking) |
| Dialog button focus order | Manual tab index tracking | FocusHandle + PrepaintContext::register_focusable | Phase 4 event system already implements tab navigation |
| Theme color lookups | Direct palette indexing | theme.color(ColorToken::*) | Semantic tokens decouple from palette, supports dark/light themes (Phase 6) |

**Key insight:** Phase 7 is NOT about building rendering primitives—it's about composing existing primitives (Div, TextElement, Button, Stack) into editor-specific layouts. The wgpu_client components do too much (own FontSystem, call prepare/render); ora Views do less (just build element trees).

## Common Pitfalls

### Pitfall 1: Storing Rendering State in Views
**What goes wrong:** View stores FontSystem, Buffer, prepared: bool flag. On re-render, framework rebuilds element tree but view's cached state is stale.
**Why it happens:** Porting existing wgpu_client components directly without understanding ora's declarative model.
**How to avoid:** Views are stateless render functions. Store document state in Model<T>, not in the View struct. If state is needed, it goes in Model and View observes it.
**Warning signs:** View struct has fields like font_system: FontSystem, buffer: Buffer, text_atlas: TextAtlas. These belong in ora's framework-level TextSystem, not views.

### Pitfall 2: Forgetting Activity Timeout on Caret Blink
**What goes wrong:** Caret blinks during typing, making cursor jump visually between visible/invisible states mid-edit.
**Why it happens:** Implementing blink timer without checking last_activity, or not calling on_activity() on keyboard events.
**How to avoid:** Maintain last_activity: Instant, reset on every keyboard event. Don't blink if now - last_activity < 500ms.
**Warning signs:** Users report "cursor flickers while typing" or "hard to see where I'm typing".

### Pitfall 3: Hardcoding Component Heights
**What goes wrong:** TabBar height hardcoded as 28.0 in tab_bar.rs, but also defined as 35.0 in app.rs layout calculations. Components misalign.
**Why it happens:** Existing wgpu_client has duplication (STATE.md notes "Status bar height (24.0) hardcoded in 4 places").
**How to avoid:** Define component sizes as theme tokens or const in single location. Use sp(SpacingToken::*) for margins/padding, ComponentSize enum for component-specific heights.
**Warning signs:** Layout calculations repeated in multiple files, git diff shows same magic number in >1 file.

### Pitfall 4: Using 'M' Width for Gutter Calculation
**What goes wrong:** Gutter reserves 4 ems of space (width of 'M'), but digits are narrower than 'M' in proportional fonts. Gutter is too wide.
**Why it happens:** Confusing em (width of 'M') with digit width. Monospace fonts have uniform digit width < em.
**How to avoid:** Measure '0' glyph width specifically. For monospace, all digits have same width. Don't use em for gutter width.
**Warning signs:** Gutter wider than necessary, especially with proportional UI fonts (though editor uses monospace).

### Pitfall 5: Not Respecting CONTEXT.md Locked Decisions
**What goes wrong:** Implementing escape-key-dismisses-dialog or backdrop-click-dismisses, but CONTEXT.md explicitly says "must use buttons".
**Why it happens:** Following standard modal patterns without reading project-specific constraints.
**How to avoid:** CONTEXT.md Decisions section is LOCKED. Don't implement alternatives. If decision seems wrong, flag for user discussion before planning.
**Warning signs:** Implementing features not in CONTEXT.md, adding "smart defaults" that contradict locked decisions.

### Pitfall 6: Blocking on Action Handler Context Access
**What goes wrong:** Button on_click needs to call cx.set_theme() but handlers lack context. Work blocked.
**Why it happens:** STATE.md notes "Action and button handlers lack context access" as known issue from Phase 6.
**How to avoid:** This is a known blocker. For Phase 7, dialog buttons can emit events to parent view, which has context access. Full fix deferred to future work.
**Warning signs:** Trying to call cx.method() from button handler closure, getting borrow checker errors.

## Code Examples

Verified patterns from existing codebase:

### Example 1: View Observing Model with Reactivity
```rust
// Source: ora/examples/reactive_demo.rs + Phase 3 Model/Observer pattern
use ora::{View, ViewContext, Model, AnyElement, Div, TextElement, px};

pub struct TabBarView {
    editor_state: Model<EditorState>,  // Reactive state
}

impl View for TabBarView {
    fn render(&self, cx: &mut ViewContext) -> AnyElement {
        // Observer pattern: when editor_state changes, this view re-renders
        let state = self.editor_state.read(cx);
        let tab_bar = &state.tab_bar;  // core_editor::TabBarPresentation

        Div::new()
            .flex_row()
            .h(px(28.0))
            .children(tab_bar.tabs.iter().map(|tab| self.render_tab(tab, cx)))
            .into()
    }
}

// In parent view or app initialization:
impl App {
    fn create_tab_bar_view(&mut self, cx: &mut AppContext) -> Entity<TabBarView> {
        let editor_state = cx.new_model(|_| EditorState::new());

        // Subscribe to editor state changes
        let tab_bar_view = cx.new_view(|cx| {
            cx.observe(&editor_state, |_view, _model, cx| {
                cx.notify(); // Trigger re-render when model changes
            });

            TabBarView { editor_state }
        });

        tab_bar_view
    }
}
```

### Example 2: StatusBar with Multiple Text Regions
```rust
// Source: wgpu_client/src/components/status_bar.rs + ora element patterns
use ora::{View, Div, TextElement, px, pct, JustifyContent, ColorToken};

impl StatusBarView {
    fn render(&self, cx: &mut ViewContext) -> AnyElement {
        let status = self.get_status(cx); // core_editor::StatusPresentation
        let theme = cx.theme();

        Div::new()
            .flex_row()
            .w(pct(100.0))
            .h(px(24.0))  // Or theme.component_height(ComponentSize::StatusBar)
            .bg(theme.color(ColorToken::BgSecondary))
            .p(8.0)
            .justify(JustifyContent::SpaceBetween)
            .child(
                // Left side: cursor position
                TextElement::new(format!("Ln {}, Col {}", status.cursor_line, status.cursor_column))
                    .size(12.0)
                    .color(theme.color(ColorToken::FgSecondary))
            )
            .child(
                // Right side: language, encoding, git branch
                Div::new()
                    .flex_row()
                    .gap(16.0)
                    .child(TextElement::new(&status.language).size(12.0))
                    .child(TextElement::new(&status.encoding).size(12.0))
                    .child(TextElement::new(&status.git_branch).size(12.0))
            )
            .into()
    }
}
```

### Example 3: Sidebar with Collapsible State
```rust
// Source: wgpu_client/src/components/sidebar.rs + CONTEXT decisions
use ora::{View, Div, TextElement, button, ButtonVariant, px, pct, ColorToken};

pub struct SidebarView {
    is_collapsed: bool,  // Or in Model<SidebarState>
    width: f32,  // Resizable via draggable edge
}

impl View for SidebarView {
    fn render(&self, cx: &mut ViewContext) -> AnyElement {
        let theme = cx.theme();

        if self.is_collapsed {
            // CONTEXT decision: narrow icon rail, not completely hidden
            return Div::new()
                .w(px(48.0))  // Icon rail width
                .h(pct(100.0))
                .bg(theme.color(ColorToken::BgSecondary))
                .flex_col()
                .child(button("📁", ButtonVariant::Ghost, self.toggle_focus.clone()))
                .into();
        }

        Div::new()
            .w(px(self.width))  // User-resizable
            .h(pct(100.0))
            .bg(theme.color(ColorToken::BgSecondary))
            .border_r(1.0, theme.color(ColorToken::BorderDefault))
            .child(self.render_header(cx))  // Toggle button here
            .child(self.render_content(cx))  // File tree, etc.
            .into()
    }

    fn render_header(&self, cx: &mut ViewContext) -> Div {
        Div::new()
            .flex_row()
            .h(px(36.0))
            .p(8.0)
            .justify(JustifyContent::SpaceBetween)
            .child(TextElement::new("Explorer").size(14.0))
            .child(
                // CONTEXT decision: dedicated toggle button (not header click)
                button("⏴", ButtonVariant::Ghost, self.toggle_focus.clone())
            )
    }
}
```

### Example 4: TextArea with Syntax Highlighting
```rust
// Source: wgpu_client/src/components/text_area.rs + core_editor StyledSpan mapping
use ora::{View, Div, TextElement, px, pct, Color};

impl TextAreaView {
    fn render(&self, cx: &mut ViewContext) -> AnyElement {
        let model: &RenderModel = self.get_render_model(cx);
        let theme = cx.theme();

        Div::new()
            .relative()
            .w(pct(100.0))
            .h(pct(100.0))
            .overflow_scroll()  // CONTEXT decision: no wrap, horizontal scroll
            .children(
                model.visible_lines.iter().enumerate().map(|(idx, line)| {
                    self.render_line(line, idx, theme)
                })
            )
            .into()
    }

    fn render_line(&self, line: &LinePresentation, row: usize, theme: &Theme) -> Div {
        Div::new()
            .flex_row()
            .h(px(theme.line_height()))
            .child(
                Div::new()
                    .flex_row()
                    .children(
                        line.spans.iter().map(|span| {
                            let color = self.map_style_to_color(span.style, theme);
                            TextElement::new(&span.text)
                                .size(theme.font_size())
                                .color(color)
                        })
                    )
            )
    }

    fn map_style_to_color(&self, style: TextStyle, theme: &Theme) -> Color {
        use core_editor::view_model::TextStyle;
        match style {
            TextStyle::Keyword => theme.color(ColorToken::SyntaxKeyword),
            TextStyle::String => theme.color(ColorToken::SyntaxString),
            TextStyle::Comment => theme.color(ColorToken::SyntaxComment),
            TextStyle::Selection => theme.color(ColorToken::Selection),
            // ... map all TextStyle variants
            _ => theme.color(ColorToken::FgPrimary),
        }
    }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Component owns FontSystem | Framework owns TextSystem | Phase 2 (Jan 2026) | Views can't create glyphon resources; use TextElement API |
| glyphon prepare/render per component | Single-batch text rendering | Phase 2 (Jan 2026) | All text from all views rendered in one pass; no per-view rendering |
| Manual scissor rect calculation | Element-level clipping | Phase 2 API / Phase 9 GPU | Overflow:hidden on Div, framework manages clip stack |
| Hardcoded color/spacing values | Theme token system | Phase 6 (Jan 2026) | Components query theme, support dark/light mode |
| Direct Model mutation in views | Model<T> + Observer pattern | Phase 3 (Jan 2026) | Views observe models, can't mutate directly during render |

**Deprecated/outdated in wgpu_client:**
- `RectRenderer`: Replaced by ora's Div element with GPU-accelerated rounded rect/shadow shader (Phase 2)
- `Buffer::new()` in components: Replaced by TextElement, framework manages buffers (Phase 2)
- Theme direct palette access: Replaced by semantic ColorToken (Phase 6)
- Hardcoded ComponentSize: Replaced by sp() spacing scale + TextSize (Phase 6)

## Open Questions

Things that couldn't be fully resolved:

1. **Tab Dropdown Menu for Overflow**
   - What we know: CONTEXT.md specifies "dropdown menu at end for tabs that don't fit"
   - What's unclear: Dropdown component doesn't exist yet (Phase 8 UI-02 FileTree might provide tree/list component)
   - Recommendation: Phase 7 render all tabs without dropdown; add dropdown in Phase 8 when list/menu components exist

2. **Sidebar Resize Handle Hit Testing**
   - What we know: CONTEXT.md says "draggable edge to adjust width", Phase 4 has mouse event system
   - What's unclear: Is there a Resize event type, or do we track MouseDown + MouseMove on thin Div?
   - Recommendation: Use Div with hover/active states + mouse capture (Phase 4 EVT-05 MouseCapture), detect drag threshold

3. **Action Handler Context Access (Known Blocker)**
   - What we know: STATE.md notes "Action and button handlers lack context access" from Phase 6
   - What's unclear: Timeline for fix, workaround pattern
   - Recommendation: For Phase 7 dialogs, buttons emit events that parent view handles (parent has context access). Track as technical debt for future improvement.

4. **Caret Blink Rate Configurability**
   - What we know: CONTEXT.md leaves "caret blink rate" to Claude's discretion
   - What's unclear: Should this be a theme token, user setting, or hardcoded const?
   - Recommendation: Start with const DEFAULT_BLINK_RATE = 500ms (WCAG-safe), add to theme tokens if users request configurability

5. **Gutter Width Auto-Adjustment**
   - What we know: Gutter width = digits × char_width, recalculates when max_line_number changes
   - What's unclear: Does gutter resize trigger re-layout of entire editor? Performance implications?
   - Recommendation: Cache digit_count, only recalculate gutter width when it changes (not every frame). Layout system handles width change propagation.

## Sources

### Primary (HIGH confidence)
- ora codebase: `ora/src/` (Phases 1-6 implementation) - Verified element library, theme tokens, event system
- wgpu_client existing components: `wgpu_client/src/components/` - Baseline architecture to migrate from
- core_editor view_model: `core_editor/src/view_model/` - Presentation data structures (TabBarPresentation, StatusPresentation, RenderModel, StyledSpan, TextStyle)
- [GPUI Component Library](https://longbridge.github.io/gpui-component/) - 60+ components for GPUI, includes Editor, Tabs
- [Zed GPUI README](https://github.com/zed-industries/zed/blob/main/crates/gpui/README.md) - Official GPUI architecture: views as render functions, element trees
- [Material UI Modal](https://mui.com/material-ui/react-modal/) - Industry-standard modal/backdrop patterns
- [Modal UX Design 2026](https://userpilot.com/blog/modal-ux-design/) - Best practices: semi-transparent backdrop, right-aligned buttons, escape patterns

### Secondary (MEDIUM confidence)
- [VS Code Custom Layout](https://code.visualstudio.com/docs/configure/custom-layout) - Sidebar/panel architecture: Primary Side Bar, Secondary Sidebar, Panel regions
- [CSS caret-animation MDN](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/caret-animation) - Caret blink behavior, prefers-reduced-motion
- [WCAG Animation Best Practices](https://medium.com/design-bootcamp/animation-flashing-content-digital-accessibility-best-practices-a33b5470cc42) - < 3 flashes/sec rule, motion sensitivity
- [Zed Gutter Width Issue #21860](https://github.com/zed-industries/zed/issues/21860) - Use '0' digit width, not 'M' em width
- [AlterNET Code Editor Rendering](https://www.alternetsoft.com/blog/code-editor-text-rendering) - GPU text rendering performance: GDI, DirectX, metrics caching

### Tertiary (LOW confidence)
- [Tailwind Modal Dialogs](https://tailwindcss.com/plus/ui-blocks/application-ui/overlays/modal-dialogs) - Visual examples of modal styling (commercial UI kit)
- [GPU Text Rendering Techniques - Monotype](https://www.monotype.com/resources/expertise/gpu-text-rendering-techniques) - Glyph texture mapping concepts

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - All dependencies already integrated in ora (Phases 1-6), no new libraries needed
- Architecture: HIGH - Patterns verified in existing ora examples (element_library_demo.rs, reactive_demo.rs) and wgpu_client components
- Pitfalls: HIGH - Derived from STATE.md known issues, existing component duplication, and CONTEXT.md locked decisions

**Research date:** 2026-01-30
**Valid until:** 2026-03-02 (30 days for stable desktop GUI patterns; GPUI/Zed iteration cycle is fast but architecture is stable)
