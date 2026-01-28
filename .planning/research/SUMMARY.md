# Project Research Summary

**Project:** ora - GPUI-inspired UI framework crate for muda code editor
**Domain:** GPU-accelerated immediate-mode UI framework for IDE/code editor
**Researched:** 2026-01-28
**Confidence:** HIGH

## Executive Summary

Building a production-quality UI framework for a code editor in Rust requires GPU acceleration via wgpu, flexible layout via taffy (flexbox), high-performance text rendering via glyphon, and a reactive state system based on entity/observer patterns. The standard approach in 2025 follows GPUI's proven architecture: centralized entity ownership in an App context, three-phase rendering (layout/prepaint/paint), deferred effect execution for reactivity, and aggressive GPU batching to achieve 120 FPS targets.

The recommended stack is mature and battle-tested. wgpu 28 is industry-standard for cross-platform GPU rendering (powers Firefox WebGPU, Zed editor). glyphon 0.9 wraps cosmic-text for HarfBuzz shaping with excellent performance. taffy 0.9 implements the full CSS Flexbox spec and is production-ready. For state management, slotmap provides generational arenas to prevent use-after-free, enabling GPUI's entity-based reactive pattern. This stack is already partially integrated in muda's wgpu_client (wgpu 23, glyphon 0.7), requiring a 2-3 day migration to latest versions.

The critical risks are architectural: (1) winit's event loop must own the application lifecycle—fighting this causes platform-specific failures, (2) immediate-mode UI rebuild without dirty region tracking causes frame drops at scale, (3) draw call explosion without batching pegs CPU before GPU is saturated, (4) reactive state updates without effect queues cause reentrancy bugs. All four risks are **preventable in Phase 1-3** if GPUI's proven patterns are followed. The existing muda codebase already exhibits symptoms of these risks (RenderModel rebuilt every frame, syntax highlighting re-parsed unnecessarily, manual caching causing invalidation bugs), making ora's architectural discipline critical.

## Key Findings

### Recommended Stack

The 2025 standard for building GPUI-like frameworks centers on wgpu for GPU abstraction, winit for windowing, glyphon/cosmic-text for text rendering, taffy for layout, and slotmap for entity storage. Animation requires dedicated easing libraries (keyframe) since no standard CSS transition library exists for wgpu contexts. All core technologies are stable, actively maintained, and proven in production (Zed, Dioxus, Bevy).

**Core technologies:**
- **wgpu 28**: Cross-platform GPU API (Metal/Vulkan/DX12) — industry standard, powers Firefox WebGPU and Zed; breaking changes every 3 months but ecosystem follows quickly
- **winit 0.30.12**: Cross-platform windowing — de facto standard for Rust GPU apps; mature API with excellent platform coverage
- **glyphon 0.9**: 2D text rendering for wgpu — wraps cosmic-text (HarfBuzz shaping) and etagere (atlas packing); supports ligatures, color emoji, bidirectional text
- **taffy 0.9.2**: Flexbox & CSS Grid layout — high-performance pure Rust implementation of CSS layout specs; faster than browser engines due to zero-cost abstractions
- **slotmap 1.0.7**: Generational arena for entity storage — prevents ABA problem with version checking; enables GPUI's centralized entity ownership pattern
- **keyframe 1.1.1**: Easing functions & keyframe animation — CSS-compatible cubic-bezier curves without reinventing standard easings
- **palette 0.7.6**: Color manipulation & conversion — type-safe color operations across multiple color spaces; essential for design tokens and accessibility

**Migration from current state:** Existing wgpu_client uses wgpu 23, winit 0.30, glyphon 0.7. Upgrade path requires 2-3 days (wgpu 23→28 API migration, glyphon 0.7→0.9 compatibility). Add taffy, slotmap, keyframe, palette as new dependencies.

