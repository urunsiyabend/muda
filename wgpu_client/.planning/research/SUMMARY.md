# Project Research Summary

**Project:** wgpu_client - GPU-Rendered IDE UI
**Domain:** Visual regression testing and rendering quality for wgpu-based IDE applications
**Researched:** 2026-01-28
**Confidence:** HIGH

## Executive Summary

This project is a GPU-accelerated IDE UI built with Rust, wgpu, and glyphon text rendering. The current codebase has critical rendering bugs (disappearing sidebar elements, missing tab bar text, text overflow) caused by shared renderer state corruption and improper GPU resource lifecycle management. Research shows these are classic wgpu immediate-mode UI pitfalls with well-established solutions.

The recommended approach is a three-phase roadmap: (1) Fix rendering architecture by establishing clear component boundaries and GPU resource lifecycle patterns; (2) Add visual regression testing infrastructure using insta + image-compare to prevent future regressions; (3) Optimize performance and startup time with async pipeline creation and render pass batching. This follows proven patterns from projects like iced, egui, and warp-terminal.

The primary risk is attempting to optimize performance before fixing architectural issues. The pitfalls research makes clear that shared renderer state corruption must be resolved before adding features or optimization. The architecture already has the right primitives (Bounds struct, split-based layout, glyphon integration) — implementation just needs enforcement mechanisms (scissor rects, proper batching, coordinate space discipline).

## Key Findings

### Recommended Stack

The research identified a mature stack for GPU-rendered UI testing and rendering. Core technologies are battle-tested in production Rust applications with strong documentation and active maintenance.

**Core technologies:**
- **insta 1.46.1 + cargo-insta**: Snapshot testing framework — industry standard for Rust snapshot testing with excellent CI integration and interactive review workflow
- **image-compare 0.5.0**: SSIM/RMS image comparison — best balance of speed and perceptual accuracy for visual regression testing, pure Rust implementation
- **glyphon 0.7 + cosmic-text**: GPU text rendering — already integrated, handles glyph atlas and text shaping correctly when used properly
- **wgpu 0.23**: Cross-platform GPU API — modern foundation, but requires disciplined coordinate space management and proper render pass organization
- **image 0.25**: PNG I/O for screenshots — de facto standard, lossless format essential for test snapshots

**Critical version compatibility:**
- insta and cargo-insta versions must match exactly (1.46.1)
- wgpu render-to-texture pattern requires COPY_SRC usage flag on textures
- glyphon TextBounds requires physical pixel coordinates (scale_factor multiplication)

### Expected Features

The feature research identified three categories based on current IDE UI standards (VS Code, Visual Studio 2026, IntelliJ).

**Must have (table stakes):**
- Stable element rendering — UI elements must not disappear after interaction (CRITICAL BUG)
- Visible text labels — all UI text must render correctly (CRITICAL BUG)
- Correct layout bounds — text must not overflow into adjacent regions (CRITICAL BUG)
- Fast startup time — IDE responsive within 1-2 seconds on modern hardware
- Smooth scrolling — 60+ FPS text scrolling with GPU rendering advantage
- Responsive layout — no broken layouts on window resize

**Should have (competitive):**
- Visual regression testing — automated screenshot comparison to prevent rendering bugs
- Smooth animations — command palette fade, panel resize transitions
- Pixel-perfect alignment — round coordinates to physical pixels for crisp rendering
- High-DPI support — proper scale factor handling for Retina/4K displays
- Contextual UI polish — status bar with relevant information (git branch, encoding)

**Defer (v2+):**
- Customizable layout — draggable panel splitters, persistent state
- Adaptive theming — separate IDE vs editor theme system
- Micro-interactions — hover shadows, elaborate state transitions

**Anti-features to avoid:**
- Real-time layout animations for every change (causes jank)
- Unlimited GPU buffer sizes (causes OOM crashes)
- Blocking GPU initialization (freezes UI on startup)
- Per-frame full layout recalculation (massive CPU overhead)
- Silent error handling with unwrap_or defaults (hides bugs)

### Architecture Approach

The architecture research identified a hierarchical split-based layout system that's well-suited for IDE chrome (fixed sidebars, status bars, tab bars) with wgpu scissor rectangles as the missing enforcement mechanism.

