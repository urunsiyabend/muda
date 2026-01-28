# Pitfalls Research

**Domain:** GPU-rendered IDE UI with wgpu
**Researched:** 2026-01-28
**Confidence:** HIGH

## Critical Pitfalls

### Pitfall 1: Shared Renderer State Corruption (Multiple prepare() Calls)

**What goes wrong:**
When using a single `StyledRectRenderer` or `UITextRenderer` instance across multiple UI components, calling `clear()` and `prepare()` multiple times in the same frame causes previous components' geometry to disappear. Only the last component's data survives to render.

**Why it happens:**
The renderer's internal instance buffer is cleared at the start of each `prepare()` call. In the current codebase, `styled_rect_renderer.clear()` is called 5 separate times in `renderer/mod.rs` (lines 485, 541, 609, 705, 802) for different UI regions (sidebar, tabs, status, panels, command palette). Each `clear()` wipes out the previous component's queued instances.

**How to avoid:**
- **Pattern 1 (Current codebase):** Each UI component builds its primitives independently, but accumulate ALL primitives before clearing/preparing the shared renderer
- **Pattern 2:** Use separate renderer instances per logical UI layer (one for chrome, one for editor, one for overlays)
- **Pattern 3:** Render each component immediately after prepare in its own render pass (current approach - works but verbose)

**Warning signs:**
- UI elements disappear when clicking/interacting (triggers rerender of one component)
- Elements appear in wrong z-order
- Only the last-rendered component is visible
- Progressive disappearance as more components render

**Phase to address:**
Phase 1 (Rendering Architecture) - must establish clear renderer lifecycle and ownership patterns before building more complex UI

**Root cause of current bug:**
The sidebar icons/text disappearing on click is caused by this pattern. When the sidebar is interacted with, it triggers a re-render that clears the shared `styled_rect_renderer` state, wiping out previously queued tab bar geometry if the render passes aren't ordered correctly.

---

### Pitfall 2: Glyphon Buffer Reuse Without Proper Clearing

**What goes wrong:**
`glyphon::TextRenderer::prepare()` clears its internal `glyph_vertices` vector at the start of each call. Multiple `prepare()` calls between renders cause only the last call's text to appear. Text from earlier calls vanishes.

**Why it happens:**
Glyphon's design assumes you pass ALL text areas in a single `prepare()` call per frame. The library clears previous frame data automatically. Developers often misunderstand this and prepare text incrementally, component by component.

**How to avoid:**
- Collect all `TextArea` instances in a Vec, then call `prepare()` once per frame with all areas
- Use separate `TextRenderer` instances for logically distinct rendering layers
- The `UITextRenderer` wrapper (current codebase) helps by batching text blocks, but must be used correctly

**Warning signs:**
- Text disappears when new text is added elsewhere
- Only the most recently prepared text renders
- Intermittent text visibility based on render order

**Phase to address:**
Phase 1 (Rendering Architecture) - establish text batching patterns early

**Root cause of current bug:**
Tab bar file names not being visible is likely caused by multiple `ui_text_renderer.prepare()` calls overwriting each other. The tab bar text preparation happens after sidebar text, potentially clearing the glyphon buffers if not properly batched.

---

### Pitfall 3: Viewport/Bounds Coordinate Space Confusion

**What goes wrong:**
Mixing logical (DPI-independent) and physical (pixel) coordinates causes text and geometry to overflow outside intended bounds, clip incorrectly, or render at wrong positions. Text overflows into wrong UI regions.

**Why it happens:**
- wgpu/GPU expects physical pixels for vertices and viewports
- UI layout is typically in logical pixels (DPI-independent)
- glyphon's `TextArea` takes physical coordinates but `TextBounds` is in i32 pixels
- `StyledRect` uses logical coordinates that must be converted before GPU upload
- Scale factor conversions happen at multiple layers, easy to miss or double-apply

**How to avoid:**
- Establish a single source of truth: UI layout in logical coords, convert to physical at GPU boundary
- Create clear conversion functions: `to_physical()`, `to_logical()`
- Document coordinate space for every struct field
- In current codebase: `StyledRectInstance::to_instance()` does logical→NDC conversion, `UITextRenderer::prepare()` handles logical→physical

**Warning signs:**
- Text or geometry position shifts with DPI changes
- Content overflows container bounds
- Clipping doesn't work as expected
- Layout breaks on high-DPI displays

**Phase to address:**
Phase 1 (Rendering Architecture) - coordinate space conventions must be established from the start

