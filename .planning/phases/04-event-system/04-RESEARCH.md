# Phase 4: Event System - Research

**Researched:** 2026-01-29
**Domain:** Event routing, hit testing, focus management, two-phase dispatch
**Confidence:** MEDIUM

## Summary

Event systems in UI frameworks require four coordinated subsystems: hit testing (determining which element receives mouse events), two-phase dispatch (capture and bubble phases similar to DOM), focus management (tracking which element receives keyboard input), and action registry (mapping platform-specific keys to semantic actions). GPUI (the reference architecture for this project) implements a sophisticated system that separates mouse events (routed via spatial hit testing) from keyboard events (routed via focus hierarchy), with both using the same two-phase dispatch mechanism.

The standard approach uses hitbox registration during prepaint phase (after layout is resolved), spatial Z-order testing for mouse events (front-to-back), a focus handle system with reference counting for keyboard routing, and typed Action traits rather than string identifiers for keyboard shortcuts. Mouse capture is essential for drag operations (mousedown on element A, mouseup elsewhere should still deliver to A), and hover/active state tracking should be framework-managed rather than element-managed to avoid inconsistencies.

Critical architectural decisions: hitboxes must be registered AFTER layout (prepaint phase) when final bounds are known, Z-order iteration must be reverse (last-painted = topmost = first tested), focus must survive re-renders via stable IDs (not direct element references), and keyboard-triggered focus rings require tracking input modality (mouse vs keyboard).

**Primary recommendation:** Use GPUI's architecture pattern of hitbox registration during prepaint, Z-ordered hit testing in reverse, FocusHandle with reference counting for stable keyboard routing, and typed Action traits for keybinding targets.

## Standard Stack

This phase requires no external crates beyond existing dependencies. Implementation uses Rust standard library types and existing framework infrastructure.

### Core (Existing Dependencies)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| winit | (current) | Platform event source | De facto standard for cross-platform windowing in Rust |
| std::collections | stdlib | HashMap, HashSet for tracking | Standard Rust collections for event routing data structures |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| smallvec | (optional) | Stack-allocated vectors | Focus path storage to avoid heap allocations |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Typed Action trait | String identifiers | Strings lose type safety, no compile-time keybinding validation |
| FocusHandle with ID | Direct element refs | Direct refs don't survive re-renders in immediate mode UI |
| Framework hover tracking | Per-element tracking | Element tracking creates inconsistency when cursor moves between frames |

**Installation:**
No additional crates required. Uses existing winit integration and Rust stdlib.

## Architecture Patterns

### Recommended Project Structure
```
ora/src/
├── events/
│   ├── mod.rs           # Event types, Action trait
│   ├── mouse.rs         # Mouse event types, hit testing
│   ├── keyboard.rs      # Keyboard event types, key translation
│   ├── focus.rs         # FocusHandle, FocusId, focus management
│   ├── actions.rs       # Action registry, keybinding matching
│   └── dispatch.rs      # Two-phase dispatch implementation
└── element.rs           # Add hitbox registration to Element trait
```

### Pattern 1: Hitbox Registration During Prepaint
**What:** Register element bounds for hit testing after layout is resolved
**When to use:** Every interactive element (buttons, inputs, clickable divs)
**Example:**
```rust
// Source: GPUI architecture pattern (DeepWiki)
// In PrepaintContext
pub struct Hitbox {
    pub bounds: Rect,
    pub element_id: ElementId,
    pub opaque: bool, // false = pointer-events: none
}

impl PrepaintContext {
    pub fn register_hitbox(&mut self, bounds: Rect, element_id: ElementId, opaque: bool) {
        self.hitboxes.push(Hitbox { bounds, element_id, opaque });
    }
}

// In element implementation
fn prepaint(&mut self, state: &mut Self::RequestLayoutState, cx: &mut PrepaintContext) {
    let bounds = cx.bounds(state.layout_id);
    cx.register_hitbox(bounds, self.element_id, true);
    // prepaint children...
}
```

