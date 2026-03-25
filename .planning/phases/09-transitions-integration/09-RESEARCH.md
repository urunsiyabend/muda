# Phase 9: Transitions & Integration - Research

**Researched:** 2026-03-25
**Domain:** Rust UI animation system + wgpu_client-to-ora migration
**Confidence:** HIGH (all findings derived from direct codebase inspection)

## Summary

Phase 9 has two distinct work streams: (1) a CSS-like transition/animation system built
entirely inside ora, and (2) migrating wgpu_client from a large self-contained GPU app
to a 3-line shell that delegates to `ora::run()`.

The animation work is pure green-field inside ora — no external library is needed.
wgpu_client already has fully functional `Tween<T>` and `CubicBezier` / `Easing` types
that can be ported to ora verbatim. The easing math is correct; only the integration
point (where transitions live and how redraws are driven) needs design.

The migration work is a code-movement exercise. All UI logic already exists in ora
(AppLayout, TabBarView, SidebarView, TextAreaView, etc.) and consumes `core_editor`
types directly via `use core_editor::view_model::...`. The locked decision says ora
views must NOT import core_editor types directly — an adapter trait boundary is
required. This is the main architectural decision still to be designed.

**Primary recommendation:** Implement `TransitionState` as a per-element store in
`AppContext` (keyed by `HitboxId`), port the `Tween<T>` / `Easing` primitives from
wgpu_client, drive redraws by calling `window.request_redraw()` when any transition is
in-flight, then migrate wgpu_client using a thin `EditorModel` adapter trait.

---

## Standard Stack

This phase uses no new external dependencies. Everything is implemented in-house.

### Core (already in Cargo.toml)
| Crate | Version | Role | Notes |
|-------|---------|------|-------|
| `winit` | 0.30 | Window + event loop | `request_redraw()` drives animation frames |
| `wgpu` | 23 | GPU rendering | No changes needed |
| `std::time::Instant` | std | Animation timing | `Instant::now()` + elapsed |
| `core_editor` | path | Editor state source | Accessed via adapter trait, not directly |

### Primitives to Port from wgpu_client (HIGH confidence)

The following files in `wgpu_client/src/design_system/animation/` can be ported
directly to `ora/src/animation/`:

| File | Port to ora as | What it provides |
|------|---------------|-----------------|
| `tween.rs` | `ora::animation::Tween<T>` | Interpolation + retarget-from-current |
| `easing.rs` (CubicBezier) | `ora::animation::CubicBezier` | CSS cubic-bezier solver |
| `tokens/motion.rs` (Easing enum) | `ora::animation::Easing` | Linear/EaseIn/EaseOut/EaseInOut/Spring |

**Key types to preserve:**
- `Tween::retarget(&mut self, new_end: T)` — critical for interruption behavior (starts
  new animation from current interpolated value)
- `Tweenable` trait with `lerp()` — enables typed interpolation
- `Color` implements `Tweenable` already in wgpu_client; port to ora's `style::Color`

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Custom Tween | `keyframe` crate | Unnecessary dependency; our Tween is simpler and already tested |
| Hand-rolled bezier | `splines` crate | Our CubicBezier with Newton-Raphson is sufficient for CSS-compatible easing |
| Separate animation thread | Single-thread with `request_redraw` | Single-thread is correct for wgpu/winit; avoids synchronization |

---

## Architecture Patterns

### Recommended Project Structure

New module in ora:

```
ora/src/
├── animation/               # NEW: animation primitives
│   ├── mod.rs               # pub use tween, easing, transition
│   ├── tween.rs             # Tween<T> + Tweenable trait (ported from wgpu_client)
│   ├── easing.rs            # Easing enum + CubicBezier (ported from wgpu_client)
│   └── transition.rs        # TransitionSpec, TransitionState, AnimationClock
├── context.rs               # AppContext gains AnimationClock
├── element/
│   └── trait_def.rs         # PaintContext gains animation query methods
└── elements/
    └── div.rs               # Div gains transition_bg(), transition_opacity(), etc.
```