**Root cause of current bug:**
Text content overflowing into tab bar region is caused by incorrect bounds calculation or coordinate space mismatch. The `TextBounds` clipping in glyphon expects physical pixels, but if logical coordinates are passed, clipping fails and text renders outside intended region.

---

### Pitfall 4: Slow Startup From Synchronous Pipeline Creation

**What goes wrong:**
Creating render pipelines (`device.create_render_pipeline()`) and compute pipelines during initialization blocks the main thread for 500ms-2000ms, causing unacceptable startup delay. Users see a frozen window or loading spinner.

**Why it happens:**
Each pipeline creation compiles shaders from WGSL to device-specific microcode. This is CPU and GPU intensive. In the current codebase, 5+ pipelines are created synchronously in component constructors (`StyledRectRenderer::new`, `UITextRenderer::new`, various component renderers).

**How to avoid:**
- Use async pipeline creation: `device.create_render_pipeline_async()` (wgpu 0.20+)
- Show a loading screen/splash screen during initialization
- Cache pipelines using `PipelineCache` (helps on subsequent launches, especially on Android)
- Lazy-initialize pipelines only when components are first used
- Pre-compile shaders and embed compiled SPV/MSL/DXBC blobs

**Warning signs:**
- Window appears but is unresponsive for seconds
- "Not responding" messages from OS
- Profiling shows majority of time in `create_render_pipeline`
- Startup time grows linearly with component count

**Phase to address:**
Phase 2 (Startup Optimization) - after core rendering works, optimize initialization path

**Root cause of current bug:**
Slow initial startup is caused by synchronous pipeline creation for all components upfront. The `GpuRenderer::new()` constructor creates 12+ components, each creating their own pipeline synchronously.

---

### Pitfall 5: Buffer Reallocation Thrashing on Dynamic Content

**What goes wrong:**
Dynamically sized content (file trees, lists, text) causes vertex/instance buffers to be destroyed and recreated every frame as content changes size. This causes stuttering, frame drops, and memory fragmentation.

**Why it happens:**
The pattern `if buffer_size < needed_size { destroy_buffer(); create_new_buffer(larger_size) }` is common but expensive. GPU command queue must drain before buffer destruction is safe. Creating buffers requires GPU memory allocation and synchronization.

**How to avoid:**
- **Over-allocate strategy:** Pre-allocate buffers larger than current need (e.g., 2x, or max expected size)
- **Growth factor:** When resizing, grow by 1.5x-2x instead of exact fit
- **Pooling:** Reuse buffers across frames, only grow never shrink
- **Ring buffers:** Cycle through multiple buffers for dynamic data (similar to frame-in-flight buffering)

Current codebase uses MAX_INSTANCES (2048 for styled rects, 256 for text) which is good, but no dynamic growth strategy.

**Warning signs:**
- Frame time spikes when content length changes
- Memory usage grows over time but never shrinks
- Profiling shows time in `create_buffer` or `destroy_buffer`
- Stuttering when scrolling or adding UI elements

**Phase to address:**
Phase 3 (Performance) - after basic rendering is stable

---

### Pitfall 6: Render Pass Fragmentation (Too Many Passes)

**What goes wrong:**
Creating a separate render pass for each UI component causes GPU overhead from render target switching, pipeline state changes, and validation. Performance degrades with component count.

**Why it happens:**
Render passes provide clear isolation, making code simpler to reason about. Developers create one pass per component for organizational clarity. Current codebase has 8+ render passes per frame (clear, sidebar rects, sidebar text, tab bar rects, tab bar text, status rects, status text, etc.).

**How to avoid:**
- Batch compatible geometry into single render pass
- Use pipeline changes instead of pass changes for different shaders
- Group by render target and blend mode, not by component
- Use RenderBundles for repeated geometry (not yet in codebase)

**Warning signs:**
- Profiling shows many `begin_render_pass` calls
- GPU utilization is low despite many draw calls
- Adding new UI components causes linear performance decrease
- Validation layers show frequent "redundant state changes"

