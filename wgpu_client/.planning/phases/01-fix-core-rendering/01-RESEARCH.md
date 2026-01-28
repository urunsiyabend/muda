# Phase 1: Fix Core Rendering - Research

**Researched:** 2026-01-28
**Domain:** GPU-accelerated UI rendering with wgpu and glyphon
**Confidence:** HIGH

## Summary

Phase 1 requires fixing critical rendering bugs in a GPU-accelerated IDE built with wgpu v23, winit v0.30, and glyphon v0.7. The bugs stem from three core issues: (1) shared renderer state corruption when multiple components reuse the same text renderer instances, (2) coordinate space confusion between logical and physical pixels, and (3) missing hardware clipping (scissor rectangles) allowing content to overflow component boundaries.

The codebase already has good architectural foundations with component isolation, a design system, and proper render pipeline structure. The fixes require understanding glyphon's buffer lifecycle (single prepare() call pattern), wgpu's scissor rectangle API for hardware clipping, and careful coordinate space management across the rendering pipeline.

**Primary recommendation:** Fix rendering bugs by implementing proper glyphon buffer isolation per component, adding scissor rectangle clipping to enforce layout boundaries, and auditing all coordinate transformations to ensure consistent logical-to-physical conversion.

## Standard Stack

The codebase already uses the established stack for GPU-accelerated Rust UI:

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| wgpu | 23 | Cross-platform GPU API | Industry standard for safe GPU access in Rust, supports all major backends (Vulkan/Metal/D3D12) |
| winit | 0.30 | Windowing and events | De facto standard for cross-platform window management in Rust |
| glyphon | 0.7 | Text rendering | Purpose-built for wgpu, uses cosmic-text for shaping, handles glyph atlas efficiently |
| bytemuck | 1.21 | Safe byte casting | Standard for GPU buffer data conversion (Pod/Zeroable traits) |
| pollster | 0.4 | Async runtime | Minimal async executor for wgpu initialization |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| log | 0.4 | Logging facade | Already integrated, use for error/debug output |
| simplelog | 0.12 | Log implementation | Already integrated, provides file and stderr output |
| anyhow | 1.0 | Error handling | Already integrated, use for renderer initialization and GPU errors |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| glyphon | wgpu-text, wgpu_glyph | glyphon is actively maintained and designed for modern wgpu, others are older or less maintained |
| Custom software fallback | tiny-skia | tiny-skia provides CPU rendering but requires significant integration work, defer until GPU reliability is proven |

**Installation:**
No additional crates needed. Current dependencies are correct for the task.

## Architecture Patterns

### Current Component Structure (Already Implemented)
```
GpuRenderer (orchestrator)
├── TextArea (legacy glyphon-based)
├── Gutter (legacy glyphon-based)
├── TabBar (legacy glyphon-based)
├── Sidebar (legacy glyphon-based)
├── UITextRenderer (shared, design system)
└── StyledRectRenderer (shared, design system)
```

**Problem:** Mixing legacy components (each with own glyphon instances) with shared renderers causes state corruption.

### Pattern 1: Component Isolation
**What:** Each component owns its GPU resources (buffers, pipelines, atlases)
**When to use:** For components with independent rendering state
**Example:**
```rust
// From sidebar.rs - CORRECT pattern
pub struct SidebarComponent {
    font_system: FontSystem,
    swash_cache: SwashCache,
    text_atlas: TextAtlas,
    text_renderer: TextRenderer,
    viewport: Viewport,
    buffer: Option<Buffer>,
    rect_renderer: RectRenderer,
    prepared: bool,
}
```

**Why it works:** Isolated state prevents cross-component corruption. Each component prepares independently.

### Pattern 2: Shared Renderer with Batch Collection
**What:** Centralized renderer that accepts primitives from all components in a single batch
**When to use:** For design system components that share visual style
**Example:**
```rust
// From styled_rect_renderer.rs - CORRECT pattern
impl StyledRectRenderer {
    pub fn clear(&mut self) { self.instances.clear(); }
    pub fn push_all(&mut self, rects: &[StyledRect], ...) { ... }
    pub fn prepare(&self, queue: &wgpu::Queue) { ... }
    pub fn render<'a>(&'a self, pass: &mut RenderPass<'a>) { ... }
}

// Usage in renderer.rs
self.styled_rect_renderer.clear();
self.styled_rect_renderer.push_all(&tab_rects, ...);
self.styled_rect_renderer.prepare(&self.queue);
// ... create render pass ...
self.styled_rect_renderer.render(&mut pass);
```

