# Phase 2: Layout & Rendering Pipeline - Research

**Researched:** 2026-01-28
**Domain:** Custom flexbox layout engine, GPU rectangle/text rendering, wgpu batching
**Confidence:** MEDIUM-HIGH

## Summary

Phase 2 implements a simple flexbox layout engine (row/column containers with gap, alignment, flex-grow/shrink), GPU-accelerated rectangle rendering with rounded corners and shadows, and glyphon-based text rendering. The standard approach uses:

1. **Layout**: Custom three-pass layout algorithm (top-down preparation, bottom-up measurement, top-down distribution) inspired by CSS flexbox but simplified to avoid full specification complexity
2. **Rectangle rendering**: Instanced rendering with storage buffers, shader-based rounded corners using distance fields and Gaussian blur
3. **Text rendering**: glyphon wrapper over cosmic-text for shaping, etagere for atlas packing, wgpu for rendering

**Primary recommendation:** Implement custom layout engine with three-pass constraint resolution, use instanced rendering with storage buffers for rectangles, integrate glyphon for text with measurement during layout phase.

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| wgpu | 23.x | GPU abstraction and rendering | Industry standard for cross-platform GPU access in Rust, stable API |
| glyphon | 0.9.0 | Text rendering for wgpu | Most actively maintained wgpu text renderer, wraps cosmic-text and etagere |
| cosmic-text | Latest | Text shaping and layout | Advanced text handling with complex script support, font fallback, used by glyphon |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| pollster | 0.4 | Async executor | Already in use for GPU init, continue for consistency |
| bytemuck | Latest | Safe transmutation | Recommended for buffer data casting (vertex/instance data) |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Custom layout | taffy (flexbox/grid) | Taffy is full CSS impl (~20k LOC), adds complexity. Custom is 600 LOC for needed subset. User chose simple for v1. |
| glyphon | glyph-brush | glyph-brush less actively maintained, glyphon specifically designed for wgpu integration |
| Storage buffers | Uniform buffers | Uniforms limited to 64 KiB, storage allows 128 MiB. Needed for large instance arrays. |

**Installation:**
```bash
# Already in Cargo.toml: wgpu, pollster
# Add for Phase 2:
cargo add glyphon --version 0.9
cargo add bytemuck --features derive
```

## Architecture Patterns

### Recommended Project Structure
```
ora/src/
├── layout/              # Layout constraint solving
│   ├── mod.rs          # LayoutEngine, LayoutCache
│   ├── constraints.rs  # Constraint types (AvailableSpace, Size, Rect)
│   └── flexbox.rs      # Flexbox algorithm (three-pass)
├── rendering/          # GPU rendering systems
│   ├── mod.rs          # RenderContext, batch coordination
│   ├── rectangles.rs   # Rectangle batching and shader
│   └── text.rs         # Glyphon integration wrapper
├── elements/           # Concrete element implementations
│   ├── div.rs          # Div primitive
│   └── text.rs         # Text primitive
└── style/              # Style data structures
    ├── mod.rs          # Style, Color, Border, etc.
    └── units.rs        # Length (Px, Percent), Rect<T>
```

### Pattern 1: Three-Pass Layout Algorithm

**What:** Constraint resolution using three sequential passes instead of recursion

**When to use:** Resolving flexbox layouts with percentage sizing and auto-sizing children

**Algorithm:**
1. **Pass 1 (Top-Down)**: Traverse tree, prepare queues, establish parent-child relationships
2. **Pass 2 (Bottom-Up)**: Calculate auto sizes starting from leaves (text nodes have intrinsic size)
3. **Pass 3 (Top-Down)**: Distribute space using flex values, apply alignment, resolve percentages