**Phase to address:**
Phase 3 (Performance) - optimize after correctness is established

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Synchronous pipeline creation | Simple code, no async complexity | Slow startup (500ms-2s) | MVP only, must fix before release |
| Single global renderer instance | Less memory, simpler lifetime management | Requires complex clear/prepare choreography, brittle | Never - use separate instances per layer |
| Exact-fit buffer sizing | Minimal memory usage | Reallocation thrashing, stuttering | Only for truly static content |
| One render pass per component | Clean separation, easier debugging | GPU overhead, poor performance | Early development, refactor before optimization phase |
| Physical coordinates in UI code | Matches GPU expectations | Breaks on DPI change, hard to maintain | Never - always use logical coords in UI layer |
| Hardcoded buffer limits (MAX_INSTANCES) | Simple, predictable memory usage | Silent failures when limit exceeded | Acceptable if limits are monitored and generous |

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| glyphon + wgpu | Calling `prepare()` multiple times per frame | Collect all TextAreas, call `prepare()` once |
| glyphon viewport | Using logical coordinates for viewport | Viewport must be physical resolution (width × scale, height × scale) |
| winit + wgpu surface | Not handling `SurfaceError::Lost` / `::Outdated` | Reconfigure surface on these errors |
| wgpu buffer updates | Using `mapped_at_creation` for per-frame data | Use `write_buffer()` for dynamic data, `mapped_at_creation` only for initialization |
| Multiple renderers | Sharing device/queue but separate atlases | Each TextRenderer needs its own TextAtlas, they don't share glyph cache |
| Scale factor | Applying scale factor in UI layout | Apply scale factor only at GPU boundary, not in layout |

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Buffer reallocation per frame | Frame time spikes, stuttering | Pre-allocate 2x expected size, grow with 1.5x factor | Any dynamic content (lists, trees) |
| No pipeline caching | Slow startup every launch | Use `PipelineCache`, cache to disk between runs | Noticeable on Android, 500ms+ desktop |
| Synchronous pipeline creation | UI freeze at startup | Use `create_render_pipeline_async()` | 3+ pipelines (immediate) |
| Too many render passes | Low GPU utilization | Batch geometry, <10 passes per frame | 15+ render passes per frame |
| Unbatched text rendering | CPU bottleneck in prepare | Batch text into single prepare() call | 50+ text areas |
| No culling/clipping | Wasted GPU time | Cull off-screen elements before building geometry | 500+ UI elements |

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Unbounded buffer allocation | OOM crash from malicious input | Cap max buffer size, validate input lengths |
| No shader compilation timeout | DoS from infinite compilation | Set timeout on pipeline creation |
| Exposing GPU device loss as crash | Application crash from external GPU reset | Handle `DeviceError::Lost` gracefully, recreate device |

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| No loading indicator during startup | Users think app is frozen | Show splash screen or progress bar |
| Instant full-screen window | Jarring, covers other windows | Fade in or start slightly smaller than screen |
| No visual feedback on GPU errors | Silent failures, blank screen | Show error overlay or fallback to software rendering |
| Unchanging cursor during long operations | Users uncertain if app is working | Change cursor to busy/wait cursor |

## "Looks Done But Isn't" Checklist

- [ ] **Text rendering:** Text appears but: clipping doesn't work, overflows bounds on long strings, breaks on font changes
- [ ] **Interaction states:** Hover works but: doesn't reset on mouse leave, remains stuck after click
- [ ] **Window resize:** UI resizes but: coordinate transforms break, text becomes blurry, buffers aren't recreated
- [ ] **DPI changes:** Works at 1x scale but: breaks at 1.5x/2x/2.67x, text clips incorrectly
- [ ] **Multiple instances:** Single component works but: shared state corrupts when multiple instances render
- [ ] **Long content:** Short strings work but: performance degrades with 1000+ list items, no virtualization

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Shared renderer state corruption | LOW | Refactor to batch all geometry before `clear()`, or use separate renderer instances |
| Glyphon buffer clearing | LOW | Collect all TextAreas in Vec, call `prepare()` once with full Vec |
| Coordinate space confusion | MEDIUM | Audit all coordinate transforms, add explicit `to_physical()`/`to_logical()` calls, add asserts |
| Slow startup | MEDIUM | Profile with `cargo flamegraph`, identify pipeline creation hotspots, make async or cache |
| Buffer reallocation thrashing | LOW | Add over-allocation factor (2x), implement growth strategy |
| Render pass fragmentation | HIGH | Major refactor to batch geometry by render target/pipeline instead of component |

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Shared renderer state corruption | Phase 1: Rendering Architecture | All UI components visible simultaneously, no disappearing elements on interaction |
| Glyphon buffer clearing | Phase 1: Rendering Architecture | All text renders correctly, no missing labels |
| Coordinate space confusion | Phase 1: Rendering Architecture | UI works correctly at 1x, 1.5x, 2x scale; no overflow |
| Slow startup | Phase 2: Startup Optimization | App becomes interactive <300ms on desktop, <1s on mobile |
| Buffer reallocation thrashing | Phase 3: Performance | 60fps maintained with dynamic content (scrolling, expanding trees) |
| Render pass fragmentation | Phase 3: Performance | <10 render passes per frame, GPU utilization >60% |