### Pattern 2: Z-Order Hit Testing (Reverse Iteration)
**What:** Test hitboxes from last-painted (topmost) to first-painted (bottom)
**When to use:** Every mouse event (click, move, scroll)
**Example:**
```rust
// Source: GPUI Frame::hit_test pattern (DeepWiki)
pub fn hit_test(&self, point: Point) -> Option<ElementId> {
    // Iterate in REVERSE order (last painted = topmost)
    for hitbox in self.hitboxes.iter().rev() {
        if !hitbox.opaque {
            continue; // Skip transparent hitboxes
        }
        if hitbox.bounds.contains(point) {
            return Some(hitbox.element_id);
        }
    }
    None
}
```

### Pattern 3: FocusHandle with Reference Counting
**What:** Stable focus identifiers that survive re-renders
**When to use:** Any focusable element (inputs, buttons, custom controls)
**Example:**
```rust
// Source: GPUI focus system (DeepWiki)
pub struct FocusHandle {
    id: FocusId,
    _ref: Arc<()>, // Reference counting for cleanup
}

impl FocusHandle {
    pub fn focus(&self, cx: &mut AppContext) {
        cx.set_focused(self.id);
    }

    pub fn is_focused(&self, cx: &AppContext) -> bool {
        cx.focused_id() == Some(self.id)
    }
}

// In view constructor
impl MyView {
    fn new(cx: &mut ViewContext) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            // ...
        }
    }
}
```

### Pattern 4: Two-Phase Dispatch
**What:** Events propagate in capture (root→target) then bubble (target→root)
**When to use:** All mouse and keyboard events
**Example:**
```rust
// Source: DOM event model (javascript.info), adapted to Rust
pub enum DispatchPhase {
    Capture,
    Bubble,
}

pub struct EventContext<'a> {
    phase: DispatchPhase,
    propagation_stopped: bool,
    // ...
}

impl EventContext<'_> {
    pub fn stop_propagation(&mut self) {
        self.propagation_stopped = true;
    }
}

// Dispatch algorithm
fn dispatch_event(target_path: &[ElementId], event: &MouseEvent) {
    // Capture: root to target
    for element_id in target_path.iter() {
        if let Some(handler) = handlers.get(element_id) {
            handler(event, DispatchPhase::Capture);
            if event.propagation_stopped() { return; }
        }
    }

    // Bubble: target to root
    for element_id in target_path.iter().rev() {
        if let Some(handler) = handlers.get(element_id) {
            handler(event, DispatchPhase::Bubble);
            if event.propagation_stopped() { return; }
        }
    }
}
```

### Pattern 5: Action Trait with Type Safety
**What:** Keyboard shortcuts target typed actions, not strings
**When to use:** All keybinding definitions
**Example:**
```rust
// Source: GPUI action system (0xshadow blog)
pub trait Action: 'static + Send + Sync {
    fn name(&self) -> &'static str;
}

// Define actions as structs
#[derive(Clone)]
pub struct Increment;
impl Action for Increment {
    fn name(&self) -> &'static str { "increment" }
}

// Register handler
view.on_action(|view: &mut Counter, _action: &Increment, cx| {
    view.count += 1;
    cx.notify();
});

// Register keybinding
keymap.bind("ctrl+up", Increment);
```