**Example:**
```rust
// Source: https://tchayen.com/how-to-write-a-flexbox-layout-engine

pub fn compute_layout(root: &mut LayoutNode, available_space: Size) {
    // Pass 1: Prepare (top-down traversal)
    let mut queue = VecDeque::new();
    queue.push_back(root);

    while let Some(node) = queue.pop_front() {
        for child in &node.children {
            queue.push_back(child);
        }
    }

    // Pass 2: Measure (bottom-up)
    // Start from leaves, propagate intrinsic sizes upward
    compute_intrinsic_sizes(root);

    // Pass 3: Distribute (top-down)
    // Apply flex, alignment, resolve percentages
    distribute_space(root, available_space);
}

fn compute_intrinsic_sizes(node: &mut LayoutNode) {
    // Recurse to children first (bottom-up)
    for child in &mut node.children {
        compute_intrinsic_sizes(child);
    }

    // For row: width = sum(child widths) + gaps
    // For column: height = sum(child heights) + gaps
    // Text nodes: measure via glyphon
}
```

**Key insight:** Percentage sizing requires parent dimensions already resolved. Three passes prevent infinite recursion by establishing clear dependency order.

### Pattern 2: Instanced Rectangle Rendering

**What:** Render thousands of rectangles in single draw call using instance data

**When to use:** Drawing UI elements (backgrounds, borders, shadows) where geometry is identical but position/color/style varies

**Example:**
```rust
// Source: https://sotrh.github.io/learn-wgpu/beginner/tutorial7-instancing/
// Adapted from WebGPU: https://tchayen.com/thousands-styled-rectangles-in-120fps-on-gpu

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct RectInstance {
    color: [f32; 4],           // RGBA
    position: [f32; 2],        // X, Y
    _pad: f32,
    sigma: f32,                // Blur radius for shadows/AA
    corners: [f32; 4],         // TL, TR, BR, BL radius
    size: [f32; 2],            // Width, height
    window_size: [f32; 2],     // For NDC conversion
}

// Vertex buffer layout with VertexStepMode::Instance
let instance_layout = VertexBufferLayout {
    array_stride: std::mem::size_of::<RectInstance>() as u64,
    step_mode: VertexStepMode::Instance,
    attributes: &[
        // Define location for each field (mat4 takes 4 slots)
    ],
};

// Single draw call for all rectangles
render_pass.set_vertex_buffer(1, instance_buffer.slice(..));
render_pass.draw(0..6, 0..instance_count); // 6 vertices (2 triangles), N instances
```

**Shader approach:**
- Vertex shader: Position full-screen quad per instance
- Fragment shader: Use distance fields for rounded corners, Gaussian for AA

### Pattern 3: Glyphon Text Rendering Integration

**What:** Measure text during layout, batch all text into single render pass

**When to use:** Any text rendering in UI framework

**Example:**
```rust
// Source: https://docs.rs/glyphon/latest/glyphon/
// Source: https://pop-os.github.io/cosmic-text/cosmic_text/

// During framework initialization
let mut font_system = FontSystem::new();
let mut swash_cache = SwashCache::new();
let cache = glyphon::Cache::new(&device);
let mut atlas = TextAtlas::new(&device, &queue, &cache, Format::Bgra8UnormSrgb);
let text_renderer = TextRenderer::new(
    &mut atlas,
    &device,
    MultisampleState::default(),
    None,
);

// During layout phase - MEASURE TEXT
let metrics = Metrics::new(font_size, line_height);
let mut buffer = Buffer::new(&mut font_system, metrics);
buffer.set_size(&mut font_system, Some(available_width), None);
buffer.set_text(&mut font_system, text_content, attrs, Shaping::Advanced);
buffer.shape_until_scroll(&mut font_system, true);

// Get measured dimensions
let (width, total_lines) = buffer.layout_runs()
    .fold((0.0, 0), |(max_w, lines), run| {
        (max_w.max(run.line_w), lines + 1)
    });
let height = total_lines as f32 * line_height;

// During paint phase - COLLECT text areas
let mut text_areas = Vec::new();
text_areas.push(TextArea {
    buffer: &buffer,
    left: x,
    top: y,
    scale: 1.0,
    bounds: TextBounds {
        left: x as i32,
        top: y as i32,
        right: (x + width) as i32,
        bottom: (y + height) as i32,
    },
    default_color: Color::rgba(255, 255, 255, 255),
});

// SINGLE BATCH RENDER for all text
text_renderer.prepare(
    &device,
    &queue,
    &mut font_system,
    &mut atlas,
    &Resolution { width: window_width, height: window_height },
    text_areas,
    &mut swash_cache,
)?;

// Within render pass
text_renderer.render(&atlas, &viewport, &mut render_pass)?;
```

