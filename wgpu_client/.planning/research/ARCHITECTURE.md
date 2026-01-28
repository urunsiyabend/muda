# Architecture Research: GPU-Rendered IDE Layout and Clipping

**Domain:** GPU-accelerated IDE UI with wgpu
**Researched:** 2026-01-28
**Confidence:** HIGH

## Standard Architecture for wgpu UI Layout Systems

### System Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    Application Layer                         │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ WgpuApp (winit ApplicationHandler)                   │   │
│  │ - Window lifecycle, event routing                     │   │
│  └────────────────────────┬─────────────────────────────┘   │
├────────────────────────────┼──────────────────────────────────┤
│        State Layer         │                                  │
│  ┌─────────────────────────▼──────────────────────────────┐  │
│  │ EditorApp (core_editor integration)                    │  │
│  │ - Document management, command dispatch               │  │
│  └────────────────────────┬───────────────────────────────┘  │
├────────────────────────────┼──────────────────────────────────┤
│      Renderer Layer        │                                  │
│  ┌─────────────────────────▼──────────────────────────────┐  │
│  │ GpuRenderer                                            │  │
│  │ - Layout calculation (Bounds hierarchy)               │  │
│  │ - Component orchestration                              │  │
│  │ - Render pass assembly                                 │  │
│  └────────────┬────────────────────────────┬──────────────┘  │
├───────────────┼────────────────────────────┼──────────────────┤
│  Components   │                            │  Design System   │
│  ┌────────────▼──────────┐  ┌──────────────▼────────────┐    │
│  │ TextArea              │  │ StyledRectRenderer        │    │
│  │ Gutter                │  │ UITextRenderer            │    │
│  │ StatusBar             │  │ FlexLayout                │    │
│  │ Sidebar               │  │ Bounds (clipping struct)  │    │
│  │ TabBar                │  │ LayoutConstraints         │    │
│  │ Dialog                │  │ Design Tokens             │    │
│  └───────────────────────┘  └───────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | Boundary Enforcement |
|-----------|---------------|---------------------|
| **Bounds** | Define rectangular clipping region (x, y, width, height) | Hard boundary via split_horizontal/split_vertical |
| **GpuRenderer** | Calculate layout hierarchy, orchestrate render passes | Owns all component bounds, passes to prepare() |
| **TextArea** | Render text with glyphon TextBounds | Uses bounds parameter to set glyphon clipping rect |
| **StyledRectRenderer** | Batch render UI rectangles | Converts logical bounds to NDC, no built-in clipping |
| **RectRenderer** | Render selection/highlight backgrounds | Manual NDC conversion, no clipping enforcement |
| **glyphon** | Text rasterization and atlas management | TextBounds enforces clipping automatically |

## Layout System Architecture

### Current Implementation: Hierarchical Split-Based Layout

The existing codebase uses a **split-based layout system** where a parent `Bounds` is progressively subdivided:

```rust
// Example from renderer/mod.rs:calculate_layout()
let total = Bounds::new(0.0, 0.0, screen_width, screen_height);

// 1. Split off status bar at bottom
let (main_area, status_bar) = total.split_vertical(screen_height - status_height);

// 2. Split sidebar from left
let (sidebar, right_area) = if model.sidebar.visible {
    main_area.split_horizontal(sidebar_width)
} else {
    (Bounds::default(), main_area)
};

// 3. Split tab bar from top
let (tab_bar, editor_area) = right_area.split_vertical(tab_bar_height);

// 4. Split gutter from editor
let (gutter, text_area) = editor_area.split_horizontal(gutter_width);
```

**Characteristics:**
- **Imperative, sequential splits** — each component carved out in dependency order
- **Parent-child containment** — children cannot exceed parent bounds by construction
- **Fixed hierarchy** — no dynamic reflow, order matters
- **Explicit sizing** — components declare fixed or content-based dimensions

### Alternative Approaches (Not Currently Used)

#### Constraint-Based Layout (Cassowary Algorithm)

Used by Apple's Auto Layout and many modern UI frameworks ([Cassowary documentation](https://cassowary.readthedocs.io/en/latest/topics/theory.html), [ACM paper](https://dl.acm.org/doi/10.1145/504704.504705)).

