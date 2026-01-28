# Domain Pitfalls: GPUI-Like UI Framework in Rust

**Domain:** GPU-driven reactive UI framework (wgpu + winit + glyphon + taffy)
**Researched:** 2026-01-28
**Confidence:** MEDIUM (Web search findings corroborated with existing codebase analysis and multiple sources)

---

## Critical Pitfalls

Mistakes that cause rewrites, major performance issues, or architectural dead-ends.

### Pitfall 1: Event Loop Ownership Architecture Mismatch

**What goes wrong:** winit's event loop takes full control via `EventLoop::run()`, which never returns. Attempting to maintain control outside the framework leads to platform-specific failures (web, iOS) and fighting the API design.

**Why it happens:** Developers coming from traditional game loops or retained-mode UIs expect to control the main loop. winit 0.30+ removed `poll_events()` specifically because it couldn't work properly on all platforms.

**Consequences:**
- Incompatible with multiple event loop instances
- Cannot integrate with existing application lifecycle
- Forces complete restructuring if discovered late
- Platform-specific behavior divergence (RedrawRequested ordering varies by platform)

**Prevention:**
- Design ora to own the winit event loop from day one (`ora::run(app)` takes full control)
- Document clearly that ora controls application lifecycle, not the reverse
- Accept that wgpu_client becomes a thin shell that hands off control to ora
- Test on multiple platforms early (Windows, Linux, macOS at minimum)

**Detection:**
- Attempting to call `window.request_redraw()` from outside the event loop
- Platform-specific crashes on web or mobile targets
- Race conditions where events arrive out of order

**Phase impact:** Phase 1 (Core Framework) must get this right. No middle ground exists.