New adapter module in ora:

```
ora/src/
└── editor_adapter/          # NEW: core_editor adapter
    ├── mod.rs               # pub trait EditorModel + EditorModelExt
    └── core_editor_impl.rs  # impl EditorModel for core_editor::app::App
```

### Pattern 1: TransitionSpec + TransitionState (Per-Element)

**What:** Each element that wants transitions stores a `TransitionSpec` (what to
transition and at what duration/easing). At paint time, elements look up their
`TransitionState` in a global registry (keyed by `HitboxId`) to get the current
interpolated value.

**When to use:** Any Div-like element with `.transition_bg()` / `.transition_opacity()`

**Data model:**
```rust
// Source: codebase design (novel for this project)

/// Declared on element via builder methods — what CAN transition and how.
pub struct TransitionSpec {
    pub bg: Option<TransitionConfig>,
    pub opacity: Option<TransitionConfig>,
    pub size: Option<TransitionConfig>,
    pub position: Option<TransitionConfig>,
}

pub struct TransitionConfig {
    pub duration_ms: u32,
    pub easing: Easing,
}

/// Live state stored in AppContext — what IS currently transitioning.
pub struct TransitionState {
    pub bg_tween: Option<Tween<Color>>,
    pub opacity_tween: Option<Tween<f32>>,
    // etc.
    pub last_interaction: InteractionSnapshot, // what state were we in last frame
}

/// Registry in AppContext keyed by element stable identity.
/// HitboxId is regenerated each frame, so we need a stable key.
/// Use EntityId or a user-assigned TransitionId.
pub type TransitionRegistry = HashMap<TransitionId, TransitionState>;
```

**Identity challenge:** HitboxId is per-frame. Transitions need a stable cross-frame
identity. Options:
1. `TransitionId(u64)` assigned by the element and stored in its `RequestLayoutState`
2. Stable per view-position (fragile, breaks on conditional rendering)
3. Model entity ID (only works for model-backed elements)

**Recommendation:** Add `transition_id(id: TransitionId)` builder method on `Div`.
The `Div` element generates a `TransitionId` on first use (stored in a `Cell<Option<TransitionId>>`
field, lazily assigned from an AppContext counter). The registry in AppContext maps
`TransitionId -> TransitionState`.

### Pattern 2: Animation Frame Scheduling

**What:** While any transition is in-flight, the event loop must keep producing
frames at ~60fps.

**Current mechanism:** The event loop's `window_event` handler for
`WindowEvent::RedrawRequested` calls `gpu_state.window.request_redraw()` at the
end (line 336 in `event_loop.rs`) — this produces continuous redraws unconditionally.

**Targeted approach:** Add an `AnimationClock` to `AppContext` that tracks whether
any transition is active. After each paint phase, if any transition is in-flight,
call `window.request_redraw()`. If no transitions are active, stop.

```rust
// Source: codebase design
pub struct AnimationClock {
    active_count: usize,
}

impl AnimationClock {
    pub fn tick_animations(&mut self) -> bool {
        // Returns true if any animation is still running
        self.active_count > 0
    }
    pub fn register_active(&mut self) { self.active_count += 1; }
    pub fn register_complete(&mut self) { self.active_count = self.active_count.saturating_sub(1); }
}
```

**Event loop change:** Replace the unconditional `request_redraw()` after render with:
```rust
if self.app_context.animation_clock.has_active_animations() {
    gpu_state.window.request_redraw();
}
```

**Note:** The existing code calls `request_redraw()` unconditionally after every frame
(line 336). This is fine for now — the transition system can simply leave this in place
(continuous redraws already happen). The optimization to stop redraws when idle is
a refinement, not a requirement for Phase 9.

### Pattern 3: Div Paint-Time Interpolation