**How it works:**
- Define constraints: `component.width >= 200`, `sidebar.right == editor.left`
- Solver calculates layout satisfying all constraints
- Handles conflicting constraints with priority system

**Trade-offs:**
- ✅ Flexible, handles complex relationships
- ✅ Responsive layouts with minimal code
- ❌ Solver overhead every frame (not suitable for immediate mode)
- ❌ Debugging is harder (implicit positioning)

**When to use:** Retained-mode UI frameworks where layout is cached between frames (not suitable for this project's immediate-mode approach).

#### Flexbox Layout

Implemented in `design_system/layout/flex.rs` but **not used for main layout**. Currently used only for widget-level composition.

**How it works:**
```rust
let layout = FlexLayout::row()
    .with_gap(8.0)
    .with_justify_content(JustifyContent::SpaceBetween);

let result = layout.layout(constraints, &child_sizes);
// Returns: LayoutResult { size, children: Vec<Bounds> }
```

**Trade-offs:**
- ✅ Intuitive for row/column layouts
- ✅ Handles dynamic content sizing
- ❌ Requires two-pass layout (measure, then position)
- ❌ More complex than simple splits for fixed hierarchies

**When to use:** Dynamic widget composition (toolbars, button groups) where child count/size varies.

## Clipping Patterns in wgpu

### Pattern 1: Scissor Rectangle (Hardware Clipping)

**What:** GPU discards fragments outside a rectangular region during rasterization ([wgpu docs](https://docs.rs/wgpu/latest/wgpu/struct.RenderPass.html), [WebGPU spec](https://developer.mozilla.org/en-US/docs/Web/API/GPURenderPassEncoder/setScissorRect)).

**Implementation:**
```rust
render_pass.set_scissor_rect(x: u32, y: u32, width: u32, height: u32);
// All subsequent draws clipped to this rectangle
```

**Characteristics:**
- ✅ **Zero cost** — handled by GPU rasterizer
- ✅ **Pixel-perfect** — exact boundary enforcement
- ✅ **Multiple regions** — can set different scissor per draw call
- ❌ **Axis-aligned only** — no rotation, no rounded corners
- ❌ **Flat hierarchy** — doesn't compose (last set wins)

**Use case:** Clip text/rectangles to container bounds. **This is the missing piece** for preventing overflow bugs.

**Current gap:** The codebase doesn't use `set_scissor_rect()` anywhere. Components rely on glyphon's TextBounds (which does software clipping) and manual bounds checks. Rectangles drawn outside bounds are still rasterized.

### Pattern 2: Stencil Buffer (Nested Clipping)

**What:** Use stencil buffer to create complex clipping masks, including nested regions ([Learn WebGPU stencil](https://maxammann.org/posts/2022/01/wgpu-stencil-testing/), [GameDev discussion](https://www.gamedev.net/forums/topic/637523-multi-level-stencil-buffers-or-other-ways-to-cut-away-child-gui-elements/)).

**Implementation:**
```rust
// Pass 1: Draw mask to stencil buffer
depth_stencil: Some(wgpu::DepthStencilState {
    format: wgpu::TextureFormat::Depth24PlusStencil8,
    stencil: wgpu::StencilState {
        front: wgpu::StencilFaceState {
            compare: wgpu::CompareFunction::Always,
            fail_op: wgpu::StencilOperation::Keep,
            depth_fail_op: wgpu::StencilOperation::Keep,
            pass_op: wgpu::StencilOperation::Replace,
        },
        // Write 1 to stencil where mask is drawn
    },
    ...
});

// Pass 2: Draw content, test against stencil
stencil: wgpu::StencilState {
    front: wgpu::StencilFaceState {
        compare: wgpu::CompareFunction::Equal, // Only draw where stencil == 1
        ...
    },
}
```

**Characteristics:**
- ✅ **Arbitrary shapes** — not limited to rectangles
- ✅ **Composable** — nested masks via stencil value increments
- ❌ **Two render passes minimum** — mask, then content
- ❌ **Breaks batching** — performance penalty ([Unity docs on nested masking](https://docs.unity3d.com/6000.3/Documentation/Manual/UIE-masking.html))
- ❌ **Requires depth/stencil texture** — memory overhead

**Use case:** Rounded container clipping (dialogs, tooltips), nested scrollable regions.

**Current gap:** No stencil usage. Not needed for axis-aligned IDE panels, but would be required for rounded dialog clipping or complex widget hierarchies.

### Pattern 3: TextBounds (glyphon Software Clipping)

**What:** glyphon's built-in clipping for text rendering ([glyphon source](https://docs.rs/glyphon/latest/src/glyphon/text_render.rs.html)).

**Implementation:**
```rust
glyphon::TextArea {
    bounds: TextBounds {
        left: (bounds.x * scale_factor) as i32,
        top: (bounds.y * scale_factor) as i32,
        right: ((bounds.x + bounds.width) * scale_factor) as i32,
        bottom: ((bounds.y + bounds.height) * scale_factor) as i32,
    },
    ...
}
```

**Characteristics:**
- ✅ **Automatic** — glyphon culls glyphs outside bounds
- ✅ **Per-glyph precision** — doesn't rasterize clipped text
- ❌ **Text only** — doesn't help with rectangles, images
- ⚠️ **Physical coordinates** — requires scale_factor multiplication

**Use case:** Already used correctly in TextArea, Gutter, StatusBar. **This part works.**

### Pattern 4: Manual Culling (Current Rectangle Approach)

**What:** Don't submit draw calls for rectangles outside viewport.

**Current implementation:** None. StyledRectRenderer and RectRenderer blindly draw all submitted rectangles, even if outside bounds.

**What should be added:**
```rust
pub fn push(&mut self, rect: &StyledRect, container_bounds: Bounds, screen_width: f32, screen_height: f32) {
    // Cull rectangles completely outside container
    if rect.x + rect.width < container_bounds.x || rect.x > container_bounds.x + container_bounds.width ||
       rect.y + rect.height < container_bounds.y || rect.y > container_bounds.y + container_bounds.height {
        return; // Don't queue for rendering
    }

    // Could also clip rect to container bounds here
    let clipped = rect.clip_to(container_bounds);
    self.instances.push(clipped.to_instance(screen_width, screen_height));
}
```

**Characteristics:**
- ✅ **Zero GPU cost** — don't draw what you can't see
- ✅ **Simple** — just bounding box math
- ❌ **CPU overhead** — every rect tested
- ⚠️ **Partial visibility** — need to clip, not just cull

## Recommended Clipping Strategy

Based on existing architecture and known issues (text overflow into tab bar, disappearing sidebar elements):

### Short-term Fix (Prevent Overflow Bugs)

1. **Add scissor rectangles to render passes**
   ```rust
   // In GpuRenderer::render(), before each component's render pass:
   {
       let mut pass = encoder.begin_render_pass(...);

       // Convert logical bounds to physical pixels
       let scissor_x = (bounds.x * scale_factor) as u32;
       let scissor_y = (bounds.y * scale_factor) as u32;
       let scissor_w = (bounds.width * scale_factor) as u32;
       let scissor_h = (bounds.height * scale_factor) as u32;

       pass.set_scissor_rect(scissor_x, scissor_y, scissor_w, scissor_h);

       component.render(&mut pass); // All draws clipped to bounds
   }
   ```

2. **Validate bounds propagation**
   - Audit `calculate_layout()` — ensure no overlapping regions
   - Log bounds before component.prepare() — catch zero-size or negative bounds
   - Add debug visualization (draw bounds as wireframes)

3. **Add overflow detection to StyledRectRenderer**
   - Log warning when rect exceeds screen bounds
   - Add debug mode that colors overflowing rects red

### Medium-term Improvement (Enforce Constraints)

1. **Formalize layout contract**
   ```rust
   pub trait Component {
       fn prepare(&mut self, bounds: Bounds, ...);

       // Component MUST NOT render outside bounds
       // Enforced by scissor rect in render pass
   }
   ```

2. **Add bounds validation layer**
   ```rust
   impl Bounds {
       pub fn validate(&self, parent: Bounds) -> Result<(), LayoutError> {
           if self.x < parent.x || self.x + self.width > parent.x + parent.width {
               return Err(LayoutError::ExceedsParent);
           }
           // ... check y bounds
           Ok(())
       }
   }
   ```

3. **Replace split-based layout with flex for dynamic regions**
   - Keep split-based for fixed chrome (status bar, sidebars)
   - Use FlexLayout for tab bar (dynamic tab count)
   - Use FlexLayout for toolbar (dynamic button sets)

### Long-term Architecture (Handle Complex Clipping)

1. **Add stencil buffer support for rounded dialogs**
   - Create depth/stencil texture: `Depth24PlusStencil8`
   - Dialog renders mask, then content with stencil test
   - Only needed if rounded clipping becomes required

2. **Implement clip stack for nested scrollable regions**
   ```rust
   struct ClipStack {
       regions: Vec<Bounds>,
   }

   impl ClipStack {
       pub fn push(&mut self, bounds: Bounds) -> Bounds {
           let parent = self.regions.last().copied().unwrap_or(Bounds::max());
           let clipped = bounds.intersect(parent);
           self.regions.push(clipped);
           clipped
       }

       pub fn pop(&mut self) { self.regions.pop(); }
   }
   ```

## Data Flow for Layout and Clipping

### Current Flow (Missing Enforcement)

```
1. WgpuApp::resumed()
   ↓
2. GpuRenderer::new() — creates components with fixed bounds assumptions
   ↓
3. WgpuApp::window_event() → state update
   ↓
4. GpuRenderer::render(model, scale_factor)
   ↓
5. calculate_layout() — compute Bounds hierarchy
   ↓ (Bounds passed but not enforced)
6. component.prepare(bounds, ...)  — build vertex data
   ↓ (No clipping check)
7. component.render(render_pass) — submit draw calls
   ↓ (GPU draws everything, even overflow)
8. present()
```

### Recommended Flow (With Enforcement)

```
1. WgpuApp::resumed()
   ↓
2. GpuRenderer::new() — components don't assume bounds
   ↓
3. WgpuApp::window_event() → state update
   ↓
4. GpuRenderer::render(model, scale_factor)
   ↓
5. calculate_layout() — compute Bounds hierarchy
   ↓ (Bounds validated in debug builds)
6. component.prepare(bounds, ...) — build vertex data, cull invisible
   ↓ (Components respect bounds or log warning)
7. render_pass.set_scissor_rect(bounds) — HARDWARE ENFORCEMENT
   ↓
8. component.render(render_pass) — submit draw calls
   ↓ (GPU clips fragments outside scissor rect)
9. present()
```

## Anti-Patterns to Avoid

### Anti-Pattern 1: Component Assumes Fixed Screen Position

**What people do:**
```rust
impl TextArea {
    pub fn new() -> Self {
        // Assume text area is always at (0, 0)
        Self { bounds: Bounds::new(0.0, 0.0, 800.0, 600.0) }
    }
}
```

**Why it's wrong:** Breaks when layout changes (sidebar toggle, window resize). Component can't be reused.

**Do this instead:**
```rust
impl TextArea {
    pub fn prepare(&mut self, bounds: Bounds, ...) {
        // Accept bounds from renderer, don't store
        self.buffer.set_size(bounds.width, bounds.height);
    }
}
```

### Anti-Pattern 2: Mixing Logical and Physical Coordinates

**What people do:**
```rust
// Bounds in logical pixels, but vertex in physical
let vertex_x = bounds.x; // Wrong! Need * scale_factor
```

**Why it's wrong:** Text/UI appears wrong size on high-DPI displays. Clipping is misaligned.

**Do this instead:**
```rust
// Always convert at boundary
let physical_x = bounds.x * scale_factor;
let physical_width = bounds.width * scale_factor;

// Or pass scale_factor explicitly
component.prepare(logical_bounds, scale_factor);
```

### Anti-Pattern 3: Ignoring Bounds in Render Logic

**What people do:**
```rust
pub fn render(&self, pass: &mut RenderPass) {
    // Draw all instances, even if outside bounds
    pass.draw(0..self.instances.len());
}
```

**Why it's wrong:** Wastes GPU time, causes visual overflow bugs, no clipping enforcement.

**Do this instead:**
```rust
pub fn render(&self, pass: &mut RenderPass) {
    if self.instances.is_empty() { return; }

    // Scissor rect already set by renderer before this call
    // Just draw — GPU handles clipping
    pass.draw(0..self.instances.len());
}
```

And in GpuRenderer:
```rust
// Set scissor before each component render
pass.set_scissor_rect(physical_x, physical_y, physical_width, physical_height);
text_area.render(&mut pass);
```

### Anti-Pattern 4: Layout Inside Component

**What people do:**
```rust
impl StatusBar {
    pub fn render(&self) {
        // Compute own layout
        let left_section = Bounds::new(0.0, 0.0, 100.0, 22.0);
        let right_section = Bounds::new(100.0, 0.0, 200.0, 22.0);
        // ...
    }
}
```

**Why it's wrong:** Duplicates layout logic, can't coordinate with other components, hard to change global layout.

**Do this instead:**
```rust
// Renderer owns layout
let status_bounds = calculate_layout(...).status_bar;
status_bar.prepare(status_bounds, ...);

// StatusBar operates within given bounds
impl StatusBar {
    pub fn prepare(&mut self, bounds: Bounds, ...) {
        // Use bounds, don't compute layout
        let (left, right) = bounds.split_horizontal(bounds.width / 2.0);
    }
}
```

## Integration Points

### glyphon TextBounds (Already Integrated)

| Aspect | Integration Pattern | Notes |
|--------|---------------------|-------|
| Coordinate space | Physical pixels (must multiply by scale_factor) | `(bounds.x * scale_factor) as i32` |
| Clipping behavior | Software culling — glyphs outside bounds not rasterized | Efficient, no GPU cost |
| Bounds validation | None — will clip to texture size if bounds exceed | Could add debug assertions |

### wgpu Scissor Rect (Missing Integration)

| Aspect | Integration Pattern | Notes |
|--------|---------------------|-------|
| Coordinate space | Physical pixels, origin top-left | Same as glyphon TextBounds |
| Clipping behavior | Hardware — fragments outside rect discarded by rasterizer | Zero cost after setup |
| Bounds validation | Must fit within render target — wgpu validates | Will panic if exceeds texture size |
| Per-component usage | Set once per render pass, before draws | Not inherited between passes |

**Recommended integration:**
```rust
impl GpuRenderer {
    fn render_component_with_clipping(
        &self,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        bounds: Bounds,
        scale_factor: f32,
        render_fn: impl FnOnce(&mut RenderPass),
    ) {
        let mut pass = encoder.begin_render_pass(...);

        // Convert logical bounds to physical scissor rect
        let scissor_x = (bounds.x * scale_factor).max(0.0) as u32;
        let scissor_y = (bounds.y * scale_factor).max(0.0) as u32;
        let scissor_w = ((bounds.width * scale_factor).min(self.config.width as f32 - scissor_x as f32)) as u32;
        let scissor_h = ((bounds.height * scale_factor).min(self.config.height as f32 - scissor_y as f32)) as u32;

        pass.set_scissor_rect(scissor_x, scissor_y, scissor_w, scissor_h);
        render_fn(&mut pass);
    }
}
```

### Split-Based Layout (Existing)

| Aspect | Pattern | Notes |
|--------|---------|-------|
| Split order | Sequential: status → sidebar → tabs → gutter → text | Dependency order matters |
| Bounds propagation | Parent Bounds → split → two child Bounds | No overlap by construction |
| Dynamic sizing | Query component for size, then split | E.g., `gutter.width(model)` |
| Visibility toggle | Return zero-size Bounds if hidden | E.g., `(Bounds::default(), main_area)` |

## Scalability Considerations

### Performance at Scale

| Element Count | Layout Overhead | Clipping Overhead | Notes |
|---------------|-----------------|-------------------|-------|
| 10-100 components | Negligible | Negligible | Single-frame layout, no batching issues |
| 100-1000 (virtualized text) | Still negligible | Glyphon handles culling | TextBounds clips efficiently |
| 1000+ UI elements | Flex layout becomes measurable | Scissor rect still zero cost | Avoid per-frame flex for static layouts |

### Clipping Priorities

**What breaks first:** Rectangle overflow (no enforcement)
**How to fix:** Add `set_scissor_rect()` to all component render passes

**What breaks next:** Stencil buffer thrashing with nested masks
**How to fix:** Limit nesting depth, use axis-aligned scissor where possible

### Scaling Path

1. **Phase 1 (Current):** Split-based layout + glyphon TextBounds + **ADD scissor rects**
   - Handles 10-100 components efficiently
   - No complex clipping needed

2. **Phase 2 (Complex UI):** Introduce FlexLayout for dynamic regions
   - Tab bar with variable tabs
   - Toolbars with dynamic button sets
   - Still < 1ms layout overhead

3. **Phase 3 (Advanced Features):** Add stencil buffer for rounded clipping
   - Dialogs with rounded corners
   - Nested scrollable regions
   - Accept batching penalty for visual quality

## Build Order Implications

### Dependency Chain

```
1. Bounds struct (components/mod.rs)
   ↓ Used by
2. Component trait + prepare(bounds) signature
   ↓ Used by
3. GpuRenderer::calculate_layout() — produces Bounds hierarchy
   ↓ Consumed by
4. Component::prepare() implementations — accept bounds, build GPU data
   ↓ Rendered by
5. RenderPass with set_scissor_rect() — enforce bounds in hardware
```

**Critical path:** Cannot enforce clipping without Bounds → cannot test component rendering → cannot fix overflow bugs.

### Testing Strategy

**Unit tests:** Bounds splitting logic (split_horizontal, split_vertical, inset)
```rust
#[test]
fn test_split_doesnt_overlap() {
    let parent = Bounds::new(0.0, 0.0, 100.0, 100.0);
    let (left, right) = parent.split_horizontal(40.0);

    assert_eq!(left.x + left.width, right.x); // Adjacent, not overlapping
    assert_eq!(left.width + right.width, parent.width); // Sum equals parent
}
```

**Integration tests:** Layout calculation produces valid hierarchy
```rust
#[test]
fn test_layout_no_overflow() {
    let renderer = setup_test_renderer();
    let layout = renderer.calculate_layout(&model, 1.0);

    // Text area must not overlap tab bar
    assert!(layout.text_area.y >= layout.tab_bar.y + layout.tab_bar.height);
}
```

**Visual tests:** Screenshot comparison to catch clipping regressions
- Capture frame with text overflowing
- Apply scissor rect fix
- Verify text is clipped, not drawn outside bounds

## Sources

**wgpu Clipping:**
- [RenderPass::set_scissor_rect documentation](https://docs.rs/wgpu/latest/wgpu/struct.RenderPass.html)
- [WebGPU setScissorRect specification](https://developer.mozilla.org/en-US/docs/Web/API/GPURenderPassEncoder/setScissorRect)
- [GitHub discussion: Rendering to specific rectangle](https://github.com/gfx-rs/wgpu/discussions/5403)

**GPU UI Clipping Patterns:**
- [Use.GPU Layout and Clipping](https://usegpu.live/docs/guides-layout-and-ui)
- [GPU UI Architecture Notes (Nicolas Silva)](https://nical.github.io/drafts/gui-gpu-notes.html)

**Stencil Buffer:**
- [Stencil Testing in WebGPU and wgpu (Max Ammann)](https://maxammann.org/posts/2022/01/wgpu-stencil-testing/)
- [Unity UI Masking Documentation](https://docs.unity3d.com/6000.3/Documentation/Manual/UIE-masking.html)
- [GameDev: Multi-level Stencil Buffers](https://www.gamedev.net/forums/topic/637523-multi-level-stencil-buffers-or-other-ways-to-cut-away-child-gui-elements/)

**glyphon:**
- [glyphon GitHub Repository](https://github.com/grovesNL/glyphon)
- [glyphon::TextBounds source](https://docs.rs/glyphon/latest/src/glyphon/text_render.rs.html)

**Layout Systems:**
- [Cassowary Algorithm (ACM Paper)](https://dl.acm.org/doi/10.1145/504704.504705)
- [Cassowary Documentation](https://cassowary.readthedocs.io/en/latest/topics/theory.html)
- [GitHub: egui immediate mode GUI](https://github.com/emilk/egui)
- [iced wgpu UI Framework](https://docs.iced.rs/iced_wgpu/index.html)

**Current Codebase:**
- `wgpu_client/src/components/mod.rs` — Bounds struct with split methods
- `wgpu_client/src/renderer/mod.rs` — calculate_layout() hierarchy
- `wgpu_client/src/design_system/layout/flex.rs` — FlexLayout implementation (unused in main layout)
- `wgpu_client/.planning/PROJECT.md` — Known overflow bugs documented

---
*Architecture research for: GPU-rendered IDE UI with wgpu*
*Researched: 2026-01-28*