**Sources:**
- [EventLoop 3.0 Changes · Issue #2900 · rust-windowing/winit](https://github.com/rust-windowing/winit/issues/2900)
- [How does the event loop system work for this library? · Discussion #3662](https://github.com/rust-windowing/winit/discussions/3662)

---

### Pitfall 2: Immediate-Mode Rebuild Without Incremental Rendering

**What goes wrong:** Rebuilding the entire UI tree every frame (immediate-mode style) performs well for small UIs but becomes CPU-bound at scale. Without incremental invalidation, scrolling a large file or hovering over many elements causes frame drops.

**Why it happens:** GPUI's immediate-mode API is elegant and easy to use, making developers defer optimization. The framework "just works" until it doesn't—usually discovered during performance profiling late in development.

**Consequences:**
- Scrolling stutters on files > 1000 lines
- Hover effects lag when many elements are present
- CPU pegged at 100% even when nothing changes visually
- Power consumption spikes on laptops
- 120 FPS target impossible to maintain

**Prevention:**
- Implement dirty region tracking from Phase 1
- Cache render model and only rebuild on state changes (muda already has this issue: `build_render_model()` called every frame unconditionally)
- Use compositing and invalid region tracking (WebRender had to retrofit this painfully)
- Add frame time budgeting (4ms batching made terminal emulator usable)
- Profile early with realistic data (10k+ line files, 100+ UI elements)

**Detection:**
- Frame times > 16ms (60 FPS) or > 8ms (120 FPS)
- CPU usage high even when idle
- Profiler shows significant time in `View::render()` every frame
- Battery drain during static screens

**Phase impact:** Phase 2 (Layout & Rendering) must include invalidation strategy. Retrofitting is painful.

**Sources:**
- [Eight million pixels and counting – GUIs on the GPU](https://nical.github.io/drafts/gui-gpu-notes.html)
- [Building a GPU-Accelerated Terminal Emulator with Rust and GPUI](https://dev.to/zhiwei_ma_0fc08a668c1eb51/building-a-gpu-accelerated-terminal-emulator-with-rust-and-gpui-4103)
- [GPUI: A Deep Dive into the High-Performance Rust UI Framework](https://beckmoulton.medium.com/gpui-a-technical-overview-of-the-high-performance-rust-ui-framework-powering-zed-ac65975cda9f)

---

### Pitfall 3: Draw Call Explosion Without Aggressive Batching

**What goes wrong:** Submitting hundreds or thousands of draw calls per frame becomes CPU-bound before GPU-bound. Rule of thumb: > 100 draw calls will stutter on mobile or integrated GPUs. Typical frame with 20k+ quads requires sophisticated batching.

**Why it happens:** Naive rendering treats each UI element as a separate draw call. Text rendering is especially vulnerable—each glyph can become a draw call without proper batching.

**Consequences:**
- Performance bottleneck is CPU, not GPU (GPU sits idle)
- Frame time dominated by draw call submission overhead
- Mobile/integrated GPU performance collapse
- Cannot achieve target FPS regardless of scene complexity

**Prevention:**
- Use instanced rendering (single unit quad mesh, instanced per element)
- Batch all quads by material/texture into single draw call
- Group text glyphs by atlas region into batches
- Minimize pipeline state changes (use same PipelineLayout across pipelines to avoid rebinding)
- Measure draw call count in debug builds (add warning at > 100 per frame)

**Detection:**
- Profiler shows significant time in `wgpu::RenderPass::draw()`
- GPU utilization < 50% while frame rate drops
- Adding more UI elements causes linear slowdown
- Debug overlay shows > 100 draw calls per frame

**Phase impact:** Phase 2 (Layout & Rendering) batching strategy is critical. Cannot be added later without full renderer rewrite.

**Sources:**
- [Eight million pixels and counting – GUIs on the GPU](https://nical.github.io/drafts/gui-gpu-notes.html)
- [Honest Feedback on WGPU and the Need for Better Learning Resources · Issue #8010](https://github.com/gfx-rs/wgpu/issues/8010)

---

### Pitfall 4: Reentrancy Bugs in Reactive State Updates

**What goes wrong:** Emitting events during event handling causes reentrancy—listener function emits to the same emitter it's subscribed to, creating infinite loops, stack overflows, or state corruption.

**Why it happens:** Reactive frameworks naturally want to propagate changes immediately. Model changes trigger view updates which trigger more model changes. Classic observer pattern reentrancy.

**Consequences:**
- Stack overflow crashes during state updates
- Infinite update loops that freeze the UI
- Subtle state corruption when nested updates interleave
- Impossible to debug without understanding full event graph

**Prevention:**
- Use effect queue system (GPUI pattern: `cx.notify()` pushes to queue, processes later)
- Give run-to-completion semantics: handle one event fully before next
- Never invoke listeners directly from `emit()` or `notify()`
- Document clearly: "State updates are queued, not immediate"
- Add cycle detection in debug builds

**Detection:**
- Stack traces showing recursive `View::render()` or `Model::update()`
- UI freezes during specific state changes
- Stack overflow crashes with "exceeded recursion limit"
- Debug cycle detector fires

**Phase impact:** Phase 3 (Reactive State) must implement effect queue architecture. Cannot be patched in later.

**Sources:**
- [Ownership and data flow in GPUI — Zed's Blog](https://zed.dev/blog/gpui-ownership)
- [GPUI Framework | zed-industries/zed | DeepWiki](https://deepwiki.com/zed-industries/zed/2.2-gpui-framework)

---

### Pitfall 5: wgpu API Churn and Breaking Changes

**What goes wrong:** wgpu is pre-1.0 and introduces breaking changes frequently. Tutorial code breaks within months. Migration paths are unclear. Working code becomes unbuildable after dependency updates.

**Why it happens:** wgpu tracks the evolving WebGPU specification, which is still in Working Draft phase. As spec details change, wgpu API changes. The ecosystem is young and rapidly evolving.

**Consequences:**
- Cannot upgrade dependencies without rewriting rendering code
- Security fixes and performance improvements locked out
- Codebase fragments across wgpu versions (transitive deps diverge)
- Maintenance burden escalates over time
- Tutorials and examples become useless

**Prevention:**
- Pin wgpu version explicitly in Cargo.toml (already done: `wgpu = "23"`)
- Budget 1-2 weeks per major version upgrade
- Create abstraction layer between ora and wgpu (don't expose wgpu types in public API)
- Monitor wgpu changelog and test pre-releases on feature branch
- Accept 6-12 month lag behind latest wgpu version
- Document which wgpu version ora targets

**Detection:**
- Compilation failures after `cargo update`
- Deprecation warnings from wgpu
- Performance regressions after wgpu upgrade
- New validation errors in existing code

**Phase impact:** All phases. Design ora's public API to hide wgpu. Phase 1 abstraction layer is critical.

**Sources:**
- [Honest Feedback on WGPU and the Need for Better Learning Resources · Issue #8010](https://github.com/gfx-rs/wgpu/issues/8010)
- [wgpu/CHANGELOG.md at trunk · gfx-rs/wgpu](https://github.com/gfx-rs/wgpu/blob/trunk/CHANGELOG.md)

---

## Moderate Pitfalls

Mistakes that cause delays, technical debt, or require significant refactoring.

### Pitfall 6: Text Atlas Memory Exhaustion

**What goes wrong:** glyphon's texture atlas grows unbounded as new glyphs are encountered. Long editing sessions with varied fonts, sizes, or large unicode coverage exhaust GPU memory (especially integrated GPUs).

**Why it happens:** No eviction policy—once a glyph enters the atlas, it stays forever. Large font sizes (48pt+) with CJK text consume massive atlas space.

**Consequences:**
- VRAM exhaustion after extended sessions
- GPU allocation failures cause crashes
- Frame rate degrades as atlas grows (lookup overhead)
- Mobile/integrated GPU users hit limits first

**Prevention:**
- Implement LRU glyph eviction (muda CONCERNS.md already flags this)
- Cap atlas size and reuse old entries
- Use multiple atlases partitioned by font/size
- Consider signed distance field rendering for large sizes
- Monitor VRAM usage in telemetry

**Detection:**
- `wgpu::Device::create_texture()` allocation failures
- Frame time increases over session duration
- Memory profiler shows unbounded texture growth
- Crashes after 1+ hour editing sessions

**Phase impact:** Phase 2 (Layout & Rendering). Design glyph cache management upfront.

**Sources:**
- muda codebase CONCERNS.md (line 92-98: "GPU Texture Atlas Growth Unbounded")
- [glyphon | 🦅🦁 Fast, simple 2D text renderer for wgpu](https://kandi.openweaver.com/rust/grovesNL/glyphon)

---

### Pitfall 7: Taffy Layout Integration and Coordinate Space Mismatch

**What goes wrong:** Taffy computes layout in logical coordinates (x, y, width, height), but GPU rendering requires device coordinates (pixels). Mixing coordinate systems causes clipping errors, misaligned text, and scissor rect bugs.

**Why it happens:** Layout engine works in abstract units, but GPU shaders need screen-space coordinates. Conversion logic scattered across codebase instead of centralized. DPI scaling multiplies the confusion.

**Consequences:**
- Text renders outside scissor rect
- Components clip incorrectly at window edges
- Mouse hover areas misaligned with visual elements
- DPI scaling breaks layout

**Prevention:**
- Define clear coordinate space types (`LogicalPos`, `DevicePos`)
- Centralize conversion functions (`logical_to_device()`, `device_to_logical()`)
- Document which coordinate space each API uses
- Use newtype pattern to prevent mixing (e.g., `struct LogicalX(f32)`)
- Test at multiple DPI scales (100%, 150%, 200%)

**Detection:**
- Components render outside bounds after window resize
- Mouse events don't align with visual elements
- Scissor clipping edge cases fail
- DPI scaling produces incorrect layouts

**Phase impact:** Phase 2 (Layout & Rendering). Establish coordinate system discipline early.

**Sources:**
- muda codebase CONCERNS.md (line 72: "Scissor rect calculations duplicated across components")
- [taffy - Rust](https://docs.rs/taffy)
- [GitHub - DioxusLabs/taffy: A high performance rust-powered UI layout library](https://github.com/DioxusLabs/taffy)

---

### Pitfall 8: Missing Compositor and Invalidation for Power Efficiency

**What goes wrong:** Without compositing and dirty region tracking, basic interactions (scrolling, button clicks) consume orders of magnitude more power than necessary. Battery life plummets. Users on laptops notice high power usage.

**Why it happens:** Developers optimize for frame rate, not power consumption. Immediate-mode rendering re-renders everything, even static regions. Mobile/laptop users pay the cost.

**Consequences:**
- High battery drain even during idle periods
- Laptops throttle CPU/GPU to conserve power (frame rate drops)
- Users perceive app as "heavy" or "power hungry"
- Reduced user session duration on battery

**Prevention:**
- Implement dirty region tracking (only redraw changed areas)
- Use compositing layers for static content
- Skip rendering if nothing changed since last frame
- Measure power consumption on laptops during development
- Add "power saving mode" for battery users

**Detection:**
- Power profiler shows high CPU/GPU usage when idle
- Battery drains faster than similar apps
- Laptop fans spin up during light usage
- Frame buffer unchanged but GPU still rendering

**Phase impact:** Phase 2 (Layout & Rendering). Retrofitting is extremely painful (WebRender experience).

**Sources:**
- [Eight million pixels and counting – GUIs on the GPU](https://nical.github.io/drafts/gui-gpu-notes.html)

---

### Pitfall 9: Flexbox Interop with Grid and Nested Layout Complexity

**What goes wrong:** Taffy supports both Flexbox and CSS Grid, but their interaction in nested contexts causes layout bugs. Indefinitely sized containers with mixed layout modes produce incorrect dimensions.

**Why it happens:** Flexbox and Grid have different sizing semantics. Nested layouts require multiple measurement passes. Edge cases around min/max constraints interact poorly.

**Consequences:**
- Components render with zero width/height
- Nested layouts produce incorrect dimensions
- Flex-grow doesn't work as expected in grid containers
- Performance degrades with deeply nested layouts

**Prevention:**
- Test nested layouts early (Flex in Grid, Grid in Flex)
- Document which layout combinations are supported
- Use simple layout hierarchy (prefer Flexbox-only for ora v1)
- Add layout validation in debug builds (detect zero-sized nodes)
- Defer Grid layout to future milestone if complexity exceeds budget

**Detection:**
- Components invisible due to zero dimensions
- Layout changes when parent container resizes unexpectedly
- Flex-basis calculations produce wrong values
- Performance degrades with nested layouts

**Phase impact:** Phase 2 (Layout & Rendering). Decide if Grid is needed for v1.

**Sources:**
- [Support multiple layout algorithms · Issue #28 · DioxusLabs/taffy](https://github.com/DioxusLabs/taffy/issues/28)
- [Support CSS Grid · Issue #204 · DioxusLabs/taffy](https://github.com/DioxusLabs/taffy/issues/204)

---

### Pitfall 10: Render Pipeline State Incompatibility Panics

**What goes wrong:** wgpu panics with "Render pipeline targets are incompatible with render pass" when FragmentState color attachment configuration doesn't match RenderPassDescriptor. Debugging is cryptic.

**Why it happens:** Pipeline creation happens separately from render pass setup. Configuration mismatch only detected at runtime. Error messages don't clearly explain which attachments are incompatible.

**Consequences:**
- Runtime panics instead of compile-time errors
- Cryptic error messages make debugging difficult
- Adding new render targets breaks existing pipelines
- Multi-pass rendering requires careful state management

**Prevention:**
- Create helper functions that construct matched pipeline + render pass pairs
- Document color attachment requirements clearly
- Use consistent PipelineLayout across all pipelines (avoids rebinding resources)
- Add validation layer in debug builds with better error messages
- Test all render pass combinations during development

**Detection:**
- Panic with "incompatible with render pass" message
- Validation errors from wgpu backend
- Render pass state changes break existing pipelines

**Phase impact:** Phase 2 (Layout & Rendering). Design pipeline management strategy upfront.

**Sources:**
- [Relationship between `RenderPassDescriptor::color_attachments` and `FragmentState::targets` in `RenderPipelineDescriptor` is not well documented · Issue #7322](https://github.com/gfx-rs/wgpu/issues/7322)
- [wgpu render pipeline state management problems](https://github.com/gfx-rs/wgpu-rs/issues/18)

---

### Pitfall 11: State Management Lifecycle Bugs in Reactive Hooks

**What goes wrong:** Calling side-effect functions (like `material_assets.add()`) directly in component creation methods leaks resources. Each update cycle adds new materials/textures without cleanup.

**Why it happens:** Developers treat reactive `create()` methods like one-time constructors, but they run every update. State management requires "mostly functional" style with explicit side-effect control.

**Consequences:**
- Memory leaks (resources allocated every frame)
- Resource exhaustion (GPU handles leak)
- Performance degradation over time
- Subtle bugs where state persists unexpectedly

**Prevention:**
- Document clearly: "create() runs every update, not once"
- Provide explicit state management methods (`.insert()`, `.create_mutable()`)
- Use functional style in create(): minimize side effects
- Add resource leak detection in debug builds
- Use RAII patterns for GPU resources

**Detection:**
- Memory usage grows unbounded during updates
- Profiler shows increasing allocation rate
- Debug builds detect resource handle leaks
- Frame time increases over session duration

**Phase impact:** Phase 3 (Reactive State). Document lifecycle clearly. Add examples.

**Sources:**
- [GitHub - viridia/quill: A reactive UI framework for Bevy](https://github.com/viridia/quill)
- [GitHub - actuate-rs/actuate: A framework for declarative programming in Rust](https://github.com/actuate-rs/actuate)

---

## Minor Pitfalls

Mistakes that cause annoyance but are fixable without major refactoring.

### Pitfall 12: Font Metrics Measurement Fallback Inaccuracy

**What goes wrong:** Glyph measurement fails silently and returns hardcoded fallback (`font_size * 0.6`), causing text positioning errors for fonts where the approximation is wrong.

**Why it happens:** Font loading can fail, but error handling defaults to approximation instead of propagating failure. No warning logged.

**Consequences:**
- Text positioning slightly wrong for some fonts
- Monospace fonts may not align correctly
- Silent failure makes debugging difficult

**Prevention:**
- Log warning when fallback is used (muda CONCERNS.md already flags this)
- Ensure font loading succeeds before measuring
- Test with various fonts (monospace, variable-width, CJK)
- Add fallback metric validation (compare to actual measurements)

**Detection:**
- Text alignment slightly off in specific fonts
- Monospace grid doesn't align
- No error logged but positioning wrong

**Phase impact:** Phase 2 (Layout & Rendering). Add logging and validation.

**Sources:**
- muda codebase CONCERNS.md (line 26-29: "Character Width Measurement Fallback")

---

### Pitfall 13: Manual Viewport Cache Invalidation Bugs

**What goes wrong:** Manually caching viewport dimensions to avoid recalculation causes state synchronization bugs. Cache invalidation logic misses edge cases (file open, window resize).

**Why it happens:** Performance optimization to avoid recalculating viewport every frame, but cache invalidation requires tracking all state changes that affect dimensions.

**Consequences:**
- Viewport dimensions stale after certain operations
- Text rendering uses wrong bounds
- Layout doesn't update after state changes

**Prevention:**
- Remove manual cache, recalculate every frame (measure first—may not be bottleneck)
- Use automatic invalidation (dirty flag on state changes)
- If cache needed, centralize invalidation logic
- Add assertions that cached value matches calculated value

**Detection:**
- Layout doesn't update after window resize
- Text renders outside bounds after file open
- Assertions fire: cached != calculated

**Phase impact:** Phase 2 (Layout & Rendering). Validate caching is necessary before implementing.

**Sources:**
- muda codebase CONCERNS.md (line 19-23: "Manual Viewport Caching")

---

### Pitfall 14: CSS Transition Animation System Undefined Value Handling

**What goes wrong:** Animation libraries that assume undefined keyframe values should use defaults break advanced animations (merged timelines, property-specific easing).

**Why it happens:** CSS transition semantics unclear. Some libraries fill in defaults, breaking composition.

**Consequences:**
- Complex animations don't compose correctly
- Merged timelines produce wrong interpolation
- Per-property easing conflicts with defaults

**Prevention:**
- Don't assume undefined values = use defaults
- Allow explicit "inherit" vs "undefined" in keyframes
- Document animation composition semantics clearly
- Test merged timelines and property-specific easing

**Detection:**
- Animations don't interpolate as expected
- Merged timelines produce jumps or wrong values
- Per-property easing ignored

**Phase impact:** Phase 4 (Transitions). Design animation API carefully.

**Sources:**
- [Mina — Rust GUI library // Lib.rs](https://lib.rs/crates/mina)
- [mina - Rust](https://docs.rs/mina)

---

### Pitfall 15: Component Size Token Defined But Unused

**What goes wrong:** Design system defines size tokens (ComponentSize enum, Space enum) but components ignore them and use raw literals. Token system provides no value if not enforced.

**Why it happens:** Existing code predates token system. Migration incomplete. No enforcement mechanism.

**Consequences:**
- Design system exists but is fiction
- Components still hardcode values
- Changing tokens doesn't affect layout
- Wasted effort defining unused tokens

**Prevention:**
- Migrate existing components to token system (muda pain point)
- Add lints to detect raw float literals in layout code
- Code review checklist: "Uses design tokens, not raw values"
- Make raw values inaccessible (newtype wrappers, private fields)

**Detection:**
- Grep codebase for raw float literals (`24.0`, `8.0`)
- Components don't respond to token changes
- Design system file never imported

**Phase impact:** Phase 5 (Design System). Enforce token usage from start.

**Sources:**
- muda codebase PROJECT.md (line 62-71: Pain points ora addresses)

---

## Phase-Specific Warnings

| Phase Topic | Likely Pitfall | Mitigation |
|-------------|---------------|------------|
| Phase 1: Core Framework & Event Loop | Event loop ownership mismatch | Accept winit owns the loop. Design ora as `ora::run(app)` from day one. Test on multiple platforms early. |
| Phase 1: Core Framework & Event Loop | wgpu API churn | Abstract wgpu behind ora's API. Pin wgpu version. Budget upgrade time. |
| Phase 2: Layout & Rendering | Immediate-mode rebuild performance | Implement dirty region tracking and incremental invalidation upfront. Profile with realistic data. |
| Phase 2: Layout & Rendering | Draw call explosion | Design batching strategy first. Use instancing. Measure draw call count. |
| Phase 2: Layout & Rendering | Coordinate space mismatch | Define logical vs device coordinate types. Centralize conversions. Test at multiple DPI. |
| Phase 2: Layout & Rendering | Compositor missing | Add compositing and dirty tracking early. Retrofitting is painful. |
| Phase 2: Layout & Rendering | Text atlas exhaustion | Implement glyph eviction policy. Cap atlas size. Monitor VRAM. |
| Phase 2: Layout & Rendering | Pipeline state panics | Create matched pipeline + render pass helpers. Use consistent PipelineLayout. |
| Phase 3: Reactive State | Reentrancy bugs | Use effect queue pattern. Never invoke listeners directly. Add cycle detection. |
| Phase 3: Reactive State | Lifecycle resource leaks | Document create() runs every update. Use functional style. Provide explicit state methods. |
| Phase 4: Transitions | Animation composition bugs | Don't assume undefined = default. Test merged timelines. Document semantics. |
| Phase 5: Design System Migration | Token system unused | Enforce token usage. Add lints for raw literals. Make raw values inaccessible. |
| Phase 5: Design System Migration | Viewport cache bugs | Remove manual cache if possible. Centralize invalidation. Add assertions. |

---

## Known Sharp Edges from Existing muda Codebase

These issues are already present in wgpu_client and should be prevented in ora:

1. **Heights hardcoded in multiple places** - Status bar height (24.0) appears in 4 places instead of referencing constant. ora must enforce single source of truth for all dimensions.

2. **Font metrics redefined per component** - Each component calculates its own font size/line height. ora text system must centralize typography.

3. **Spacing tokens unused** - Space enum exists but raw floats used instead. ora must make tokens the only way to specify spacing.

4. **Gutter width calculated twice** - Duplicated logic in separate locations. ora layout must have single calculation path per component.

5. **Parallel tab bar implementations** - Two versions with different heights (28px vs 35px). ora must prevent parallel implementations.

6. **Scissor rect duplication** - Clipping logic repeated across components. ora must handle clipping automatically.

7. **RenderModel rebuilt every frame** - Full rebuild even when nothing changed. ora must cache and invalidate incrementally.

8. **Syntax highlighting no cache** - Re-parses visible lines every frame. ora must integrate with tree-sitter incremental API.

**Prevention in ora:** These are exactly the problems ora solves. Roadmap must prioritize architectural decisions that prevent duplication and enforce single source of truth.

---

## Sources

### Web Search Results

- [GitHub - gfx-rs/wgpu: A cross-platform, safe, pure-Rust graphics API](https://github.com/gfx-rs/wgpu)
- [Honest Feedback on WGPU and the Need for Better Learning Resources · Issue #8010 · gfx-rs/wgpu](https://github.com/gfx-rs/wgpu/issues/8010)
- [wgpu/CHANGELOG.md at trunk · gfx-rs/wgpu](https://github.com/gfx-rs/wgpu/blob/trunk/CHANGELOG.md)
- [GitHub - DioxusLabs/taffy: A high performance rust-powered UI layout library](https://github.com/DioxusLabs/taffy)
- [Support multiple layout algorithms · Issue #28 · DioxusLabs/taffy](https://github.com/DioxusLabs/taffy/issues/28)
- [Support CSS Grid · Issue #204 · DioxusLabs/taffy](https://github.com/DioxusLabs/taffy/issues/204)
- [taffy - Rust](https://docs.rs/taffy)
- [GitHub - grovesNL/glyphon: 🦅🦁 Fast, simple 2D text renderer for wgpu](https://github.com/grovesNL/glyphon)
- [glyphon | 🦅🦁 Fast, simple 2D text renderer for wgpu](https://kandi.openweaver.com/rust/grovesNL/glyphon)
- [EventLoop in winit::event_loop - Rust](https://docs.rs/winit/latest/winit/event_loop/struct.EventLoop.html)
- [EventLoop 3.0 Changes · Issue #2900 · rust-windowing/winit](https://github.com/rust-windowing/winit/issues/2900)
- [How does the event loop system work for this library? · Discussion #3662](https://github.com/rust-windowing/winit/discussions/3662)
- [GPUI Framework | zed-industries/zed | DeepWiki](https://deepwiki.com/zed-industries/zed/2.2-gpui-framework)
- [GPUI: A Deep Dive into the High-Performance Rust UI Framework](https://beckmoulton.medium.com/gpui-a-technical-overview-of-the-high-performance-rust-ui-framework-powering-zed-ac65975cda9f)
- [Ownership and data flow in GPUI — Zed's Blog](https://zed.dev/blog/gpui-ownership)
- [Optimizing the Metal pipeline to maintain 120 FPS in GPUI — Zed's Blog](https://zed.dev/blog/120fps)
- [Building a GPU-Accelerated Terminal Emulator with Rust and GPUI](https://dev.to/zhiwei_ma_0fc08a668c1eb51/building-a-gpu-accelerated-terminal-emulator-with-rust-and-gpui-4103)
- [Eight million pixels and counting – GUIs on the GPU](https://nical.github.io/drafts/gui-gpu-notes.html)
- [Warp: Why is building a UI in Rust so hard?](https://www.warp.dev/blog/why-is-building-a-ui-in-rust-so-hard)
- [Relationship between `RenderPassDescriptor::color_attachments` and `FragmentState::targets` in `RenderPipelineDescriptor` is not well documented · Issue #7322](https://github.com/gfx-rs/wgpu/issues/7322)
- [Guidance on Pipelines and Buffers · Issue #18 · gfx-rs/wgpu-rs](https://github.com/gfx-rs/wgpu-rs/issues/18)
- [GitHub - viridia/quill: A reactive UI framework for Bevy](https://github.com/viridia/quill)
- [GitHub - actuate-rs/actuate: A framework for declarative programming in Rust](https://github.com/actuate-rs/actuate)
- [Mina — Rust GUI library // Lib.rs](https://lib.rs/crates/mina)
- [mina - Rust](https://docs.rs/mina)
- [Graphics: why immediate mode? - The Rust Programming Language Forum](https://users.rust-lang.org/t/graphics-why-immediate-mode/93356)
- [Towards principled reactive UI | Raph Levien's blog](https://raphlinus.github.io/rust/druid/2020/09/25/principled-reactive-ui.html)
- [Entity-Component-System architecture for UI in Rust | Raph Levien's blog](https://raphlinus.github.io/personal/2018/05/08/ecs-ui.html)

### Existing muda Codebase

- `.planning/PROJECT.md` (pain points, context, requirements)
- `.planning/codebase/CONCERNS.md` (tech debt, known bugs, performance bottlenecks, fragile areas)

---

**Confidence note:** Most findings are MEDIUM confidence (web search verified with multiple sources and existing codebase analysis). Some specifics (like wgpu breaking changes, winit event loop architecture) are HIGH confidence (official documentation/issues). Detailed performance numbers (draw call thresholds, frame times) are LOW-MEDIUM confidence (based on community reports, not ora-specific profiling).