**What:** During the paint phase, `Div::paint()` checks its `TransitionSpec`, looks up
the corresponding `TransitionState` in the registry, advances the tween, and uses the
interpolated value in place of the declared value.

**Current flow in `Div::paint()`:**
```rust
// Current: instant state switch
if cx.is_active(hitbox_id) {
    if let Some(active_bg) = self.active_bg {
        style.background = Background::Solid(active_bg);
    }
} else if cx.is_hovered(hitbox_id) {
    if let Some(hover_bg) = self.hover_bg {
        style.background = Background::Solid(hover_bg);
    }
}
```

**Transitioned flow:**
```rust
// With transitions: read interpolated value from TransitionState
let target_bg = if cx.is_active(hitbox_id) {
    self.active_bg.unwrap_or(base_bg)
} else if cx.is_hovered(hitbox_id) {
    self.hover_bg.unwrap_or(base_bg)
} else {
    base_bg
};

// Look up or create TransitionState for this element
if let Some(spec) = &self.transition_spec {
    if let Some(bg_config) = &spec.bg {
        let state = cx.get_or_create_transition(self.transition_id);
        let current = state.advance_bg(target_bg, bg_config);
        style.background = Background::Solid(current);
    }
} else {
    style.background = Background::Solid(target_bg);
}
```

**PaintContext needs:** A method to access the `TransitionRegistry` for get/update.
The `AppContext` owns the registry, and `PaintContext` holds a `*const AppContext`
pointer — this needs to become `*mut AppContext` or the registry must be accessible
through a separate mechanism (e.g., a `RefCell<TransitionRegistry>` in AppContext).

**Recommendation:** Add `pub(crate) transition_registry: RefCell<TransitionRegistry>`
to `AppContext`. Access via `PaintContext::transition_state_mut(id)`.

### Pattern 4: Adapter Trait for core_editor

**Current state (VIOLATION of locked decision):** `ora/src/views/tab_bar.rs` line 12:
```rust
use core_editor::view_model::{TabBarPresentation, TabPresentation};
```
All ora views currently import `core_editor` types directly. This violates the locked
decision that "ora views do NOT import core_editor types directly."

**Fix:** Define presentation types in ora (or a shared crate) and provide a conversion
from core_editor types. Two approaches:

Option A — **Mirror types in ora** (recommended for Phase 9):
Define `ora::editor_adapter::TabBarPresentation` that mirrors the core_editor struct.
The wgpu_client adapter implements `From<core_editor::view_model::TabBarPresentation>
for ora::editor_adapter::TabBarPresentation`. Views use only the ora-side type.

Option B — **Trait-based abstraction**:
Define `trait EditorModel` with methods that return presentation data. The adapter
implements `EditorModel` wrapping a `core_editor::app::App`. Views receive `&dyn
EditorModel`.

**Recommendation: Option A (Mirror types) for Phase 9.** It's simpler, the existing
types are already correct, and the migration is mechanical (copy struct definitions,
add `From` impls). Option B is better long-term but requires refactoring all view
render() signatures.

**Minimum viable adapter:**
```rust
// ora/src/editor_adapter/mod.rs
// Source: codebase design

/// Adapter trait: abstract over any editor backend.
pub trait EditorDataSource {
    fn build_render_model(&self, viewport_height: usize) -> RenderModel;
    fn dispatch_editor_command(&mut self, cmd: EditorCommand);
    // ... etc
}

/// Concrete implementation wrapping core_editor.
pub struct CoreEditorAdapter {
    app: core_editor::app::App,
}

impl EditorDataSource for CoreEditorAdapter {
    fn build_render_model(&self, viewport_height: usize) -> RenderModel {
        self.app.build_render_model(viewport_height)
    }
    // etc.
}
```

This means ora's `AppLayout` is initialized with a `Box<dyn EditorDataSource>` (or
generic `E: EditorDataSource`) rather than calling `core_editor` methods directly.

### Pattern 5: Enter/Exit Transitions