### Pattern 6: Input Modality Tracking for Focus Rings
**What:** Track whether focus came from keyboard or mouse
**When to use:** Focus ring styling (only show on keyboard navigation)
**Example:**
```rust
// Source: CSS :focus-visible pattern (CSS-Tricks, MDN)
pub enum FocusSource {
    Keyboard,
    Mouse,
    Programmatic,
}

pub struct FocusState {
    focused_id: Option<FocusId>,
    focus_source: FocusSource,
}

// On Tab key
fn handle_tab_key(cx: &mut AppContext) {
    cx.focus_next(FocusSource::Keyboard);
}

// On mouse click
fn handle_mouse_down(cx: &mut AppContext, element_id: ElementId) {
    cx.set_focused(element_id, FocusSource::Mouse);
}

// In element render
fn render(&self, cx: &ViewContext) -> impl Element {
    div()
        .when(cx.is_focused(self.focus_handle.id()) && cx.focus_source_is_keyboard(), |div| {
            div.border_color(blue()).border_width(px(2.0))
        })
}
```

### Pattern 7: Mouse Capture for Drag Operations
**What:** Element captures mouse for duration of drag (mousedown → mouseup)
**When to use:** Draggable elements, sliders, scrollbars
**Example:**
```rust
// Source: Windows SetCapture pattern (CodeProject)
pub struct MouseCapture {
    element_id: ElementId,
    button: MouseButton,
}

impl AppContext {
    pub fn capture_mouse(&mut self, element_id: ElementId, button: MouseButton) {
        self.mouse_capture = Some(MouseCapture { element_id, button });
    }

    pub fn release_mouse_capture(&mut self) {
        self.mouse_capture = None;
    }
}

// In event handler
fn on_mouse_down(&mut self, event: &MouseDownEvent, cx: &mut EventContext) {
    self.dragging = true;
    cx.capture_mouse(self.element_id, MouseButton::Left);
}

fn on_mouse_up(&mut self, event: &MouseUpEvent, cx: &mut EventContext) {
    self.dragging = false;
    cx.release_mouse_capture();
}
```

### Anti-Patterns to Avoid

- **Registering hitboxes during layout:** Bounds are not final until layout completes. Register during prepaint.
- **Forward Z-order iteration:** Last-painted elements are topmost; iterate hitboxes in reverse.
- **String-based actions:** Type-safe Action traits prevent typos and enable refactoring.
- **Per-element hover tracking:** Framework should track cursor position and elements query state to avoid inconsistencies.
- **Direct element references in focus:** References don't survive re-renders; use stable FocusId with reference counting.
- **Focus rings on all focus:** Only show focus rings when focus came from keyboard (accessibility requirement).

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Physical/logical key translation | Manual keycode mapping per platform | winit KeyEvent with physical_key and logical_key fields | winit handles platform differences (XKB, VK codes, Carbon) |
| Mouse coordinate DPI scaling | Manual DPI factor calculation | winit's logical coordinates (already scaled) | winit provides pre-scaled coordinates in WindowEvent |
| Focus path computation | Ad-hoc parent traversal during event | Build focus path once during prepaint from element tree | Compute once, reuse for all keyboard events |
| Key repeat filtering | Manual timing logic | winit KeyEvent.repeat field | Platform handles repeat timing correctly |
| Dead key handling | Custom composition state | winit KeyEvent.text field | Platform IME integration for composition |

**Key insight:** Platform abstractions (winit) already solve cross-platform input inconsistencies. Building custom translations creates bugs on less-tested platforms. Use winit's abstractions and focus framework logic on routing, not translation.

## Common Pitfalls

### Pitfall 1: Hitbox Registration Before Layout Completes
**What goes wrong:** Hitboxes registered during layout phase use incorrect/incomplete bounds, causing click targets to be misaligned with visual elements.
**Why it happens:** Temptation to register hitboxes immediately when element is created, before layout constraint solving.
**How to avoid:** ONLY register hitboxes during prepaint phase, after `layout_cx.compute()` has run. Prepaint receives final LayoutOutput bounds.
**Warning signs:** Clicking slightly above/below/beside visible buttons. Hitboxes out of sync with visual bounds.