**What NOT to include:** Full web UI frameworks (Dioxus, Leptos) are designed for VDOM/web targets, not immediate-mode GPU rendering. Game engines (Bevy) have ECS overkill for UI entity management. Other UI frameworks (egui, iced) are competitors with incompatible architectures. Deprecated crates (stretch, wgpu_glyph) have active maintained replacements.

### Expected Features

Research identified 12 table stakes features, 11 differentiators, and 14 anti-features to explicitly avoid in v1.

**Must have (table stakes):**
- Flexbox layout engine (taffy integration)
- GPU text rendering with glyph caching (glyphon wrapper)
- Scissor clipping for scrollable areas
- Mouse event routing with hit testing
- Keyboard event system with action registry
- Focus management with focus stack
- Reactive state system (Entity/Model with observers)
- Element tree diffing for dirty tracking
- Design token system (colors, spacing, typography)
- Styled primitives (Div, Text, Image)
- View trait system for declarative rendering
- Event loop ownership (framework controls winit)

**Should have (competitive):**
- CSS-like transitions (opacity, color, transform animations)
- Tailwind-style layout API (builder pattern: `.flex().gap(4).p(8)`)
- Hierarchical bounds calculation (parent provides bounds to children)
- Centralized typography scale (body, small, code, heading)
- Semantic color roles (primary_background, secondary_text, accent_active)
- Component size tiers (Small/Medium/Large variants)
- Automatic hover/active states (framework tracks mouse position)
- Integrated glyphon wrapper (hides FontSystem, atlas lifecycle complexity)
- Single-batch text rendering (all text in one pass with shared atlas)
- Layout debug overlay (toggle-able bounds visualization)