**Major components:**
1. **GpuRenderer** — orchestrates layout calculation (Bounds hierarchy) and render pass assembly; owns all component bounds
2. **Bounds struct** — defines rectangular clipping regions via split_horizontal/split_vertical methods; prevents overlap by construction
3. **Component renderers** — TextArea, StatusBar, Sidebar, TabBar each accept Bounds from renderer and prepare GPU data within constraints
4. **StyledRectRenderer + UITextRenderer** — shared rendering infrastructure that batches geometry across components (currently causing bugs due to improper clearing)
5. **glyphon integration** — handles text rasterization with TextBounds software clipping (works correctly for text, needs complementary scissor rects for geometry)

**Key architectural patterns:**
- Split-based layout for fixed hierarchies (status bar, sidebars) — simple, explicit, no solver overhead
- Logical coordinates in UI layer, convert to physical at GPU boundary — critical for DPI correctness
- Scissor rectangles for hardware clipping enforcement — zero-cost boundary enforcement, the missing piece
- Separate renderer instances per logical layer OR batch all geometry before clearing — prevents state corruption

**Current gaps:**
- No scissor rect usage anywhere in codebase (rectangles can overflow)
- Shared renderer state corruption from multiple clear() calls per frame
- Coordinate space conversions not consistently applied (causes overflow bugs)

### Critical Pitfalls

Based on wgpu community experience and codebase analysis, these are the highest-impact pitfalls:

1. **Shared Renderer State Corruption** — Calling clear() and prepare() multiple times per frame on shared StyledRectRenderer causes previous components' geometry to disappear. Root cause of sidebar icon disappearing bug. **Fix:** Batch all geometry before clearing, or use separate renderer instances per layer.

2. **Glyphon Buffer Reuse Without Proper Clearing** — Multiple glyphon prepare() calls between renders causes only the last text to appear. Root cause of missing tab bar text. **Fix:** Collect all TextAreas in Vec, call prepare() once per frame with all areas.

3. **Viewport/Bounds Coordinate Space Confusion** — Mixing logical (DPI-independent) and physical (pixel) coordinates causes overflow and clipping failures. Root cause of text overflowing into tab bar. **Fix:** Establish single source of truth (logical in UI, physical at GPU boundary), document coordinate space for every field.

4. **Slow Startup From Synchronous Pipeline Creation** — Creating render pipelines blocks main thread for 500ms-2000ms. Current codebase creates 12+ pipelines synchronously in GpuRenderer::new(). **Fix:** Use create_render_pipeline_async() or show loading screen.

5. **Buffer Reallocation Thrashing on Dynamic Content** — Destroying and recreating buffers every frame as content size changes causes stuttering. **Fix:** Pre-allocate 2x expected size, grow with 1.5x factor, never shrink.

## Implications for Roadmap

Based on research, the roadmap should prioritize architectural fixes over features or optimization. The existing code has the right structure (Bounds, split-based layout, glyphon integration) but lacks enforcement mechanisms.

### Phase 1: Fix Rendering Architecture
**Rationale:** All three critical bugs (disappearing elements, missing text, overflow) stem from architectural issues in renderer state management and coordinate space handling. Must establish solid foundation before adding features or optimization.

**Delivers:**
- Stable rendering with no disappearing elements
- All text labels visible and correctly positioned
- Proper clipping boundaries enforced via scissor rects
- Validated coordinate space conversions

**Addresses features:**
- Stable element rendering (table stakes)
- Visible text labels (table stakes)
- Correct layout bounds (table stakes)
- Responsive layout (table stakes)

**Avoids pitfalls:**
- Shared renderer state corruption (establish proper batching)
- Glyphon buffer clearing (single prepare() call with all areas)
- Coordinate space confusion (audit and fix all conversions)

**Implementation approach:**
1. Add scissor rectangles to all render passes (wgpu::RenderPass::set_scissor_rect)
2. Refactor StyledRectRenderer to batch all geometry before clear()
3. Collect all TextAreas and call glyphon prepare() once per frame
4. Audit coordinate space conversions (logical vs physical)
5. Add bounds validation in debug builds
6. Add debug visualization for bounds (wireframe overlay)

**Research needs:** SKIP — patterns well-documented in wgpu docs and Learn Wgpu tutorials

### Phase 2: Visual Regression Testing Infrastructure
**Rationale:** Once rendering is stable, add automated testing to prevent regressions. Visual bugs are the primary risk for GPU-rendered UI — manual testing doesn't scale. Insta snapshot testing is proven approach.

**Delivers:**
- Automated screenshot capture via render-to-texture
- SSIM-based image comparison with appropriate thresholds
- CI integration with snapshot artifact upload
- Developer workflow with cargo-insta review