### Pitfall 2: Forward Z-Order Hit Testing
**What goes wrong:** Background elements receive clicks intended for foreground elements. Modals don't block clicks.
**Why it happens:** Iterating hitboxes in insertion order (first-painted to last-painted), but last-painted elements are topmost.
**How to avoid:** Iterate hitboxes in reverse order: `hitboxes.iter().rev()`. Last painted = topmost = first tested.
**Warning signs:** Clicks "fall through" overlays. Can interact with elements behind modals.

### Pitfall 3: Hover State Per-Element Tracking
**What goes wrong:** Hover state becomes inconsistent when cursor moves quickly. Multiple elements think they're hovered simultaneously.
**Why it happens:** Each element independently tracks hover via MouseEnter/MouseLeave events. Events can be missed or misordered.
**How to avoid:** Framework tracks cursor position and hovered element. Elements QUERY state via `cx.is_hovered(element_id)`.
**Warning signs:** Multiple hover effects active. Hover effect "sticks" after cursor leaves. Hover effects flicker.

### Pitfall 4: Focus Survives Re-render via Direct References
**What goes wrong:** Focused element reference becomes stale after re-render. Keyboard input goes nowhere.
**Why it happens:** Immediate mode UI destroys and recreates elements every frame. Direct references don't survive.
**How to avoid:** Use stable FocusId (not element reference). FocusHandle with reference counting keeps ID alive across frames.
**Warning signs:** Keyboard input stops working after UI updates. Focus "lost" when unrelated UI changes.

### Pitfall 5: Focus Ring Always Visible
**What goes wrong:** Mouse clicks show ugly focus rings. Users see outlines everywhere.
**Why it happens:** Focus ring styled on all focus, not distinguishing between keyboard and mouse focus.
**How to avoid:** Track focus source (keyboard/mouse). Only show focus ring when `focus_source == Keyboard`.
**Warning signs:** Clicked buttons have persistent outlines. User complains about "blue borders everywhere".

### Pitfall 6: Mouse Capture Leaks on Alt-Tab
**What goes wrong:** User drags element, Alt-Tabs away, returns to find mouse still "captured". Must restart app.
**Why it happens:** Capturing mousedown, releasing on mouseup, but Alt-Tab prevents mouseup delivery.
**How to avoid:** Release capture on window deactivation (WindowEvent::Focused(false)). Also release on Escape key.
**Warning signs:** Mouse "stuck" after task switching. Drag operations don't end properly.

### Pitfall 7: Keybinding Conflicts Without Context
**What goes wrong:** Global keybinding triggers when typing in text field. Ctrl+A selects all text AND triggers app-wide action.
**Why it happens:** All keybindings active simultaneously without context filtering.
**How to avoid:** Context-aware keybinding matching. Text input context disables most global shortcuts. Most specific context wins.
**Warning signs:** Can't type certain letters in text fields. Keyboard shortcuts trigger inappropriately.

## Code Examples

Verified patterns from official sources:

### Element with Hitbox Registration
```rust
// Pattern: Interactive element with click handling
// Source: GPUI prepaint pattern (DeepWiki)

pub struct Button {
    element_id: ElementId,
    on_click: Option<Box<dyn Fn(&mut EventContext)>>,
}

impl Element for Button {
    type RequestLayoutState = ButtonState;

    fn request_layout(&mut self, cx: &mut LayoutContext) -> (LayoutId, Self::RequestLayoutState) {
        let layout_id = cx.request_layout(&self.style);
        let state = ButtonState { layout_id };
        (layout_id, state)
    }

    fn prepaint(&mut self, state: &mut Self::RequestLayoutState, cx: &mut PrepaintContext) {
        let bounds = cx.bounds(state.layout_id);
        // Register hitbox AFTER layout is resolved
        cx.register_hitbox(bounds, self.element_id, HitboxType::Opaque);
    }

    fn paint(&mut self, state: &mut Self::RequestLayoutState, cx: &mut PaintContext) {
        let bounds = cx.bounds(state.layout_id);
        cx.paint_styled_rect(&self.style, &bounds);
    }
}
```