**Defer (v2+):**
- Multi-window support (adds cross-window state sync complexity)
- Custom shader API (breaks abstraction, 99% of UI doesn't need it)
- Keyframe/spring animations (CSS transitions sufficient for editor chrome)
- Web rendering target (WASM build, different event loop, missing APIs)
- Accessibility/screen reader v1 (foundation exists: semantic elements, focus)
- Hot-reload of themes (compile-time fast enough for theme development)
- Drag-and-drop API (implement specific cases: tab reorder when needed)
- Rich animation timeline (editors aren't animation-heavy apps)
- Virtualized scrolling (implement per-component: file tree, command palette)
- Built-in gesture recognition (editors are keyboard-first, mouse events sufficient)
- Component marketplace/plugins (ora is internal to muda in v1)
- Responsive breakpoints (editor layout is manual: user drags panels)
- Undo/redo for UI state (document undo/redo exists in core_editor)

### Architecture Approach

GPUI employs a layered architecture separating application state ownership, window management, element tree construction, layout computation, and GPU rendering. The ora framework should follow this proven pattern: centralized entity ownership (App owns all data, Entity<T> handles provide access), three-phase rendering (layout with taffy → prepaint commits hitboxes → paint writes Scene), deferred effect execution (observers queued, not invoked synchronously), and two-phase event dispatch (capture root→target, bubble target→root).

**Major components:**
1. **App** — Root context owning all entity data in SlotMap; dispatches effects (notify/emit)
2. **Context<T>** — Scoped entity access with notify/emit; borrows from App
3. **Window** — Window state, root view, event dispatch, frame orchestration
4. **Entity<T>** — Handle to App-owned data; access via update/read
5. **View trait** — Render method returning element tree; view-specific state
6. **Element trait** — Layout (request_layout → LayoutId) and paint (write to Scene) implementation
7. **DispatchTree** — Event routing via hit testing (mouse) or focus path (keyboard)
8. **Frame** — Hit testing and hitbox tracking for event dispatch
9. **Taffy** — Flexbox layout computation; computes all bounds in single pass
10. **Scene** — Retained draw command buffer (quads, text, images) submitted to GPU
11. **Blade/wgpu** — Cross-platform graphics abstraction generating GPU commands

**Data flow:** Platform event → Window normalizes → DispatchTree routes (two-phase: capture/bubble) → Handler updates state → cx.notify() queues effect → Effect flush triggers observers → Observers request redraw → Window renders root view → Three-phase rendering (prepaint layout, prepaint hitboxes, paint scene) → Scene submitted to GPU → Present.

**Patterns to follow:**
- Centralized entity ownership (App owns, Entity<T> handles provide access)
- Three-phase rendering (separate layout, prepaint, paint to prevent read-after-write)
- Deferred effect execution (queue effects, flush after update completes)
- Two-phase event dispatch (capture and bubble like DOM)
- Tailwind-style builder API (fluent methods for styling)

**Anti-patterns to avoid:**
- Direct Scene mutation in prepaint (violates phase separation)
- Synchronous observer execution (causes reentrancy bugs)
- Multiple Taffy layout passes per frame (expensive, causes frame drops)
- Storing mutable references in Elements (impossible with Rust borrow checker)
- Platform-specific code in Element trait (breaks cross-platform abstraction)

### Critical Pitfalls

Research identified 15 pitfalls (5 critical, 6 moderate, 4 minor) with phase-specific warnings.

1. **Event loop ownership architecture mismatch** — winit's EventLoop::run() never returns; framework must own lifecycle from day one. Attempting external control causes platform-specific failures. **Prevention:** Design ora as `ora::run(app)` pattern. Test on Windows/Linux/macOS early. **Phase 1 critical.**

2. **Immediate-mode rebuild without incremental rendering** — Rebuilding entire UI tree every frame performs well for small UIs but becomes CPU-bound at scale. Without dirty tracking, scrolling large files causes frame drops. **Prevention:** Implement dirty region tracking and element tree diffing in Phase 2. Profile with 10K+ line files early. **Phase 2 critical, retrofitting is painful (WebRender experience).**

3. **Draw call explosion without aggressive batching** — > 100 draw calls per frame stutters on mobile/integrated GPUs. Typical frame with 20K+ quads requires batching. **Prevention:** Use instanced rendering, batch quads by material, group text glyphs by atlas region. Measure draw call count in debug builds. **Phase 2 critical, cannot be added later without renderer rewrite.**

4. **Reentrancy bugs in reactive state updates** — Emitting events during event handling causes infinite loops or stack overflows. **Prevention:** Use effect queue (GPUI pattern: cx.notify() pushes to queue, processes later with run-to-completion semantics). Add cycle detection in debug builds. **Phase 3 critical, cannot be patched later.**

5. **wgpu API churn and breaking changes** — wgpu is pre-1.0, introduces breaking changes every 3 months. Tutorial code breaks within months. **Prevention:** Pin wgpu version explicitly, abstract wgpu behind ora's API (don't expose wgpu types publicly), budget 1-2 weeks per major upgrade, accept 6-12 month lag behind latest. **All phases, design abstraction layer in Phase 1.**

6. **Text atlas memory exhaustion** — glyphon's texture atlas grows unbounded; long sessions exhaust VRAM especially on integrated GPUs. **Prevention:** Implement LRU glyph eviction (muda CONCERNS.md flags this), cap atlas size, monitor VRAM usage. **Phase 2, design glyph cache management upfront.**

7. **Taffy layout integration and coordinate space mismatch** — Mixing logical coordinates (layout) and device coordinates (GPU rendering) causes clipping errors, misaligned text, scissor rect bugs. **Prevention:** Define clear coordinate space types (LogicalPos, DevicePos), centralize conversions, test at multiple DPI scales (100%, 150%, 200%). **Phase 2, establish coordinate system discipline early.**

8. **Missing compositor and invalidation for power efficiency** — Without dirty region tracking, basic interactions consume orders of magnitude more power. Battery life plummets. **Prevention:** Implement dirty region tracking, use compositing layers for static content, skip rendering if nothing changed. **Phase 2, retrofitting is painful (WebRender had to retrofit compositing).**

## Implications for Roadmap

Based on research, suggested 7-phase structure aligns with GPUI's layered architecture and addresses critical pitfalls in dependency order:

### Phase 1: Foundation - Core Framework & Event Loop
**Rationale:** Event loop ownership must be correct from day one. winit's EventLoop::run() takes full control; fighting this causes platform-specific failures (Pitfall #1). Framework abstractions (App, Context, Entity<T>) enable all subsequent phases. wgpu abstraction prevents API churn (Pitfall #5).

**Delivers:**
- App and Entity<T> system (centralized ownership)
- Context types (App, Context<T>, AsyncContext)
- Basic Element trait (without layout or paint)
- Window wrapper with winit integration (`ora::run(app)` pattern)
- wgpu abstraction layer (hide wgpu types from public API)

**Addresses (from FEATURES.md):**
- Event loop ownership (table stakes)
- View trait system (table stakes)

**Avoids (from PITFALLS.md):**
- Pitfall #1: Event loop ownership mismatch (critical)
- Pitfall #5: wgpu API churn (critical)

**Research flags:** Standard patterns (skip `/gsd:research-phase`). GPUI architecture well-documented.

### Phase 2: Layout & Rendering - GPU Pipeline & Flexbox
**Rationale:** Must implement rendering architecture (three-phase: layout/prepaint/paint) and batching strategy before any components can render. Dirty region tracking and batching cannot be added later without full rewrite (Pitfall #2, #3). Coordinate system discipline prevents layout bugs (Pitfall #7).

**Delivers:**
- Three-phase rendering (prepaint, paint, present)
- Taffy integration (request_layout, compute_layout)
- LayoutId and Bounds types with coordinate space safety (LogicalPos vs DevicePos)
- Scene structure (draw command buffer)
- GPU batching strategy (instanced rendering, single-pass text)
- Dirty region tracking and incremental invalidation
- wgpu pipeline integration (vertex/fragment shaders for quads)
- Compositor and invalid region tracking for power efficiency

**Addresses (from FEATURES.md):**
- Flexbox layout engine (table stakes)
- GPU text rendering (table stakes)
- Scissor clipping (table stakes)
- Styled primitives - Div, Text (table stakes)
- Single-batch text rendering (differentiator)

**Uses (from STACK.md):**
- wgpu 28 (GPU rendering)
- taffy 0.9.2 (flexbox layout)
- glyphon 0.9 (text rendering)
- bytemuck (vertex buffer uploads)

**Implements (from ARCHITECTURE.md):**
- Taffy (layout computation)
- Scene (draw command buffer)
- Element trait (request_layout, prepaint, paint methods)
- Three-phase rendering pipeline

**Avoids (from PITFALLS.md):**
- Pitfall #2: Immediate-mode rebuild without dirty tracking (critical)
- Pitfall #3: Draw call explosion without batching (critical)
- Pitfall #6: Text atlas memory exhaustion (moderate)
- Pitfall #7: Coordinate space mismatch (moderate)
- Pitfall #8: Missing compositor for power efficiency (moderate)
- Pitfall #10: Render pipeline state panics (moderate)

**Research flags:** Likely needs `/gsd:research-phase` for:
- GPU batching strategies (instanced rendering patterns)
- glyphon atlas management (LRU eviction, multi-atlas strategies)
- Three-phase rendering lifecycle (prepaint vs paint separation)

### Phase 3: Reactive State - Entity/Model Observation
**Rationale:** State management must implement effect queue pattern to prevent reentrancy (Pitfall #4). Observation mechanism enables reactive UI updates. Must exist before event system (event handlers mutate state).

**Delivers:**
- Observation mechanism (cx.observe, cx.notify)
- Event emission (cx.emit, cx.subscribe)
- Effect queue and flush logic
- Model<T> wrapper with change tracking
- Observer lifecycle management

**Addresses (from FEATURES.md):**
- Reactive state system (table stakes)
- Element tree diffing (table stakes)

**Uses (from STACK.md):**
- slotmap 1.0.7 (generational arena for entity storage)

**Implements (from ARCHITECTURE.md):**
- Entity<T> handle-based access
- Deferred effect execution pattern
- Observer registration and notification

**Avoids (from PITFALLS.md):**
- Pitfall #4: Reentrancy bugs in reactive updates (critical)
- Pitfall #11: State management lifecycle bugs (moderate)

**Research flags:** Standard patterns (skip `/gsd:research-phase`). GPUI's ownership model well-documented.

### Phase 4: Event System - Dispatch & Focus
**Rationale:** Event routing requires completed rendering pipeline (hit testing against Frame hitboxes) and reactive state (event handlers trigger updates). Two-phase dispatch is architectural, cannot be added later.

**Delivers:**
- DispatchTree and focus management
- Two-phase dispatch (capture/bubble)
- Hit testing and hitbox tracking
- Mouse and keyboard event normalization
- Action registry for keyboard shortcuts
- Focus stack with enter/exit callbacks

**Addresses (from FEATURES.md):**
- Mouse event routing (table stakes)
- Keyboard event system (table stakes)
- Focus management (table stakes)

**Implements (from ARCHITECTURE.md):**
- DispatchTree (event routing)
- Frame (hitbox registry)
- Two-phase event dispatch

**Avoids (from PITFALLS.md):**
- Focus loss bugs (moderate)

**Research flags:** Standard patterns (skip `/gsd:research-phase`). DOM-like event dispatch well-understood.

### Phase 5: Element Library - Styled Primitives & Builders
**Rationale:** Reusable element library depends on all foundational layers (rendering, state, events). Builder API provides DX before integrating with core_editor.

**Delivers:**
- Basic elements (div, text, image)
- Styled element API (Tailwind-style builder pattern)
- Layout elements (flex container, stack, grid)
- Interactive elements (button, input)
- Semantic color roles (primary_background, secondary_text, etc.)
- Component size tiers (Small/Medium/Large)
- Automatic hover/active states (framework tracks mouse)

**Addresses (from FEATURES.md):**
- Tailwind-style layout API (differentiator)
- Semantic color roles (differentiator)
- Component size tiers (differentiator)
- Automatic hover/active states (differentiator)

**Implements (from ARCHITECTURE.md):**
- Element trait implementations for primitives
- Builder pattern for element construction

**Research flags:** Standard patterns (skip `/gsd:research-phase`). Tailwind/CSS builder APIs well-established.

### Phase 6: Design System Migration - Tokens & Typography
**Rationale:** Design token system exists in wgpu_client/design_system but is unused (Pitfall #15). Must be enforced in ora with type-safe tokens to prevent raw value proliferation.

**Delivers:**
- Design token system (Color, Spacing, Typography, Elevation enums)
- Theme switching (dark/light mode palette swap)
- Centralized typography scale (body, small, code, heading)
- Hierarchical bounds calculation enforcement

**Addresses (from FEATURES.md):**
- Design token system (table stakes)
- Centralized typography scale (differentiator)
- Hierarchical bounds calculation (differentiator)

**Uses (from STACK.md):**
- palette 0.7.6 (color manipulation for derived tokens)

**Avoids (from PITFALLS.md):**
- Pitfall #15: Component size tokens defined but unused (minor)
- Existing muda pain points (heights hardcoded, spacing tokens unused)

**Research flags:** Standard patterns (skip `/gsd:research-phase`). Design token systems well-documented.

### Phase 7: Transitions & Polish - CSS-like Animations
**Rationale:** Transitions are polish, not core functionality. Can be added after all foundational layers work. Optional for MVP.

**Delivers:**
- CSS-like transitions (opacity, color, transform animations)
- Property interpolation with easing
- Transition state tracking
- Layout debug overlay (bounds visualization)

**Addresses (from FEATURES.md):**
- CSS-like transitions (differentiator)
- Layout debug overlay (differentiator)

**Uses (from STACK.md):**
- keyframe 1.1.1 (easing functions)

**Avoids (from PITFALLS.md):**
- Pitfall #14: Animation system undefined value handling (minor)

**Research flags:** May need `/gsd:research-phase` for:
- CSS transition semantics in GPU context (not well-documented)
- Property interpolation strategies (different types: color, position, opacity)

### Phase Ordering Rationale

- **Phase 1-2 foundational:** Event loop ownership and rendering architecture cannot be fixed later (Pitfall #1, #2, #3). Must be correct from day one.
- **Phase 3 before 4:** Event handlers trigger state updates; reactive system must exist first to handle updates correctly (prevent Pitfall #4).
- **Phase 4 before 5:** Element library needs event system for interactive components (buttons, inputs).
- **Phase 5 before 6:** Design tokens are applied via element builder API; builder must exist first.
- **Phase 7 optional:** Transitions are polish; can be deferred to post-MVP if schedule pressure exists.

**Critical path dependencies:** Phase 1 → Phase 2 → Phase 3 → Phase 4 → Phase 5. Phases 6-7 can be parallelized or deferred.

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 2 (Layout & Rendering):** GPU batching strategies, glyphon atlas management (LRU eviction), three-phase rendering lifecycle. Complex integration with niche documentation.
- **Phase 7 (Transitions):** CSS transition semantics in GPU context, property interpolation strategies. Sparse domain-specific resources.

Phases with standard patterns (skip research-phase):
- **Phase 1 (Foundation):** GPUI architecture well-documented via official blog posts and GitHub docs.
- **Phase 3 (Reactive State):** GPUI ownership model and effect queue pattern thoroughly explained in Zed blog posts.
- **Phase 4 (Event System):** Two-phase dispatch is standard DOM pattern, well-understood.
- **Phase 5 (Element Library):** Tailwind builder APIs and styled component patterns established.
- **Phase 6 (Design System):** Token systems and theme switching well-documented in design system literature.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Core stack (wgpu, winit, glyphon, taffy) verified via official GitHub releases and crates.io. Versions confirmed current (Jan 2026). State/animation patterns extrapolated from GPUI architecture analysis. |
| Features | MEDIUM-HIGH | Table stakes validated by existing muda codebase (technologies already integrated). Differentiators based on GPUI patterns (some not yet implemented). Anti-features based on scope creep research and GPUI's explicit non-goals. |
| Architecture | HIGH | GPUI architecture extensively documented via official Zed blog posts (ownership model, 120 FPS optimization). DeepWiki community docs recent (Jan 2026). Three-phase rendering and effect queue patterns verified across multiple sources. |
| Pitfalls | MEDIUM | Critical pitfalls (event loop, reentrancy, batching) validated by multiple sources (official winit docs, GPUI technical articles, existing muda CONCERNS.md). Performance thresholds (draw call counts) are community consensus, not ora-specific profiling. |

**Overall confidence:** HIGH

### Gaps to Address

Research was comprehensive for architectural patterns and technology stack, but implementation details require validation during execution:

- **GPUI Entity/Model implementation specifics:** How exactly does GPUI implement change notifications? Need to review Zed source code for specifics during Phase 3 planning.
- **Text rendering performance at scale:** What's the atlas size needed for 200K line file? Need profiling data during Phase 2 implementation.
- **Transition animation performance:** What's the frame time impact of animating 20 elements simultaneously? Need benchmarking during Phase 7.
- **Focus management edge cases:** How does GPUI handle focus when nested dialogs open? Need deeper dive into focus stack implementation during Phase 4.
- **GPU batching implementation details:** Instanced rendering patterns for 20K+ quads per frame. Need research-phase investigation during Phase 2 planning.
- **glyphon atlas management strategies:** LRU eviction implementation, multi-atlas partitioning for varied font sizes. Need research-phase investigation during Phase 2 planning.

**Mitigation:** These gaps are expected for a new framework implementation. Use `/gsd:research-phase` for Phase 2 (GPU batching, atlas management) and optionally Phase 7 (CSS transitions). Other phases have sufficient documentation to proceed with standard planning.

## Sources

### Primary (HIGH confidence)
- [wgpu v28.0.0 Release](https://github.com/gfx-rs/wgpu/releases) — Latest stable version verified (Dec 2024)
- [winit v0.30.12 Release](https://github.com/rust-windowing/winit/releases) — Latest stable version verified (Jul 2024)
- [glyphon v0.9.0 Release](https://github.com/grovesNL/glyphon/releases) — Latest stable version verified (Jan 2025)
- [taffy v0.9.2 Release](https://github.com/DioxusLabs/taffy/releases) — Latest stable version verified (Nov 2024)
- [GPUI Context Architecture](https://github.com/zed-industries/zed/blob/main/crates/gpui/docs/contexts.md) — Official GPUI docs on App, Context<T>, Entity<T>
- [GPUI Ownership and Data Flow](https://zed.dev/blog/gpui-ownership) — Official Zed blog on centralized ownership pattern
- [GPUI README](https://github.com/zed-industries/zed/blob/main/crates/gpui/README.md) — Official GPUI architecture overview
- [Optimizing the Metal pipeline to maintain 120 FPS in GPUI](https://zed.dev/blog/120fps) — Official Zed blog on rendering optimization

### Secondary (MEDIUM confidence)
- [GPUI Technical Overview by Beck Moulton](https://beckmoulton.medium.com/gpui-a-technical-overview-of-the-high-performance-rust-ui-framework-powering-zed-ac65975cda9f) — GPU acceleration, hybrid immediate/retained mode
- [GPUI Framework DeepWiki](https://deepwiki.com/zed-industries/zed/2.2-gpui-framework) — Element trait, layout pipeline, rendering phases
- [Event Flow DeepWiki](https://deepwiki.com/zed-industries/zed/2.4-keybinding-and-action-dispatch) — Two-phase dispatch, event routing
- [Focus Management DeepWiki](https://deepwiki.com/zed-industries/zed/2.5-keybinding-and-action-system) — Focus paths, dispatch tree
- [Building a GPU-Accelerated Terminal Emulator with Rust and GPUI](https://dev.to/zhiwei_ma_0fc08a668c1eb51/building-a-gpu-accelerated-terminal-emulator-with-rust-and-gpui-4103) — Frame time budgeting, batching
- [Eight million pixels and counting – GUIs on the GPU](https://nical.github.io/drafts/gui-gpu-notes.html) — Draw call batching, compositor, power efficiency
- [EventLoop 3.0 Changes · Issue #2900](https://github.com/rust-windowing/winit/issues/2900) — Event loop ownership architecture
- [Honest Feedback on WGPU · Issue #8010](https://github.com/gfx-rs/wgpu/issues/8010) — wgpu API churn, breaking changes
- Existing muda codebase (PROJECT.md, ARCHITECTURE.md, CONCERNS.md) — Pain points, context, tech debt

### Tertiary (LOW confidence, needs validation)
- [Generational Arenas Guide](https://lucassardois.medium.com/generational-indices-guide-8e3c5f7fd594) — Entity storage patterns (needs verification against slotmap docs)
- [slotmap vs generational-arena Discussion](https://github.com/fitzgen/generational-arena/issues/13) — Performance comparison (community claims, not benchmarked)
- [keyframe crates.io](https://crates.io/crates/keyframe) — Animation library documentation (v1.1.1 verified but usage patterns unverified)
- [mina crates.io](https://crates.io/crates/mina) — CSS-like animation (v0.1.3 too immature, flagged for future evaluation)

---
*Research completed: 2026-01-28*
*Ready for roadmap: yes*