**What:** Toast notifications (slide-in/fade-in on show) and dialog (scale-up on
appear) need enter/exit animations.

**Approach:** The `Toast` and `Dialog` elements have a lifecycle:
- `Enter`: started when `show()` is called; runs from t=0 to t=1
- `Exit`: started when `dismiss()` is called; runs from t=1 to t=0

Store an `EnterExitState` alongside the element:
```rust
pub enum AnimationPhase { Entering, Visible, Exiting, Hidden }
pub struct EnterExitTween {
    phase: AnimationPhase,
    tween: Tween<f32>,  // 0.0..1.0
}
```

During paint, multiply opacity by `tween.value()`. On exit complete, set
`Display::None`.

### Anti-Patterns to Avoid

- **Storing Tween in Element struct:** Elements are recreated every frame. Transition
  state MUST live in AppContext, not in the element. The element only stores the
  `TransitionId` and `TransitionSpec`.
- **Using HitboxId as transition key:** HitboxIds are assigned fresh each frame.
  Use a stable `TransitionId` assigned in `Div::new()` or via builder.
- **Animating layout properties during a transition:** Animating `width`/`height`
  requires re-layout each frame (expensive). Keep size animations as a separate
  fast path that only triggers re-layout when size changes, not every frame.
- **Forgetting to check `Tween::retarget()` on interruption:** If hover state changes
  while a bg transition is in-flight, call `retarget(new_end)` not `restart()`. The
  `retarget` method starts from the current interpolated value — this is how VS
  Code/Zed avoid visual pops.
- **Migrating all components at once:** The locked decision says incremental migration
  one component at a time. Do NOT attempt a big-bang migration.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Easing math | Custom cubic polynomial | Port `CubicBezier` + `Easing` from wgpu_client | Already written, tested, CSS-compatible |
| Color interpolation | Custom RGBA lerp | Port `Tweenable for Color` from wgpu_client | Handles per-channel lerp correctly |
| Animation frame scheduling | Platform timer / thread sleep | `winit::window::Window::request_redraw()` | Correct approach for winit event loop; already in codebase |
| Key event translation | Re-implement in ora | Port `translate_key()` from `wgpu_client/src/input.rs` | 90 lines, complete, handles all editor commands |
| Design tokens | Inline constants | `ora::theme` tokens (already exist) | Single source of truth; wgpu_client tokens deleted |

**Key insight:** The heavy lifting (tween math, easing, color lerp, key translation)
already exists in wgpu_client. Phase 9 is mostly moving code into the right place,
not writing new algorithms.

---

## Common Pitfalls

### Pitfall 1: Element Struct Storing Transition State

**What goes wrong:** A Div storing `tween: Option<Tween<Color>>` in its struct.
On the next frame, a fresh Div is constructed (render() is called), and the tween
is lost — all animations restart from scratch every frame.

**Why it happens:** The GPUI-style element model creates elements fresh each render()
call. Elements are ephemeral per-frame renderers, not persistent objects.