### Focus-Aware View
```rust
// Pattern: View with keyboard focus and input handling
// Source: GPUI interactivity (0xshadow blog)

pub struct TextInput {
    focus_handle: FocusHandle,
    content: String,
}

impl TextInput {
    pub fn new(cx: &mut ViewContext) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            content: String::new(),
        }
    }
}

impl View for TextInput {
    fn render(&mut self, cx: &mut ViewContext) -> impl Element {
        div()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_backspace))
            .when(cx.is_focused(&self.focus_handle) && cx.focus_source_is_keyboard(), |div| {
                div.border_color(blue()).border_width(px(2.0))
            })
            .child(text(&self.content))
    }
}

impl TextInput {
    fn on_backspace(&mut self, _action: &Backspace, cx: &mut ViewContext) {
        self.content.pop();
        cx.notify();
    }
}
```

### Context-Aware Keybinding Matching
```rust
// Pattern: Keybindings respect context (focused element type)
// Source: GPUI KeyContext pattern (DeepWiki)

pub struct KeyContext {
    element_type: &'static str,
    state: HashMap<String, String>,
}

pub struct Keymap {
    bindings: Vec<KeyBinding>,
}

pub struct KeyBinding {
    keystroke: Keystroke,
    action: Box<dyn Action>,
    context_predicate: Option<Box<dyn Fn(&KeyContext) -> bool>>,
}

impl Keymap {
    pub fn match_action(&self, keystroke: &Keystroke, context: &KeyContext) -> Option<&dyn Action> {
        // Most specific match wins (later bindings override earlier)
        self.bindings.iter().rev()
            .find(|binding| {
                binding.keystroke == *keystroke &&
                binding.context_predicate.as_ref()
                    .map(|pred| pred(context))
                    .unwrap_or(true) // No predicate = always matches
            })
            .map(|binding| binding.action.as_ref())
    }
}
```