**Integration pattern:** Framework owns FontSystem/TextAtlas/SwashCache globally, views/elements reference them via context during layout/paint.

### Pattern 4: Nested Scissor Clipping

**What:** Stack scissor rectangles to clip children to parent bounds

**When to use:** overflow: hidden, scrollable containers, preventing child rendering outside parent

**Example:**
```rust
// Source: https://docs.rs/wgpu/latest/wgpu/struct.RenderPass.html

fn paint_with_clipping(
    &self,
    cx: &mut PaintContext,
    render_pass: &mut RenderPass,
    parent_clip: Option<Rect>,
) {
    // Compute element's clip rect
    let my_clip = Rect::new(self.bounds.x, self.bounds.y, self.bounds.width, self.bounds.height);

    // Intersect with parent clip (nested clipping)
    let clip_rect = if let Some(parent) = parent_clip {
        my_clip.intersect(parent)
    } else {
        my_clip
    };

    // Apply scissor only if overflow: hidden
    if self.style.overflow == Overflow::Hidden {
        render_pass.set_scissor_rect(
            clip_rect.x as u32,
            clip_rect.y as u32,
            clip_rect.width as u32,
            clip_rect.height as u32,
        );
    }

    // Paint self
    self.paint_background(cx);

    // Paint children with propagated clip
    for child in &self.children {
        child.paint_with_clipping(cx, render_pass, Some(clip_rect));
    }

    // Restore scissor if modified (or track state externally)
}
```

**Critical:** set_scissor_rect does NOT stack/nest automatically. Framework must manually track clip stack and compute intersections.

### Anti-Patterns to Avoid

- **One pipeline per rectangle style:** Don't create unique render pipelines for different border radius/colors. Use one pipeline + instance data. (Source: [GitHub wgpu discussions](https://github.com/gfx-rs/wgpu-rs/issues/18))

- **Recursive layout with percentages:** Don't implement layout as pure recursion. Parent dimensions must be resolved before percentage children can compute. Use three-pass algorithm. (Source: [tchayen flexbox tutorial](https://tchayen.com/how-to-write-a-flexbox-layout-engine))

- **Per-frame vertex buffer allocation:** Don't allocate/free vertex buffers every frame. Reuse storage buffers with dynamic writes. (Source: [WebGPU Buffer Uploads](https://toji.dev/webgpu-best-practices/buffer-uploads.html))

- **Missing layout cache invalidation:** Don't recalculate layout every frame if nothing changed. Use dirty flags to skip layout when view state unchanged. (Source: [Dirty Flag Pattern](https://gameprogrammingpatterns.com/dirty-flag.html))