**How to avoid:** TransitionState lives in `AppContext::transition_registry: HashMap
<TransitionId, TransitionState>`. The Div's struct holds only `transition_id:
Option<TransitionId>` and `transition_spec: Option<TransitionSpec>`.

**Warning signs:** Animation appears to flicker or never progresses past frame 1.

### Pitfall 2: HitboxId Used as Stable Key

**What goes wrong:** Keying transition state by `HitboxId` — which is a monotonically
increasing counter reset each frame. Two different elements end up sharing state,
or state is lost between frames.

**Why it happens:** `HitboxId` is assigned in `PrepaintContext::register_hitbox()` from
`next_hitbox_id: u64` which starts at 0 each frame.

**How to avoid:** Use a `TransitionId` that is stable across frames. Assign it once
when the element is created and store it in the element's spec.

### Pitfall 3: Direct core_editor Imports Remaining in ora Views

**What goes wrong:** After migration, ora views still `use core_editor::view_model::...`
directly. This violates the adapter boundary, couples ora to core_editor's internal
types, and prevents ora from ever supporting a different editor backend.

**Why it happens:** All existing ora views (TabBarView, SidebarView, etc.) currently
import core_editor types. This is the pre-migration state.

**How to avoid:** As part of each component migration, replace the `use core_editor::...`
import with the mirrored ora type. Add a compiler check: ora's Cargo.toml should
eventually remove the direct `core_editor` dependency (or scope it to the adapter
module only).

**Warning signs:** `grep -r "use core_editor" ora/src/views/` returns results.

### Pitfall 4: Size/Position Transitions Triggering Full Re-Layout

**What goes wrong:** Animating `width` or `height` causes `request_layout()` and
`compute_flexbox()` to run every frame at 60fps. For complex layouts this is ~1-3ms
per frame and can cause jank.

**Why it happens:** Size is a layout property. Any change requires a full layout pass.

**How to avoid:** For size/position transitions, apply the animated offset as a
transform in the paint step only (modify `Rect` bounds at paint time, not at layout
time). The layout stays fixed; only the final drawn rect moves. This is how CSS
`transform: translateX()` works — it doesn't affect layout.

**Warning signs:** CPU spikes when hover/active transitions are active on many elements.

### Pitfall 5: Unconditional Continuous Redraw After Animation

**What goes wrong:** The current event loop calls `request_redraw()` after every
`RedrawRequested` event unconditionally (line 336 in `event_loop.rs`). This is fine
for development but consumes unnecessary CPU/GPU when no animations are in-flight.

**Why it happens:** The current code has a TODO comment about this — it was left as
a development convenience.

**How to avoid:** Condition `request_redraw()` on either dirty entities OR active
animations. For Phase 9, the existing unconditional approach is acceptable — add the
optimization after the transition system works.

### Pitfall 6: wgpu_client Migration Breaking Compilation

**What goes wrong:** Migrating a component while it still has callers in wgpu_client
breaks the build mid-migration.

**How to avoid:** Follow the incremental strategy from CONTEXT.md:
1. Verify ora has the equivalent component
2. Wire the ora component to use the adapter trait
3. Delete the wgpu_client component
4. Verify build passes
5. Move to next component

Never leave the build broken overnight.

---

## Code Examples

### Example 1: Porting Tween (verified from wgpu_client source)

```rust
// Source: wgpu_client/src/design_system/animation/tween.rs
// Port to: ora/src/animation/tween.rs
// Changes: update `use crate::theme::Color` to `use crate::style::Color`

pub trait Tweenable: Clone + PartialEq {
    fn lerp(&self, other: &Self, t: f32) -> Self;
}

impl Tweenable for Color {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        Color {
            r: self.r + (other.r - self.r) * t,
            g: self.g + (other.g - self.g) * t,
            b: self.b + (other.b - self.b) * t,
            a: self.a + (other.a - self.a) * t,
        }
    }
}

// Tween::retarget is the key method for interruption-safe transitions:
pub fn retarget(&mut self, new_end: T) {
    self.start = self.current.clone();  // continue from current position
    self.end = new_end;
    self.start_time = Some(Instant::now());
    self.state = TweenState::Playing;
}
```

### Example 2: Div Builder API (designed for Phase 9)

```rust
// Source: codebase design — extends ora/src/elements/div.rs

impl Div {
    /// Animate background color changes with the given duration.
    pub fn transition_bg(mut self, duration_ms: u32) -> Self {
        self.transition_spec
            .get_or_insert_with(TransitionSpec::default)
            .bg = Some(TransitionConfig {
                duration_ms,
                easing: Easing::EaseOut, // global default per CONTEXT
            });
        self
    }

    /// Override easing for the most recently added transition.
    pub fn easing(mut self, easing: Easing) -> Self {
        if let Some(spec) = &mut self.transition_spec {
            // Apply to last configured transition property
            if let Some(bg) = &mut spec.bg {
                bg.easing = easing;
            }
        }
        self
    }