### Mouse Capture Pattern
```rust
// Pattern: Drag operation with mouse capture
// Source: Windows SetCapture (CodeProject), adapted to Rust

pub struct Slider {
    element_id: ElementId,
    value: f32,
    dragging: bool,
}

impl Slider {
    fn on_mouse_down(&mut self, event: &MouseDownEvent, cx: &mut EventContext) {
        if event.button == MouseButton::Left {
            self.dragging = true;
            cx.capture_mouse(self.element_id, MouseButton::Left);
            self.update_value_from_position(event.position);
            cx.notify();
        }
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, cx: &mut EventContext) {
        // Receives events even when cursor leaves element (due to capture)
        if self.dragging {
            self.update_value_from_position(event.position);
            cx.notify();
        }
    }

    fn on_mouse_up(&mut self, event: &MouseUpEvent, cx: &mut EventContext) {
        if event.button == MouseButton::Left && self.dragging {
            self.dragging = false;
            cx.release_mouse_capture();
            cx.notify();
        }
    }

    // CRITICAL: Also release on window deactivation
    fn on_window_deactivated(&mut self, cx: &mut EventContext) {
        if self.dragging {
            self.dragging = false;
            cx.release_mouse_capture();
        }
    }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| String-based actions | Typed Action trait | ~2023 (GPUI 2.0) | Compile-time keybinding validation, refactoring support |
| MouseEnter/MouseLeave events | Query-based hover state | Modern frameworks | Eliminates event ordering bugs, simplifies state |
| Global focus variable | FocusHandle with refcounting | ~2023 | Survives re-renders, supports weak references |
| Always-visible focus ring | :focus-visible / modality tracking | CSS3 spec (2020s) | Keyboard-only focus rings, better UX |
| Single-phase dispatch | Two-phase capture/bubble | DOM Level 2 (2000) | Allows global handlers (capture) and local handlers (bubble) |

**Deprecated/outdated:**
- **String-based action identifiers:** Prone to typos, no compile-time checking. Use typed Action trait.
- **Per-element hover state tracking:** Causes inconsistencies. Framework tracks hover, elements query.
- **Always showing focus rings:** Bad UX. Track input modality, only show on keyboard focus.
- **Forward Z-order hit testing:** Background receives clicks. Iterate in reverse for topmost-first.

## Open Questions

Things that couldn't be fully resolved:

1. **Optimal ElementId representation**
   - What we know: Needs Copy, Eq, Hash for use as HashMap key. Must be stable across frames.
   - What's unclear: u64 vs (u32 generation, u32 index) vs UUID. Performance vs collision tradeoffs.
   - Recommendation: Start with u64 (simple, fast). Profile if HashMap lookups become bottleneck.

2. **Action trait object storage vs enum**
   - What we know: GPUI uses Box<dyn Action> for dynamic dispatch. Alternative: enum of all actions.
   - What's unclear: Which is more ergonomic for user code? Performance difference likely negligible.
   - Recommendation: Use trait objects (Box<dyn Action>) for extensibility. Users define custom actions without modifying framework enum.

3. **Tab order algorithm complexity**
   - What we know: Need to visit focusable elements in tree order (depth-first). Tab index overrides default order.
   - What's unclear: Should we compute tab order every frame or cache until tree changes?
   - Recommendation: Compute during prepaint phase (tree is stable). Cache until next prepaint.

4. **Event handler storage strategy**
   - What we know: Handlers registered during element construction/render. Need efficient lookup by (element_id, event_type).
   - What's unclear: HashMap<(ElementId, TypeId), Handler> vs per-element Vec<Handler> vs separate HashMaps per event type.
   - Recommendation: HashMap<ElementId, ElementHandlers> where ElementHandlers contains per-event-type vectors. Balance lookup speed with memory.

## Sources

### Primary (HIGH confidence)
- GPUI Event Flow and Input Handling: https://deepwiki.com/zed-industries/zed/2.4-keybinding-and-action-dispatch
- GPUI Focus Management: https://deepwiki.com/zed-industries/zed/2.5-keybinding-and-action-system
- GPUI Interactivity Tutorial: https://blog.0xshadow.dev/posts/learning-gpui/gpui-interactivity/
- winit Event Documentation: https://docs.rs/winit/latest/winit/event/index.html
- winit KeyEvent Structure: https://docs.rs/winit/latest/winit/event/struct.KeyEvent.html

### Secondary (MEDIUM confidence)
- DOM Event Propagation (capture/bubble): https://javascript.info/bubbling-and-capturing
- CSS :focus-visible specification: https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Selectors/:focus-visible
- Mouse Capture Pattern (SetCapture): https://www.codeproject.com/Tips/127813/Using-SetCapture-and-ReleaseCapture-correctly-usua
- Visual Studio Code Context-Aware Keybindings: https://code.visualstudio.com/docs/configure/keybindings
- High-DPI Coordinate Transformation: https://learn.microsoft.com/en-us/windows/win32/learnwin32/mouse-movement

### Tertiary (LOW confidence)
- keybinds crate (type-safe keybinding library): https://crates.io/crates/keybinds
- keyboard-types crate (UI Events spec types): https://crates.io/crates/keyboard-types
- PowerToys hotkey conflict detection: https://www.windowscentral.com/software-apps/windows-11-hotkey-conflicts-are-a-pain-powertoys-just-introduced-a-smarter-way-to-manage-them

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - No external crates needed, uses existing dependencies (winit, stdlib)
- Architecture patterns: MEDIUM - GPUI documentation verified via DeepWiki, but code patterns adapted/simplified for ora
- Pitfalls: MEDIUM - Common patterns from web research (DOM, CSS), verified with multiple sources
- Implementation details: LOW - ElementId representation, event handler storage require experimentation

**Research date:** 2026-01-29
**Valid until:** ~30 days (event system patterns are stable, but ora implementation may reveal gaps)