**Addresses features:**
- Visual regression testing (competitive differentiator)
- Smooth scrolling validation (baseline quality)
- High-DPI support verification (competitive)

**Uses stack:**
- insta 1.46.1 for snapshot management
- image-compare 0.5.0 for SSIM comparison
- image 0.25 for PNG I/O
- wgpu render-to-texture for screenshot capture

**Implementation approach:**
1. Implement render-to-texture capture (wgpu::Texture with COPY_SRC usage)
2. Add buffer readback with proper alignment (256-byte rows)
3. Integrate insta with PNG snapshots
4. Configure image-compare with SSIM threshold (~0.005-0.01 for text)
5. Add CI workflow with cargo-insta test --check
6. Document local review workflow

**Research needs:** LOW — wgpu windowless tutorial covers render-to-texture, insta docs cover CI integration. May need empirical testing to tune SSIM threshold for cross-GPU determinism.

### Phase 3: Startup Time Optimization
**Rationale:** With stable rendering and testing infrastructure, address user experience issue of slow startup (currently 500ms-2s). Async pipeline creation and lazy initialization are established patterns.

**Delivers:**
- Sub-300ms startup time on desktop
- Non-blocking GPU initialization
- Optional loading screen during pipeline creation
- Pipeline caching for subsequent launches

**Addresses features:**
- Fast startup time (table stakes)
- Smooth interactions (baseline quality)

**Avoids pitfalls:**
- Synchronous pipeline creation blocking main thread
- No progress feedback during initialization

**Implementation approach:**
1. Profile current startup with cargo flamegraph
2. Convert device.create_render_pipeline() to create_render_pipeline_async()
3. Add loading screen/splash screen
4. Implement lazy initialization for non-critical components
5. Add PipelineCache for disk caching
6. Benchmark across platforms (Windows, Linux, macOS)

**Research needs:** LOW — wgpu docs cover async pipeline creation, existing GitHub issues document patterns

### Phase 4: Performance Optimization
**Rationale:** After correctness and testing are established, optimize for performance. Render pass batching and buffer allocation strategies have known patterns but require measurement to validate.

**Delivers:**
- <10 render passes per frame (currently 8+)
- Zero buffer reallocation during normal usage
- 60 FPS maintained with dynamic content (scrolling, trees)
- GPU utilization >60%

**Addresses features:**
- Smooth scrolling (baseline quality)
- Responsive layout (table stakes)

**Avoids pitfalls:**
- Render pass fragmentation (too many passes)
- Buffer reallocation thrashing
- Unbatched text rendering

**Implementation approach:**
1. Profile with GPU profiling tools (Nsight, RenderDoc)
2. Batch compatible geometry into single render passes
3. Implement buffer over-allocation (2x strategy)
4. Add buffer growth factor (1.5x-2x)
5. Consider RenderBundles for repeated geometry
6. Measure and validate improvements

**Research needs:** MEDIUM — performance optimization requires empirical measurement and profiling. General patterns documented but specific optimizations depend on actual bottlenecks.

### Phase Ordering Rationale

1. **Architecture fixes first** because all current bugs stem from state management and boundary enforcement issues. Adding features on broken foundation wastes effort.

2. **Testing infrastructure second** because visual regression testing only makes sense after rendering is stable. Need correct baseline before detecting deviations.

3. **Startup optimization third** because async pipeline creation requires stable rendering architecture to be effective. No point optimizing broken code.

4. **Performance optimization last** because optimization requires measurement, and measurement requires stable baseline. Premature optimization would target wrong bottlenecks.

This ordering follows "make it work, make it right, make it fast" principle. Each phase builds on previous phase's deliverables.

### Research Flags

**Phases with standard patterns (skip dedicated research-phase):**
- **Phase 1 (Architecture):** Scissor rects, render pass management, coordinate spaces all well-documented in wgpu official docs and Learn Wgpu tutorials
- **Phase 2 (Testing):** Insta snapshot testing, render-to-texture, image comparison all have authoritative documentation and examples
- **Phase 3 (Startup):** Async pipeline creation documented in wgpu API docs, examples in wgpu repo

**Phases needing deeper research during planning:**
- **Phase 4 (Performance):** Optimization targets depend on actual profiling results. May need research into specific bottlenecks discovered (e.g., atlas eviction, text shaping, specific GPU driver issues).