    /// Animate opacity changes.
    pub fn transition_opacity(mut self, duration_ms: u32) -> Self {
        self.transition_spec
            .get_or_insert_with(TransitionSpec::default)
            .opacity = Some(TransitionConfig {
                duration_ms,
                easing: Easing::EaseOut,
            });
        self
    }
}
```

### Example 3: Duration Extension Trait

```rust
// Source: codebase design — convenience API matching CONTEXT.md examples

pub trait DurationExt {
    fn ms(self) -> u32;
}

impl DurationExt for u32 {
    fn ms(self) -> u32 { self }
}

// Usage: .transition_bg(200.ms())
// This matches exactly the API surface specified in CONTEXT.md
```

### Example 4: wgpu_client main.rs After Migration

```rust
// Source: codebase design — INT-01 target
// File: wgpu_client/src/main.rs (after migration)

fn main() {
    let app = ora::App::new()
        .title("Muda Editor")
        .size(1200, 800)
        .on_open(|cx| {
            // Create editor adapter
            let adapter = CoreEditorAdapter::new();
            let root_view = ora::views::AppLayout::from_editor(adapter);
            cx.set_root_view(root_view);
        });

    ora::run(app);
}
```

### Example 5: Key Translation Port

```rust
// Source: wgpu_client/src/input.rs — port to ora/src/events/editor_input.rs
// The translate_key() function maps winit KeyEvent -> EditorCommand
// This moves entirely to ora so wgpu_client doesn't need to know about EditorCommand