## Sources

### wgpu General Issues
- [The Surface \| Learn Wgpu](https://sotrh.github.io/learn-wgpu/beginner/tutorial2-surface/)
- [Lifetimes, surface creation, and multi-threaded rendering · wgpu Discussion #6284](https://github.com/gfx-rs/wgpu/discussions/6284)
- [Major performance problems with multithreading · wgpu Discussion #5525](https://github.com/gfx-rs/wgpu/discussions/5525)
- [Slow Render (I must be doing something wrong) · wgpu Issue #702](https://github.com/gfx-rs/wgpu-rs/issues/702)

### glyphon Text Rendering
- [GitHub - grovesNL/glyphon](https://github.com/grovesNL/glyphon)
- [glyphon text_render.rs source](https://docs.rs/glyphon/latest/src/glyphon/text_render.rs.html)
- [Broken text rendering on wasm32 target · wgpu_glyph Issue #78](https://github.com/hecrj/wgpu_glyph/issues/78)
- [Warp: Adventures in Text Rendering: Kerning and Glyph Atlases](https://www.warp.dev/blog/adventures-text-rendering-kerning-glyph-atlases)

### Buffer Management
- [Buffers and Indices \| Learn Wgpu](https://sotrh.github.io/learn-wgpu/beginner/tutorial4-buffer/)
- [Memory Leak with short lived Buffers · wgpu Issue #5397](https://github.com/gfx-rs/wgpu/issues/5397)
- [How to properly write to a buffer every frame? · wgpu Discussion #1438](https://github.com/gfx-rs/wgpu/discussions/1438)
- [wgpu_memory - Abstraction for GPU memory allocation](https://github.com/Hugo4IT/wgpu_memory)

### Startup Performance
- [Why is the initialization so slow? · wgpu Issue #6155](https://github.com/gfx-rs/wgpu/issues/6155)
- [Long startup time · wgpu Issue #3332](https://github.com/gfx-rs/wgpu/issues/3332)
- [Pipeline Caching (between program executions) · wgpu Issue #5293](https://github.com/gfx-rs/wgpu/issues/5293)
- [PipelineCache in wgpu](https://docs.rs/wgpu/latest/wgpu/struct.PipelineCache.html)

### Render Pass Management
- [RenderPass in wgpu](https://docs.rs/wgpu/latest/wgpu/struct.RenderPass.html)
- [Struggling with render pass lifetime · wgpu Discussion #3832](https://github.com/gfx-rs/wgpu/discussions/3832)
- [GPU Web 2026-01-07 Meeting Notes](https://github.com/gpuweb/gpuweb/wiki/GPU-Web-2026%E2%80%9001%E2%80%9007)
- [WebGPU Rendering: Part 6 Multiple Render Passes](https://matthewmacfarquhar.medium.com/webgpu-rendering-part-6-multiple-render-passes-b42157dfbcb5)

### Coordinate Systems
- [WebGPU Specification](https://www.w3.org/TR/webgpu/)
- [Coordinate systems · gpuweb Issue #416](https://github.com/gpuweb/gpuweb/issues/416)
- [Uniform buffers and a 3d camera \| Learn Wgpu](https://sotrh.github.io/learn-wgpu/beginner/tutorial6-uniforms/)
- [Homogeneous Coordinates, Clip Space, and NDC](https://carmencincotti.com/2022-05-02/homogeneous-coordinates-clip-space-ndc/)

### Instancing and Progressive Rendering
- [How to advance through vertex buffer across instances? · wgpu Discussion #2016](https://github.com/gfx-rs/wgpu/discussions/2016)
- [Instancing \| Learn Wgpu](https://sotrh.github.io/learn-wgpu/beginner/tutorial7-instancing/)
- [Decoupling Vertex Buffers from Render Pipeline · wgpu Issue #191](https://github.com/gfx-rs/wgpu-rs/issues/191)

---
*Pitfalls research for: GPU-rendered IDE UI with wgpu v23, winit v0.30, glyphon v0.7*
*Researched: 2026-01-28*