**Why it works:** Single prepare() call, no state interference. Primitives are stateless data.

### Pattern 3: Scissor Rectangle Clipping (MISSING - Must Add)
**What:** Hardware-accelerated clipping that discards fragments outside a rectangle
**When to use:** To enforce component boundaries and prevent overflow
**Example:**
```rust
// Source: https://docs.rs/wgpu/latest/wgpu/struct.RenderPass.html
let mut pass = encoder.begin_render_pass(&desc);
pass.set_pipeline(&pipeline);

// Set scissor rect to clip to component bounds (in physical pixels)
pass.set_scissor_rect(
    (bounds.x * scale_factor) as u32,
    (bounds.y * scale_factor) as u32,
    (bounds.width * scale_factor) as u32,
    (bounds.height * scale_factor) as u32,
);

pass.draw(...);
```

**Critical:** Scissor rectangles must be:
- In physical pixels (screen coordinates), not logical pixels
- Within render target bounds (0..screen_width, 0..screen_height)
- Set per component to prevent overflow into adjacent regions
- Validated before use to avoid wgpu panics

**Sources:**
- [RenderPass in wgpu](https://docs.rs/wgpu/latest/wgpu/struct.RenderPass.html)
- [wgpu Discussion #5403: Render to specific rectangle](https://github.com/gfx-rs/wgpu/discussions/5403)

### Pattern 4: Glyphon Buffer Lifecycle (Currently Violated)
**What:** Single prepare() call per frame per TextRenderer instance
**When to use:** Always when using glyphon
**Example:**
```rust
// WRONG - Multiple prepare calls corrupt atlas
self.ui_text_renderer.prepare(&device, &queue, &sidebar_texts, ...);
// ... render sidebar ...
self.ui_text_renderer.prepare(&device, &queue, &tab_texts, ...); // CORRUPTION!
// ... render tabs ...

// CORRECT - Collect all text first, single prepare
let mut all_texts = Vec::new();
all_texts.extend(sidebar_texts);
all_texts.extend(tab_texts);
all_texts.extend(status_texts);
self.ui_text_renderer.prepare(&device, &queue, &all_texts, ...);
// ... render everything ...
```

**Source:** [glyphon TextRenderer::prepare documentation](https://context7.com/websites/rs_glyphon)

### Anti-Patterns to Avoid
- **Multiple prepare() calls per frame:** Corrupts glyphon's glyph atlas, causes disappearing text
- **Mixing logical and physical pixels:** Causes misaligned text, incorrect hit detection
- **No scissor rectangles:** Allows content to overflow into other components
- **Shared glyphon state across components:** Causes progressive corruption as components update

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Glyph rendering | Custom font rasterizer | glyphon | Handles font shaping, kerning, ligatures, emoji, complex scripts, atlas packing, GPU upload |
| Text measurement | Manual character width calculation | glyphon's Buffer + layout_runs() | Accounts for kerning, ligatures, font fallback, correct advance widths |
| UI clipping | Fragment shader discard | wgpu scissor rectangles | Hardware-accelerated, no shader overhead, automatically bounds-checked |
| Coordinate transforms | Manual NDC conversion | wgpu viewport + scissor | GPU handles transformation, scissor works in screen space |
| Visual regression testing | Manual screenshot comparison | insta (snapshots) + pixelmatch | Handles diff visualization, threshold tuning, cross-platform pixel differences |

**Key insight:** GPU text rendering is extremely complex. Glyphon handles font shaping (bidirectional text, combining marks, emoji), glyph atlas packing (dynamic growth, eviction), and GPU upload. Custom solutions inevitably miss edge cases (emoji, ligatures, RTL text, font fallback).

## Common Pitfalls

### Pitfall 1: Multiple glyphon prepare() Calls Per Frame
**What goes wrong:** Text progressively disappears or becomes corrupted
**Why it happens:** glyphon's TextRenderer expects a single prepare() call per frame with all TextArea instances. Multiple prepare() calls overwrite the glyph vertex buffer, causing earlier text to vanish.
**How to avoid:**
- Collect all TextBlock primitives from all components
- Convert to TextArea array in one place
- Single prepare() call before rendering
- Alternative: Use isolated TextRenderer per component (more GPU memory, but simpler)
**Warning signs:**
- Sidebar text disappears after clicking
- Tab text shows then vanishes on next frame
- Text corruption increases over time

**Current violation in renderer.rs:**
```rust
// Lines 507-514: sidebar prepare
self.ui_text_renderer.prepare(&self.device, &self.queue, &tree_texts, ...);

// Lines 564-571: tab bar prepare (CORRUPTS previous prepare!)
self.ui_text_renderer.prepare(&self.device, &self.queue, &tab_texts, ...);

// Lines 631-638: status line prepare (CORRUPTS again!)
self.ui_text_renderer.prepare(&self.device, &self.queue, &status_texts, ...);

// Lines 728-735: panel prepare (CORRUPTS again!)
self.ui_text_renderer.prepare(&self.device, &self.queue, &panel_texts, ...);
```

### Pitfall 2: Coordinate Space Confusion
**What goes wrong:** Text appears at wrong position, clipping rectangles don't align with content, hit detection fails
**Why it happens:** Mixing logical pixels (UI layout) with physical pixels (GPU rendering) without consistent conversion
**How to avoid:**
- Layout calculations: always logical pixels
- GPU coordinates: always physical pixels (logical × scale_factor)
- Scissor rectangles: always physical pixels
- Document coordinate space for each function
**Warning signs:**
- Text offset from background rectangles
- Click detection misaligned
- Scissor rectangles cut off content incorrectly
- Different behavior at different DPI scales

**Example audit needed:**
```rust
// text_area.rs line 139: Uses scale_factor correctly
left: bounds.x * scale_factor,

// sidebar.rs line 125: Uses scale_factor correctly
left: (bounds.x + 8.0) * scale_factor,

// Must verify all prepare() calls use consistent pattern
```

### Pitfall 3: Missing Layout Bounds Enforcement
**What goes wrong:** Text from text area overflows into tab bar, sidebar content leaks into editor
**Why it happens:** GPU renders everything submitted without bounds checking unless scissor rectangles are set
**How to avoid:**
- Set scissor rectangle before each component's render pass
- Calculate scissor rect from component's layout bounds in physical pixels
- Validate scissor rect is within render target bounds
- Use separate render passes if components need different scissor regions
**Warning signs:**
- Text overlapping between components
- Content visible outside component bounds
- Layout looks correct but rendering ignores boundaries

**Currently missing:** No `set_scissor_rect()` calls in renderer.rs. All components render without clipping.

### Pitfall 4: Scissor Rectangle Outside Render Target
**What goes wrong:** wgpu panics with validation error, application crashes
**Why it happens:** Scissor rectangle extends beyond (0, 0, screen_width, screen_height)
**How to avoid:**
```rust
fn safe_scissor_rect(bounds: Bounds, scale_factor: f32, screen_width: u32, screen_height: u32) -> (u32, u32, u32, u32) {
    let x = (bounds.x * scale_factor).max(0.0) as u32;
    let y = (bounds.y * scale_factor).max(0.0) as u32;
    let right = ((bounds.x + bounds.width) * scale_factor).min(screen_width as f32) as u32;
    let bottom = ((bounds.y + bounds.height) * scale_factor).min(screen_height as f32) as u32;
    let width = right.saturating_sub(x);
    let height = bottom.saturating_sub(y);
    (x, y, width, height)
}
```
**Warning signs:**
- Panic: "validation error: scissor rect not contained in render target"
- Crash when resizing window
- Crash when component bounds change

**Source:** [egui Issue #2038: wgpu crash if scissor outside window](https://github.com/emilk/egui/issues/2038)

### Pitfall 5: Viewport vs Scissor Confusion
**What goes wrong:** Setting viewport when scissor is needed, or vice versa
**Why it happens:** Both control visible regions but work differently
**How to avoid:**
- Viewport: Transforms coordinates, affects vertex shader output
- Scissor: Discards fragments, works after rasterization, doesn't affect coordinates
- For UI clipping: use scissor (doesn't change coordinate system)
- For 3D: use viewport (transforms NDC to screen)
**Warning signs:**
- Setting viewport doesn't clip content
- Coordinate system unexpectedly changes when trying to clip

**Source:** [wgpu Discussion #5403: Viewport vs Scissor](https://github.com/gfx-rs/wgpu/discussions/5403)

## Code Examples

Verified patterns from official sources:

### Scissor Rectangle Setup
```rust
// Source: wgpu RenderPass documentation
// https://docs.rs/wgpu/latest/wgpu/struct.RenderPass.html

let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
    label: Some("clipped_component"),
    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
        view: &view,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Load,
            store: wgpu::StoreOp::Store,
        },
    })],
    depth_stencil_attachment: None,
    timestamp_writes: None,
    occlusion_query_set: None,
});

// Physical pixel coordinates
pass.set_scissor_rect(
    scissor_x,      // u32: left edge
    scissor_y,      // u32: top edge
    scissor_width,  // u32: width
    scissor_height  // u32: height
);

pass.set_pipeline(&pipeline);
pass.draw(...);
```

### Single glyphon prepare() Pattern
```rust
// Source: glyphon documentation
// https://context7.com/websites/rs_glyphon

// Collect all text areas first
let text_areas: Vec<glyphon::TextArea> = vec![
    glyphon::TextArea {
        buffer: &buffer1,
        left: x1 * scale,
        top: y1 * scale,
        scale,
        bounds: compute_bounds(bounds1, scale),
        default_color: color1,
        custom_glyphs: &[],
    },
    glyphon::TextArea {
        buffer: &buffer2,
        left: x2 * scale,
        top: y2 * scale,
        scale,
        bounds: compute_bounds(bounds2, scale),
        default_color: color2,
        custom_glyphs: &[],
    },
    // ... all other text areas
];

// Single prepare call
text_renderer.prepare(
    device,
    queue,
    &mut font_system,
    &mut text_atlas,
    viewport,
    text_areas,
    &mut swash_cache,
)?;

// Render in single pass
{
    let mut pass = encoder.begin_render_pass(&desc);
    text_renderer.render(&text_atlas, viewport, &mut pass)?;
}
```

### Viewport Update Pattern
```rust
// Source: glyphon Viewport documentation
// https://context7.com/websites/rs_glyphon

// Update viewport with physical screen dimensions
viewport.update(
    queue,
    glyphon::Resolution {
        width: screen_width,   // Physical pixels (u32)
        height: screen_height, // Physical pixels (u32)
    },
);

// Viewport must match the actual render target size
// Do NOT use logical pixels here
```

### Debug Overlay Pattern (Conditional Compilation)
```rust
// Pattern for debug visualization with zero release overhead

#[cfg(debug_assertions)]
mod debug {
    use super::*;

    pub fn draw_bounds_overlay(
        encoder: &mut CommandEncoder,
        view: &TextureView,
        queue: &Queue,
        bounds: Bounds,
        scale_factor: f32,
        color: Color,
    ) {
        // Draw border around component bounds
        let rects = vec![
            // Top border
            Rect::new(bounds.x, bounds.y, bounds.width, 1.0, color),
            // Right border
            Rect::new(bounds.right() - 1.0, bounds.y, 1.0, bounds.height, color),
            // Bottom border
            Rect::new(bounds.x, bounds.bottom() - 1.0, bounds.width, 1.0, color),
            // Left border
            Rect::new(bounds.x, bounds.y, 1.0, bounds.height, color),
        ];

        // Use existing rect renderer
        // ... render rects with debug color ...
    }
}

// In render function:
#[cfg(debug_assertions)]
debug::draw_bounds_overlay(&mut encoder, &view, &self.queue, layout.text_area, scale_factor, DEBUG_COLOR);
```

### State Persistence Pattern
```rust
// Using serde + TOML for session state
// Recommended: persistent_config crate

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
struct SessionState {
    window_size: (u32, u32),
    window_position: (i32, i32),
    sidebar_visible: bool,
    sidebar_width: f32,
    panel_visible: bool,
    panel_height: f32,
    open_files: Vec<PathBuf>,
    active_tab: Option<usize>,
    scroll_positions: HashMap<PathBuf, f32>,
}

impl SessionState {
    fn save(&self, path: &Path) -> anyhow::Result<()> {
        let toml = toml::to_string_pretty(self)?;
        std::fs::write(path, toml)?;
        Ok(())
    }

    fn load(path: &Path) -> anyhow::Result<Self> {
        let toml = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&toml)?)
    }

    fn config_path() -> PathBuf {
        // Platform-specific app data directory
        #[cfg(target_os = "windows")]
        let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());

        #[cfg(not(target_os = "windows"))]
        let base = std::env::var("HOME").unwrap_or_else(|_| ".".into());

        PathBuf::from(base).join("muda").join("session.toml")
    }
}
```

**Source:** [persistent_config crate documentation](https://docs.rs/persistent_config/latest/persistent_config/)

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| wgpu_glyph | glyphon | 2023 | glyphon uses cosmic-text (better shaping, modern text APIs), more actively maintained |
| Manual glyph atlas | glyphon TextAtlas | glyphon design | Automatic atlas growth, eviction, GPU upload handled internally |
| Per-glyph draw calls | Instanced quad rendering | Modern GPU APIs | Batches thousands of glyphs in single draw call |
| CPU text layout | GPU-friendly layout | wgpu era | cosmic-text produces GPU-ready runs, minimal CPU work per frame |
| Separate scissor/viewport | Unified render state | wgpu 0.12+ | set_scissor_rect() and set_viewport() are separate methods on RenderPass |

**Deprecated/outdated:**
- **wgpu_glyph:** No longer maintained, use glyphon instead
- **glyph-brush:** Older API, glyphon wraps modern equivalent (cosmic-text)
- **Manual vertex buffer management for text:** glyphon handles this internally

## Open Questions

Things that couldn't be fully resolved:

1. **Software Rendering Fallback**
   - What we know: wgpu doesn't have built-in CPU fallback; iced uses tiny-skia as fallback renderer
   - What's unclear: Whether fallback is needed for Muda's target platforms (desktop with modern GPUs)
   - Recommendation: Defer CPU fallback until GPU reliability is proven. Add graceful error handling (show dialog, save work, exit cleanly) if GPU initialization fails.

2. **Visual Regression Testing Tools**
   - What we know: No Rust-native GPU screenshot comparison tools found; JavaScript tools (Playwright, Percy) exist but require browser environment
   - What's unclear: Best approach for Rust desktop GPU apps
   - Recommendation:
     - Use `wgpu::Texture::copy_to_buffer()` to capture framebuffer to CPU
     - Use `insta` crate for snapshot testing with image comparison
     - Use `image` crate to save/load PNG reference images
     - Consider `pixelmatch-rs` for diff visualization
     - Manual verification initially, automate incrementally

3. **Debug Overlay Performance Impact**
   - What we know: `#[cfg(debug_assertions)]` strips code in release builds
   - What's unclear: Whether debug overlay draw calls impact debug build performance significantly
   - Recommendation: Implement with compile-time flag, measure frame time in debug vs release, optimize if debug performance is unacceptable

## Sources

### Primary (HIGH confidence)
- [wgpu RenderPass API](https://docs.rs/wgpu/latest/wgpu/struct.RenderPass.html) - Scissor rectangle and viewport documentation
- [glyphon TextRenderer](https://context7.com/websites/rs_glyphon) - Buffer lifecycle and prepare() patterns
- [wgpu GitHub /gfx-rs/wgpu](https://github.com/gfx-rs/wgpu) - Official repository and issue tracker
- [Learn Wgpu Tutorial](https://sotrh.github.io/learn-wgpu/) - Render pipeline and state management patterns

### Secondary (MEDIUM confidence)
- [wgpu Discussion #5403: Render to specific rectangle](https://github.com/gfx-rs/wgpu/discussions/5403) - Scissor vs viewport clarification
- [egui Issue #2038: Scissor rect crash](https://github.com/emilk/egui/issues/2038) - Bounds validation warning
- [iced wgpu + tiny-skia fallback](https://deepwiki.com/iced-rs/iced/4.1-wgpu-renderer) - Software fallback pattern
- [persistent_config crate](https://docs.rs/persistent_config/latest/persistent_config/) - Session persistence pattern

### Tertiary (LOW confidence)
- Visual regression testing tools (2026 lists) - General VRT concepts, not Rust-specific
- WebGPU error handling best practices - Applies to wgpu but focus is web platform

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - Existing codebase uses established libraries correctly
- Architecture: HIGH - Official documentation and Context7 provide clear patterns
- Pitfalls: HIGH - Current bugs match documented issues (multiple prepare(), missing scissor)
- Debug tooling: MEDIUM - Patterns exist but no Rust-native GPU screenshot tools found
- CPU fallback: MEDIUM - Pattern exists (iced + tiny-skia) but unclear if needed

**Research date:** 2026-01-28
**Valid until:** 30 days (stable wgpu/glyphon APIs, established patterns)