- **Text measurement without shaping:** Don't estimate text bounds. Always use cosmic-text shaping for accurate dimensions, especially for complex scripts. (Source: [cosmic-text docs](https://pop-os.github.io/cosmic-text/cosmic_text/))

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Text shaping | Custom glyph positioning | cosmic-text (via glyphon) | Complex scripts (Arabic, Thai), bidirectional text, ligatures, font fallback. Thousands of edge cases. |
| Glyph atlas packing | Custom bin packing | etagere (via glyphon) | Optimal packing is NP-hard, cache eviction policies, dynamic growth. Production-tested. |
| Rounded corner rendering | Geometry tessellation | Shader distance fields | Generating smooth geometry is complex, shader is 20 lines and handles all radii/AA. (Source: [Warp blog](https://www.warp.dev/blog/how-to-draw-styled-rectangles-using-the-gpu-and-metal)) |
| Gaussian blur for shadows | Multi-pass blur | Closed-form 1D solution + sampling | "Fast Rounded Rectangle Shadows" provides closed-form error function approach, 4 samples vs 100+. (Source: [Evan Wallace shader](https://madebyevan.com/shaders/fast-rounded-rectangle-shadows/)) |
| Font fallback system | Manual fallback lists | cosmic-text FontSystem | Reuses Chromium/Firefox static fallback lists, locale-aware, platform differences. (Source: [cosmic-text GitHub](https://github.com/pop-os/cosmic-text)) |

**Key insight:** UI rendering is deceptively complex. Existing libraries handle edge cases tested across millions of users. Custom implementations introduce subtle bugs (text measurement off by 1px, clip rect rounding errors).

## Common Pitfalls

### Pitfall 1: Percentage Sizing Without Parent Constraint

**What goes wrong:** Child specifies `width: 50%` but parent has auto width. Layout solver cannot resolve constraint, produces zero/infinite width.

**Why it happens:** Percentage sizing creates circular dependency: parent size depends on children, but child size depends on parent. Spec says percentage resolves to auto if parent indefinite.

**How to avoid:**
- Three-pass algorithm: Pass 2 computes auto sizes bottom-up (parent gets content width), Pass 3 resolves percentages top-down
- If parent still indefinite after Pass 2, percentage children fall back to auto or min-content

**Warning signs:** Elements with percentage sizing rendering as zero width/height.

**Source:** [CSS flexbox spec discussions](https://github.com/w3c/csswg-drafts/issues/6822)

### Pitfall 2: Missing WGSL Alignment Padding

**What goes wrong:** Instance data in Rust doesn't match shader struct layout. GPU reads garbage data, rendering incorrect.

**Why it happens:** WGSL has strict alignment rules: vec3 aligns to 16 bytes, struct members align to their size. Rust's #[repr(C)] is not sufficient.

**How to avoid:**
- Use bytemuck::Pod + bytemuck::Zeroable traits for compile-time checks
- Manually add padding fields where needed (e.g., after `vec2` before next field)
- Test: Print buffer bytes in Rust, compare to shader expectations

**Warning signs:** Rendering works on some GPUs (different alignment rules), fails on others. Shifted/corrupted colors or positions.

**Source:** [Learn wgpu alignment](https://sotrh.github.io/learn-wgpu/showcase/alignment/)

### Pitfall 3: Scissor Rect Integer Truncation

**What goes wrong:** Layout computed in floats (32.7, 100.5), converted to u32 for set_scissor_rect. Repeated truncation accumulates error, clips 1-2px too early.

**Why it happens:** set_scissor_rect takes u32 parameters. Direct truncation (as u32) rounds toward zero, consistent bias.

**How to avoid:**
- Round to nearest integer: `(x + 0.5) as u32`
- Track clip stack in floats, only convert at GPU API boundary
- Test: Nested containers with borders should not show sub-pixel gaps

**Warning signs:** 1px gaps around clipped edges, especially in deeply nested layouts.

**Source:** [wgpu RenderPass docs](https://docs.rs/wgpu/latest/wgpu/struct.RenderPass.html)

### Pitfall 4: Text Measurement Phase Mismatch

**What goes wrong:** Text measured during layout uses different font/size than paint phase. Measured bounds don't match rendered bounds, causing overflow or clipping.

**Why it happens:** Creating separate Buffer instances with different Metrics, or measuring before font is loaded.

**How to avoid:**
- Single source of truth: Store Buffer in element's RequestLayoutState
- Measure during layout: `buffer.set_text()` then `buffer.shape_until_scroll()`
- Paint uses same Buffer, same FontSystem
- Ensure fonts loaded before first layout

**Warning signs:** Text visually clipped despite container sized to measured bounds. Text wrapping differently than layout expected.

**Source:** [glyphon GitHub issues](https://github.com/grovesNL/glyphon/issues/117)

### Pitfall 5: Storage Buffer Size Limits

**What goes wrong:** Batching 10,000 rectangles into storage buffer exceeds max buffer size (128 MiB default), pipeline creation fails.

**Why it happens:** Each instance is ~64 bytes (RectInstance struct). 10k instances = 640 KiB, usually fine. But with additional data (textures, shadows) can exceed limits.

**How to avoid:**
- Check device limits: `device.limits().max_storage_buffer_binding_size`
- Batch in chunks if needed: Multiple draws with different buffer ranges
- Use uniform buffers for per-frame globals, storage only for instance arrays
- Test with realistic instance counts (1000+ rectangles for editor UI)

**Warning signs:** Pipeline creation panics, validation errors about buffer size.

**Source:** [wgpu Limits docs](https://docs.rs/wgpu/latest/wgpu/struct.Limits.html), [WebGPU storage buffers](https://webgpufundamentals.org/webgpu/lessons/webgpu-storage-buffers.html)

### Pitfall 6: Forgetting Flex-Shrink Defaults

**What goes wrong:** Container with flex children overflows instead of shrinking to fit available space.

**Why it happens:** CSS flexbox defaults flex-shrink to 1, but custom impl might default to 0. Children refuse to shrink below content size.

**How to avoid:**
- Match CSS defaults: flex-grow: 0, flex-shrink: 1, flex-basis: auto
- Document differences from CSS if intentional
- Test: Container with constrained width + oversized children should shrink equally

**Warning signs:** Horizontal scrollbars when content should fit, items not shrinking despite available flex-shrink.

**Source:** [MDN flex-basis](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/flex-basis)

## Code Examples

Verified patterns from official sources:

### Rounded Rectangle Shader (Fragment)

```wgsl
// Source: https://madebyevan.com/shaders/fast-rounded-rectangle-shadows/
// Adapted from: https://tchayen.com/thousands-styled-rectangles-in-120fps-on-gpu

fn gaussian(x: f32, sigma: f32) -> f32 {
    let pi = 3.141592653589793;
    return exp(-(x * x) / (2.0 * sigma * sigma)) / (sqrt(2.0 * pi) * sigma);
}

fn erf_approx(x: vec4<f32>) -> vec4<f32> {
    let s = sign(x);
    let a = abs(x);
    var x_val = 1.0 + (0.278393 + (0.230389 + 0.078108 * (a * a)) * a) * a;
    x_val = x_val * x_val;
    return s - s / (x_val * x_val);
}

// For axis-aligned edges (closed-form solution)
fn rounded_box_shadow_x(x: f32, y: f32, sigma: f32, corner: f32, half_size: vec2<f32>) -> f32 {
    let delta = min(half_size.y - corner - abs(y), 0.0);
    let curved = half_size.x - corner + sqrt(max(0.0, corner * corner - delta * delta));
    let integral = 0.5 + 0.5 * erf_approx(vec2<f32>(x - curved, x + curved) * (sqrt(0.5) / sigma));
    return integral.y - integral.x;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // instance.corners = [top_left, top_right, bottom_right, bottom_left]
    // Determine which corner applies based on pixel position
    let corner_radius = select_corner(in.local_pos, instance.corners);

    // Distance to rounded rectangle edge
    let dist = rounded_rect_sdf(in.local_pos, instance.size, corner_radius);

    // Apply Gaussian for smooth anti-aliasing
    let alpha = 1.0 - smoothstep(-instance.sigma, instance.sigma, dist);

    return vec4<f32>(instance.color.rgb, instance.color.a * alpha);
}
```

### Linear Gradient Shader

```wgsl
// Source: https://godotshaders.com/shader/linear-gradient/
// Adapted for wgpu/WGSL

struct GradientInstance {
    start_color: vec4<f32>,
    end_color: vec4<f32>,
    angle_radians: f32,  // 0 = horizontal, π/2 = vertical
    // ... other rect properties
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Rotate UV by gradient angle
    let uv = in.local_pos / instance.size;  // Normalize to 0..1
    let rotated = uv.x * cos(instance.angle_radians) - uv.y * sin(instance.angle_radians);

    // Clamp to 0..1 range for gradient interpolation
    let t = clamp(rotated, 0.0, 1.0);

    let color = mix(instance.start_color, instance.end_color, t);

    return color;
}
```

### Per-Side Border Width

```rust
// Source: https://www.warp.dev/blog/how-to-draw-styled-rectangles-using-the-gpu-and-metal
// Adapted for Rust/wgpu

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct BorderWidths {
    top: f32,
    right: f32,
    bottom: f32,
    left: f32,
}

// In fragment shader (WGSL):
// fn border_sdf(pos: vec2<f32>, size: vec2<f32>, borders: BorderWidths) -> f32 {
//     // Determine quadrant to select correct border width
//     let half = size / 2.0;
//     let border_width = select(
//         select(borders.bottom, borders.top, pos.y > 0.0),
//         select(borders.left, borders.right, pos.x > 0.0),
//         // ... distance field logic
//     );
// }
```

### Layout Cache with Dirty Flags

```rust
// Source: https://gameprogrammingpatterns.com/dirty-flag.html

pub struct LayoutCache {
    computed_bounds: Rect,
    dirty: bool,
}

impl LayoutCache {
    pub fn invalidate(&mut self) {
        self.dirty = true;
    }

    pub fn compute_if_needed(
        &mut self,
        element: &dyn Element,
        available_space: AvailableSpace,
    ) -> Rect {
        if self.dirty {
            self.computed_bounds = element.compute_layout(available_space);
            self.dirty = false;
        }
        self.computed_bounds
    }
}

// In element implementation:
impl Div {
    pub fn set_width(&mut self, width: Length) {
        if self.style.width != width {
            self.style.width = width;
            self.layout_cache.invalidate(); // Mark dirty on style change
        }
    }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| CPU text rasterization | GPU glyph atlas caching (glyphon) | 2021-2023 | 10-100x faster text rendering, enables real-time updates |
| Geometry-based rounded corners | Shader distance fields | 2020-2022 | No tessellation cost, perfect AA at any scale, dynamic radius |
| Uniform buffers for instances | Storage buffers | WebGPU spec 2021 | 64 KiB → 128 MiB limit, enables batching 1000+ instances |
| Recursive flexbox layout | Three-pass constraint solving | ~2018 (Stretch) | Predictable performance, avoids stack overflow on deep trees |
| Multi-pass Gaussian blur | Closed-form error function | 2015 (Evan Wallace) | Single-pass shadow rendering, 4 samples instead of 100+ |

**Deprecated/outdated:**
- **glyph-brush**: Superseded by glyphon for wgpu. Glyphon is actively maintained, glyph-brush maintenance stalled.
- **Stretch layout engine**: Unmaintained since 2020, forked as taffy. Don't use Stretch, use taffy or custom.
- **Uniform buffers for large instance arrays**: Still works but storage buffers are modern standard for GPU batching.

## Open Questions

Things that couldn't be fully resolved:

1. **Optimal instance buffer size**
   - What we know: Storage buffers support 128 MiB default, each instance ~64 bytes
   - What's unclear: Performance cliffs for buffer updates (map_async vs write_buffer vs staging)
   - Recommendation: Start with single buffer for all instances, benchmark >1000 rects. Switch to batching if update latency >2ms.

2. **Font embedding strategy**
   - What we know: cosmic-text supports runtime font loading, binary embedding via include_bytes!
   - What's unclear: Which default font to embed (size vs coverage tradeoff), fallback policy
   - Recommendation: Embed Iosevka (code editor optimized, ~200KB), use system fonts as fallback via FontSystem

3. **Scroll momentum and input handling**
   - What we know: winit provides mouse wheel events, glyphon supports TextArea scrolling
   - What's unclear: How to implement smooth scroll momentum (easing function, duration)
   - Recommendation: Phase 2 implements basic jump scrolling, defer momentum to Phase 9 (Transitions). Use linear scroll offset in Phase 2.

4. **Multi-line text wrapping with styled runs**
   - What we know: glyphon supports styled runs via AttrsList, cosmic-text handles wrapping
   - What's unclear: How to specify run boundaries in Text::new() API (ranges? spans?)
   - Recommendation: Phase 2 supports single-style text, defer styled runs API to Phase 6 (Design System). Prove wrapping works first.

## Sources

### Primary (HIGH confidence)
- [Learn wgpu - Instancing Tutorial](https://sotrh.github.io/learn-wgpu/beginner/tutorial7-instancing/) - Official wgpu tutorial, verified patterns
- [glyphon docs.rs](https://docs.rs/glyphon) - API reference, version 0.9.0
- [cosmic-text docs](https://pop-os.github.io/cosmic-text/cosmic_text/) - Text shaping/measurement API
- [wgpu RenderPass docs](https://docs.rs/wgpu/latest/wgpu/struct.RenderPass.html) - Scissor rect and viewport API
- [tchayen: How to Write a Flexbox Layout Engine](https://tchayen.com/how-to-write-a-flexbox-layout-engine) - Three-pass algorithm explanation
- [Evan Wallace: Fast Rounded Rectangle Shadows](https://madebyevan.com/shaders/fast-rounded-rectangle-shadows/) - Shader math for shadows
- [tchayen: Thousands of Styled Rectangles in 120FPS](https://tchayen.com/thousands-styled-rectangles-in-120fps-on-gpu) - Instance data structure, batching strategy

### Secondary (MEDIUM confidence)
- [Warp blog: Styled Rectangles with Metal](https://www.warp.dev/blog/how-to-draw-styled-rectangles-using-the-gpu-and-metal) - Per-side borders, distance fields (Metal but concepts transfer)
- [WebGPU Fundamentals: Storage Buffers](https://webgpufundamentals.org/webgpu/lessons/webgpu-storage-buffers.html) - Uniform vs storage buffer tradeoffs
- [Toji.dev: WebGPU Buffer Uploads](https://toji.dev/webgpu-best-practices/buffer-uploads.html) - Buffer update best practices
- [Game Programming Patterns: Dirty Flag](https://gameprogrammingpatterns.com/dirty-flag.html) - Layout cache invalidation pattern
- [MDN: CSS flexbox](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/flex-basis) - Spec reference for flex-grow/shrink defaults
- [cosmic-text GitHub](https://github.com/pop-os/cosmic-text) - Font fallback implementation details

### Tertiary (LOW confidence)
- WebSearch results for "wgpu render pipeline best practices 2026" - General discussions, not specific guidance
- WebSearch results for "layout cache invalidation" - Pattern confirmed via Game Programming Patterns book

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - glyphon/wgpu actively used in production (iced, egui), versions verified on docs.rs
- Architecture: MEDIUM-HIGH - Three-pass layout verified via tutorial, instancing verified via Learn wgpu, but integration patterns are custom
- Pitfalls: MEDIUM - Alignment/scissor issues documented in official sources, percentage sizing from CSS spec discussions, others inferred from common bugs

**Research date:** 2026-01-28
**Valid until:** ~2026-03-28 (60 days for stable ecosystem, glyphon updates infrequent)

**Notes:**
- Context7 not available, relied on docs.rs and official tutorials
- No existing ora Phase 1 RESEARCH.md found, reviewed Phase 1 PLAN files for architectural context
- CONTEXT.md locked decisions: simple layout (no taffy), glyphon for text, scissor clipping, per-corner radius
- Research focused on Claude's discretion areas: batching strategy, shader implementation, glyphon integration patterns