// After port, ora's action system dispatches EditorCommand actions
// via cx.dispatch_action(&EditorCommand::MoveCursor { ... })
```

---

## State of the Art

| Old Approach | Current Approach | Change | Impact |
|---|---|---|---|
| Instant state switch in Div::paint() | Interpolated via TransitionState | Phase 9 | Smooth hover/active visual feedback |
| wgpu_client imports core_editor directly | ora adapter trait boundary | Phase 9 | Decoupling; allows future editor backends |
| wgpu_client contains all UI logic | ora contains all UI logic | Phase 9 | wgpu_client reduced to ~10 lines |
| Continuous unconditional redraws | Conditional on dirty/animations | Future | CPU/GPU savings when idle |

**Currently in pre-migration state:**
- `ora/src/views/tab_bar.rs` imports `core_editor::view_model::TabBarPresentation`
  directly. All other views do similarly. This must be fixed as part of INT migration.

---

## Migration Order Recommendation (Claude's Discretion)

The locked decision leaves component migration order to Claude. Based on dependency
analysis:

1. **Create adapter trait first** (EditorDataSource / EditorModel): This unblocks all
   component migrations. Without it, any ported component still imports core_editor.

2. **StatusBarView** — smallest, fewest data fields (cursor position, language, file
   path). Lowest risk validation of the adapter pattern.

3. **TabBarView** — tabs are simple list items; validates adapter with more complex
   data (dirty bit, active state).

4. **GutterView** — line numbers only, trivial data.

5. **SidebarView** — file tree data, selection state. More complex but follows same
   pattern.

6. **TextAreaView + AppLayout** — largest/most critical. Migrate last when pattern is
   proven.

7. **Input translation** — port `translate_key()` to ora, wire to action dispatch.
   Move after component migrations so the editor responds to keys in the new path.

8. **Delete wgpu_client UI logic** — after all components verified in ora, strip
   wgpu_client down to 10 lines.

---

## Easing Defaults per Property (Claude's Discretion)

Based on studying VS Code/Zed behavior patterns and the existing wgpu_client defaults:

| Property | Default Easing | Rationale |
|----------|---------------|-----------|
| `background` (hover/active) | `EaseOut` | Enters fast, settles — responsive feel |
| `opacity` | `EaseOut` | Same — snappy to visible, smooth to hidden |
| `position/transform` | `EaseInOut` | Movement benefits from smooth start/end |
| `size` | `EaseOut` | Expand fast; collapse with `EaseIn` on exit |
| Enter (appear) | `EaseOut` | Content arrives quickly |
| Exit (disappear) | `EaseIn` | Content departs quickly (not lingering) |

Global default: `EaseOut` (matches existing wgpu_client default `Easing::EaseOut`).

---

## Open Questions

1. **RefCell vs raw pointer for TransitionRegistry in PaintContext**
   - What we know: `PaintContext` holds `*const AppContext`. Advancing tweens during
     paint requires mutable access to the registry.
   - What's unclear: Whether to use `*mut AppContext` (existing pattern for entity_storage),
     `RefCell<TransitionRegistry>`, or a separate `Cell`-wrapped registry.
   - Recommendation: Use the same `*const AppContext` -> `unsafe { &mut *(...) }` pattern
     that `PaintContext::entity_storage` already uses. The registry is only written
     during paint, same safety contract.

2. **TransitionId assignment strategy**
   - What we know: IDs must be stable across frames; elements are recreated each frame.
   - What's unclear: Whether a `Cell<Option<TransitionId>>` in the Div struct is
     acceptable (elements ARE recreated but the View re-creates them with the same
     code path, so the Cell would be reset).
   - Recommendation: TransitionId is assigned by the caller via `.transition_id(id)`
     builder method, not auto-generated. Views that want transitions assign IDs
     explicitly (e.g., a static constant per element site). This is explicit and
     avoids the Cell complexity.

3. **core_editor dependency in ora's Cargo.toml**
   - What we know: `ora/Cargo.toml` currently has `core_editor = { path = "../core_editor" }`.
     The adapter boundary decision says views should not import core_editor types directly.
   - What's unclear: Whether the adapter impl (`CoreEditorAdapter`) lives in ora or
     in wgpu_client.
   - Recommendation: The adapter implementation lives in `wgpu_client`. ora defines
     the `EditorDataSource` trait but does not implement it for core_editor. This
     removes the `core_editor` dependency from ora entirely — correct long-term.
     The trait and mirrored presentation types live in a new `ora::editor_adapter` module.

---

## Sources

### Primary (HIGH confidence — direct codebase inspection)

All findings are derived from reading the actual source files. No external sources
were needed because this is a custom in-house codebase.

Key files read:
- `wgpu_client/src/design_system/animation/tween.rs` — Tween<T> implementation
- `wgpu_client/src/design_system/animation/easing.rs` — CubicBezier, Spring
- `wgpu_client/src/design_system/tokens/motion.rs` — Easing enum with cubic math
- `ora/src/elements/div.rs` — Current Div with instant state switching
- `ora/src/element/trait_def.rs` — PaintContext, LayoutContext, Element trait
- `ora/src/context.rs` — AppContext structure
- `ora/src/platform/event_loop.rs` — Event loop, RedrawRequested handler
- `ora/src/views/tab_bar.rs` — Direct core_editor import (INT-violation example)
- `ora/src/views/mod.rs` — All current views
- `ora/src/views/app_layout.rs` — AppLayout structure
- `wgpu_client/src/input.rs` — translate_key() and AppAction translation
- `wgpu_client/src/app.rs` — WgpuApp render loop and core_editor usage
- `wgpu_client/src/main.rs` — Current main.rs to be replaced
- `core_editor/src/view_model/builder.rs` — ViewModelBuilder::build() signature
- `ora/src/lib.rs` — Current public API surface

### Secondary (none needed)

All research was codebase-internal. No external library documentation was required
because all dependencies are already decided and in use.

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies; all code is already in the project
- Architecture patterns: HIGH — based on direct code reading; patterns fit existing conventions
- Pitfalls: HIGH — identified by reading actual implementations and spotting the issues
- Migration order: MEDIUM — logical analysis; actual effort may differ by component

**Research date:** 2026-03-25
**Valid until:** 2026-04-25 (stable codebase; 30 days)