**Validation needed during implementation:**
- Cross-GPU determinism testing (Phase 2) — empirical testing needed to determine acceptable SSIM threshold across NVIDIA/AMD/Intel GPUs
- Font rendering platform differences (Phase 2) — may need research into glyphon hinting settings if text snapshots vary across platforms
- Startup time on different platforms (Phase 3) — Windows/Linux/macOS may have different bottlenecks

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All recommended technologies verified through official docs and Context7. Version compatibility confirmed. wgpu patterns validated in Learn Wgpu tutorials. |
| Features | HIGH | Feature landscape derived from codebase analysis + current IDE industry standards (VS Code, Visual Studio 2026). Table stakes vs competitive distinction clear. |
| Architecture | HIGH | Split-based layout pattern present in codebase and verified in community examples. Scissor rect solution confirmed in wgpu API docs. Coordinate space issues documented in WebGPU spec. |
| Pitfalls | HIGH | All five critical pitfalls verified in wgpu GitHub issues, glyphon source code, and Learn Wgpu documentation. Root causes match observed bugs in codebase. |

**Overall confidence:** HIGH

The research is strongly grounded in official documentation, verified community patterns, and direct codebase analysis. The three critical bugs (disappearing elements, missing text, overflow) map directly to identified pitfalls with documented solutions.

### Gaps to Address

**Minor gaps requiring validation during implementation:**

- **Cross-GPU SSIM threshold** (Phase 2) — Research recommends 0.005-0.01 for text rendering based on community reports, but actual threshold must be determined empirically by running tests on NVIDIA, AMD, and Intel GPUs. May need separate thresholds for text vs geometric rendering.

- **Font rendering platform variance** (Phase 2) — Research mentions glyphon uses different rasterizers on different platforms and font hinting varies. Will need empirical testing to determine if snapshots can be cross-platform or if platform-specific snapshots are required. Mitigation: disable font hinting (FontHinting::None), bundle fonts instead of using system fonts.

- **Startup time breakdown** (Phase 3) — Research identifies synchronous pipeline creation as bottleneck, but actual profiling may reveal additional issues (font loading, texture atlas creation, initial layout). Will need cargo flamegraph profiling to validate optimization targets.

- **Render pass batching strategy** (Phase 4) — Research recommends <10 passes per frame and batching compatible geometry, but specific batching strategy depends on profiling results. May need experimentation to find optimal grouping (by shader vs by component vs by Z-layer).

**Strategy for gaps:**
- All gaps are "measure and validate" type issues that require implementation and empirical testing
- None block Phase 1 (architecture fixes) which addresses critical bugs
- Phase 2 (testing) provides mechanism to validate solutions to these gaps
- Document empirical findings in phase completion reports

## Sources

### Primary (HIGH confidence)
- **wgpu official docs** (docs.rs/wgpu) — Scissor rect API, render pass management, texture operations
- **Learn Wgpu tutorials** (sotrh.github.io/learn-wgpu) — Windowless rendering, buffer management, render-to-texture pattern
- **insta documentation** (insta.rs) — Snapshot testing patterns, CI integration, cargo-insta workflow
- **glyphon GitHub + source** (github.com/grovesNL/glyphon) — TextBounds clipping, prepare() lifecycle, TextRenderer internals
- **image-compare crates.io** — SSIM algorithm, threshold recommendations, API usage
- **WebGPU specification** (w3.org/TR/webgpu) — Coordinate systems, scissor rect spec, render pass semantics
- **Current codebase** (.planning/PROJECT.md, src/) — Known bugs, existing architecture, implementation patterns

### Secondary (MEDIUM confidence)
- **wgpu GitHub issues and discussions** — Community-reported pitfalls, performance issues, lifecycle questions. Issues #6155, #3332, #5293, #5397, #1438, discussions #6284, #5525
- **Tony Finn blog: Screenshot testing with Rust** (tonyfinn.com) — Testing patterns validated but xray crate unmaintained (use concepts not library)
- **Warp terminal blog: Text Rendering Adventures** (warp.dev/blog) — Glyph atlas best practices from production IDE
- **Visual Studio 2026 UX overview** (devblogs.microsoft.com) — Modern IDE visual quality standards, design principles
- **iced and egui repositories** — Community validation of immediate-mode GPU UI patterns

### Tertiary (LOW confidence - needs validation)
- **Cross-GPU variance threshold recommendations** — Multiple sources mention SSIM 0.005-0.01 for text but no authoritative source. Empirical testing required for project-specific validation.

---
*Research completed: 2026-01-28*
*Ready for roadmap: yes*
