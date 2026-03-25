# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-01-28)

**Core value:** A single, authoritative UI toolkit that eliminates duplicated styling, enforces consistent design tokens, and provides a scalable GPUI-like component model for the entire GPU client.
**Current focus:** Phase 9: Transitions & Integration — in progress (09-01 animation primitives done, 09-02 editor adapter done).

## Current Position

Phase: 9 of 9 (Transitions & Integration) - In progress
Plan: 1 of N in current phase (09-01 SUMMARY complete; 09-02 pre-exists on branch)
Status: In progress — 09-01 (animation primitives) SUMMARY created; 09-02 (EditorDataSource + mirror types) also on branch
Last activity: 2026-03-25 - Completed 09-01-PLAN.md (animation primitives: Easing, CubicBezier, Spring, Tween<T>, Tweenable)

Progress: [█████████████████████████████████] Phase 9 in progress (~2 plans done)

## Performance Metrics

**Velocity:**
- Total plans completed: 40 (Phase 1: 3, Phase 2: 6, Phase 3: 4, Phase 4: 5, Phase 5: 5, Phase 6: 4, Phase 7: 5, Phase 8: 8)
- Average duration: ~7.5m per plan
- Total execution time: ~5 hours 34 minutes

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 01-foundation-view-system | 3 | 23m 0s | 7m 40s |
| 02-layout-rendering-pipeline | 6 | ~55m | ~9m 10s |
| 03-reactive-state-system | 4 | ~21m | ~5m 15s |
| 04-event-system | 5 | ~74m | ~15m |
| 05-element-library | 5 | ~34m | ~6m 48s |
| 06-design-system | 4 | ~42m | ~10m 30s |
| 07-editor-chrome-text-editing | 5 | ~54m | ~11m |
| 08-advanced-ui-widgets | 8 | ~60m | ~7.5m |

**Recent Trend:**
- Phase 8.1 GPU Layered Rendering Pipeline: **COMPLETE** (3 of 3 plans)
  - Plan 08.1-01: RectangleRenderer lifetime fix + render_range() (~1m)
  - Plan 08.1-02: Multi-layer TextSystem API (ensure_layer_renderers/add_text_to_layer/prepare_layer/render_layer) (~1m)
  - Plan 08.1-03: Layered render_frame() integration, visual UAT approved (~2m)
- Phase 8 Advanced UI & Widgets: **COMPLETE** (8 of 8 plans)
- Plan 08-08: AppLayout capstone (~5m) - Stack-based overlay composition, all views orchestrated
- Plan 08-07: ContextMenu + Toast overlays (~8m) - Padding-positioned context menu, auto-dismiss toasts
- Plan 08-06: PanelManagerView (~7m) - Resizable bottom panel with 4 tabs and preset cycling
- Plan 08-05: FileTreeView + SidebarView integration (~8m) - Hierarchical file tree with indent guides
- Plan 08-04: CommandPaletteView (~8m) - Fuzzy search with match highlighting and keyboard navigation
- Plan 08-03: Checkbox + Toggle (~7m) - CheckboxSize local enum, always-paint pattern for glyphon
- Plan 08-02: Navigation Widgets (~7m) - Tab, ListItem, TreeItem with ASCII chevrons
- Plan 08-01: Input + WidgetSize (~10m) - Input element with focus/hover/disabled states
- All unit tests passing: 110 in ora, 103 in core_editor
- Note: wgpu_client token migration deferred to Phase 9 (INT-04, INT-05)
- Note: CheckboxSize local enum remains in checkbox.rs (unification with WidgetSize deferred to Phase 9)

*Updated after each plan completion*

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- GPUI-style views (trait-based, render() returns element tree) - Matches Zed's proven model for editor UIs
- Entity/Model reactive state - Decouples state ownership from views
- Ora owns winit event loop - Simplifies app lifecycle
- Ora wraps glyphon internally - Views shouldn't manage text atlases
- Simple stack/flex layout (NOT taffy) - User chose simple layout for v1
- Use generational-arena for entity storage - Provides stable handles with generation checking (01-01)
- Pollster::block_on for GPU initialization - Simple blocking async for startup (01-01)
- Zero-size window guard in resize - Prevents wgpu panic on minimize (01-01)
- ViewBox wrapper for root view storage - Type-erased Box<dyn AnyView> with Entity handle (01-02)
- Three-phase element lifecycle - request_layout/prepaint/paint for rendering pipeline (01-02)
- ViewContext wraps AppContext - Consistent access pattern, room for future expansion (01-02)
- Unsafe raw pointer in OraWindow::render - Safe workaround for borrow checker, to be revisited in Phase 3 (01-02)
- PaintCommand collection pattern - Elements produce commands, decoupled from GPU (01-03)
- Stub clear-color rendering - Phase 1 proves architecture without vertex buffer complexity (01-03)
- RedrawRequested drives lifecycle - Natural per-frame entry point for rendering work (01-03)
- Stub LayoutContext - Sequential ID allocation, real constraint solving deferred to Phase 2 (01-03)
- flex_shrink defaults to 1.0 (CSS default) - Matches standard flexbox behavior, children shrink to fit (02-01)
- Per-corner border-radius (Corners<f32>) - Each corner independently styled for complex UI patterns (02-01)
- Per-side border widths (Edges<f32>) - Fine-grained border control matching CSS model (02-01)
- RGBA floats for Color - 0.0..1.0 range matches GPU shaders, eliminates conversion overhead (02-01)
- AvailableSpace enum (Definite/MinContent/MaxContent) - Supports three-pass flexbox algorithm (02-01)
- RectInstance 160-byte struct (16-byte aligned) - Storage buffer compatibility for instanced rendering (02-03)
- Storage buffers for instance data - Supports 128 MiB (vs 64 KiB uniform limit) for batching 1000+ rectangles (02-03)
- Shader-based quad generation - vertex_index generates geometry, no vertex buffer needed (02-03)
- Distance field SDF for rounded corners - Per-corner radius selection in fragment shader (02-03)
- Closed-form Gaussian shadows - Error function approximation for single-pass rendering (02-03)
- Downgraded glyphon to 0.7 for wgpu 23 compatibility - glyphon 0.9 requires wgpu 25 (02-04)
- Framework owns all font resources - Elements don't create FontSystem instances, just provide text content (02-04)
- Buffer reuse between measure and paint - Same Buffer from measure_text() must be used in add_text_area() (02-04)
- Div children stored internally in Vec<AnyElement> - Elements manage their own child collections (02-05)
- TextState stores Buffer in Option - Taken during paint to move into PaintCommand::Text (02-05)
- PaintCommand::Text carries glyphon::Buffer directly - Simplifies rendering pipeline coordination (02-05)
- LayoutContext uses raw pointer to TextSystem - Necessary for measure_text() integration with borrow checker (02-05)
- Flexbox stretch checks Length::Auto (not resolved zero) - CSS stretch applies to auto-sized children regardless of content measurement (02-06)
- gradient_end_color == color for solid backgrounds - Prevents false gradient detection in shader (02-06)
- Viewport must be updated every frame - glyphon requires Resolution update before prepare() (02-06)
- Distance-to-each-edge border detection - Robust per-side border width selection in shader (02-06)
- EffectQueue uses VecDeque for priority lanes (notify before emit) - Ensures observers see consistent state before events propagate (03-01)
- Notify effects deduplicated via HashSet, emit effects are not - Multiple notify calls to same entity = one effect (03-01)
- ModelContext buffers effects locally, drains to AppContext after closure - Avoids aliased mutable borrows (03-01)
- Update depth tracking ensures flush only at top level (depth == 0) - Batches nested updates, prevents reentrancy (03-01)
- Model<T> wraps Entity<T> - Reactive entities distinguished from plain entities (03-01)
- Subscription.detach() pattern for view-lifetime subscriptions - Full Drop cleanup via thread-local queue (03-02)
- Borrow-safe observer invocation via take_observers_for/restore_observers - Avoids aliased mutable borrows with &mut AppContext (03-02)
- AppContext persists across frames in OraApp - Critical change: observer_set and effect_queue must survive frames for reactivity (03-02)
- Cascade depth limit of 10 with warning at 5 - Prevents infinite loops in observer chains (03-02)
- Dirty entities tracked in HashSet - Foundation for future view re-render optimization (03-02)
- Thread-local PENDING_CLEANUPS for Subscription Drop - CleanupAction enum avoids AppContext access during Drop (03-03)
- SubscriberSet nested HashMap (EntityId -> TypeId -> Vec) - Efficient typed event subscription registry (03-03)
- GlobalEventBus separate from SubscriberSet - App-wide events without emitter entity (03-03)
- global_emit_queue in EffectQueue - Separate queue for global events avoids sentinel entity ID (03-03)
- Type-erased event dispatch with downcast_ref - Subscribers receive &dyn Any and downcast to concrete types (03-03)
- LocalExecutor<'static> for main-thread async - Spawned futures MUST NOT block (03-04)
- Executor ticked after events AND in about_to_wait - Ensures async work progresses even without input (03-04)
- Context forwarding to AppContext - ViewContext/WindowContext delegate Model ops to AppContext (03-04)
- TextElement.size() auto-scales line_height = font_size * 1.2 - CSS standard ratio prevents text clipping (03-04)
- FocusHandle with Arc reference counting - Survives re-renders, stable identity across frames (04-02)
- Input modality determines focus ring visibility - FocusSource enum (Keyboard vs Mouse vs Programmatic) (04-02)
- PrepaintContext collects focus order - register_focusable() builds tab order during tree traversal (04-02)
- Action trait with boxed_clone() for type erasure - Enables typed action dispatch without Clone requirement (04-04)
- Last-wins keymap conflict resolution - Iterate bindings in reverse, allows overriding defaults (04-04)
- Tab navigation before action matching - Ensures fundamental focus navigation always works (04-04)
- Modifiers with Eq and Hash - Required for Keystroke hashability in keymap lookups (04-04)
- InteractionState in AppContext alongside FocusState - Centralized hover/active tracking (04-05)
- Hover/active path includes ancestors - Enables parent elements to query child interaction (04-05)
- MouseCapture for drag operations - Drag continues outside element bounds until mouse up (04-05)
- Interactive styling in Div (hover_bg/active_bg/focus_ring) - Elements query state during paint (04-05)
- TextElement opaque:false for non-interactive hitboxes - Text doesn't capture mouse events (04-05)
- Persistent FocusHandles stored in view - Created once, not every render (prevents ID explosion) (04-05)
- PaintContext with InteractionState and FocusState - Elements query interaction during paint (04-05)
- px() and pct() constructor functions for explicit Length specification - Tailwind-style fluent API (05-01)
- From<f32> and From<i32> for implicit pixel values - Raw numbers default to pixels for ergonomics (05-01)
- Removed w_pct/h_pct in favor of w(pct(50)) syntax - Cleaner unified API (05-01)
- CSS-style alignment methods (justify/items/row/column) - More flexible than convenience-only approach (05-01)
- Impl Into<Length> for sizing methods - Enables flexible unit specification in builder (05-01)
- Composite elements store child elements internally - Button stores TextElement for lifecycle control, establishes pattern for complex elements (05-03)
- ButtonVariant state-based styling - variant.style(state) lookup pattern for semantic UI components (05-03)
- Interaction state priority: disabled > active > hover > enabled - Standard hierarchy for button state rendering (05-03)
- image = "0.24" version for dependency compatibility - Version 0.25 requires unavailable gif 0.14 (05-04)
- LRU-based texture eviction - Prevents unbounded GPU memory growth with automatic cleanup (05-04)
- ObjectFit CSS model (Contain/Cover/Fill) - Familiar web-style image scaling semantics (05-04)
- Atomic TextureId generation - Thread-safe monotonic IDs without locking (05-04)
- Placeholder color for loading images - Dark gray default, customizable via builder (05-04)
- 11-step gray scale (50-950) with soft neutrals - Comfortable for extended dark mode use, not harsh contrast (06-01)
- Semantic color tokens map to palette - BgPrimary/FgSecondary resolve to gray_900/blue_500 based on theme mode (06-01)
- Theme defaults to dark mode - ThemeMode::Dark as default variant (06-01)
- theme.color() primary API, theme.palette() escape hatch - 90% semantic, 10% direct palette access (06-01)
- sp() spacing scale with 4px base - Tailwind-style spacing, supports negative values for overlap (06-02)
- TextSize bundles font_size and line_height - Prevents mismatched font/line-height pairs (06-02)
- Dual naming for TextSize - Semantic names (Body, Small) and scale aliases (Sm, Xs) (06-02)
- FontFamily enum distinguishes UI vs Code fonts - Automatic suggestion from TextSize::Code (06-02)
- Theme stored in AppContext with dark mode default - Centralizes theme for global access, AppContext::new() initializes Theme::dark() (06-03)
- Raw pointer for app_context in PaintContext - Necessary to avoid borrow checker aliasing (immutable app_context + mutable entity_storage) (06-03)
- ThemeChanged global event for theme switching - Simple marker event, subscribers query theme() directly (06-03)
- Elements access theme during paint phase only - LayoutContext lacks theme access, elements call cx.theme() in paint() (06-04)
- ButtonVariant::style() accepts &Theme parameter - Enables variant color computation based on current theme (06-04)
- TextElement::set_color() for paint-time updates - Allows dynamic text color changes after layout measurement (06-04)
- Demo 'T' key handler in event loop - Temporary workaround for theme toggle until action handlers receive context (06-04)
- Views consume presentation data, own no rendering state (no FontSystem/TextAtlas) - GPUI pattern for ora views (07-01)
- Borrow-safe render pattern: complete mutable cx operations before getting theme reference - Avoids aliased borrows (07-01)
- TAB_BAR_HEIGHT (28.0) and STATUS_BAR_HEIGHT (24.0) as exported constants - Consistent heights across layout (07-01)
- Sidebar collapsed state shows 48px icon rail (not completely hidden) - CONTEXT decision for expand affordance (07-02)
- Sidebar toggle via dedicated button (not header click) - CONTEXT decision for explicit control (07-02)
- Gutter current line highlight in text color only (not background) - CONTEXT decision, parent layout handles background (07-02)
- GutterView dynamic width: LEFT_PADDING + digits*char_width + SEPARATOR_PADDING - Adapts to document size (07-02)
- Stack w/h accept impl Into<Length> - Enables pct(100.0) for full-screen modal overlays (07-03)
- ColorToken::Warning for unsaved changes UI - Amber palette (amber_500/600) for warning indicators (07-03)
- Backdrop always raw Color (not token) - Semi-transparent black at 60% is constant across themes (07-03)
- Dialog buttons right-aligned per CONTEXT - justify_end() for Save/Don't Save/Cancel button row (07-03)
- BLINK_RATE=500ms, ACTIVITY_TIMEOUT=500ms for CaretElement - WCAG-safe blink rate, stays solid during typing (07-04)
- CARET_WIDTH=2px thin beam style - Per CONTEXT decision for caret appearance (07-04)
- Syntax highlighting via ColorToken mapping - TextStyle->ColorToken for consistent theme-aware syntax colors (07-04)
- Selection backgrounds solid (ColorToken::Selection) - Per CONTEXT decision, not semi-transparent (07-04)
- LINE_HEIGHT=21.0 as exported constant - Consistent line height across text area and gutter (07-04)
- TAB_BAR_HEIGHT=36px (increased from 28px) for readability (07-05)
- STATUS_BAR_HEIGHT=28px (increased from 24px) for readability (07-05)
- SIDEBAR_DEFAULT_WIDTH=260px (increased from 220px) for readability (07-05)
- IDE Layout: Tab bar inside main area (after sidebar), not spanning full width (07-05)
- Tab close button (x) with hover highlight - proof of concept without icon system (07-05)
- Sidebar toggle uses Div with always-visible ASCII arrows (< or >) (07-05)
- Current line highlight via per-line bg color check (is_current_line) (07-05)
- overflow_hidden on text area/editor for clipping intent (scissor not GPU-rendered) (07-05)
- WidgetSize enum in button.rs (not a separate file) - co-located with first consumer, imported by all others (08-01)
- Input uses WidgetSize.height() as min_height (not fixed height) - allows content to expand (08-01)
- Input text color: FgMuted for placeholder, FgPrimary for value - standard placeholder convention (08-01)
- Input border: Accent when focused, FgMuted when hovered, Border when normal, BgSecondary when disabled (08-01)
- Disabled alpha: 0.5 on bg and text - matches Button disabled pattern (08-01)
- TAB_HEIGHT=36px matches Phase 7 TAB_BAR_HEIGHT constant — tabs keep their established height (08-02)
- ListItem uses low-alpha Accent (0.15) for selected state — distinguishable without harsh contrast (08-02)
- TreeItem chevron uses 'v'/'>' ASCII — no icon system dependency (08-02)
- Indent guides painted as 1px rects (not borders) — simpler and GPU-efficient (08-02)
- Color extraction pattern: all cx.theme().color() calls before any mutable cx.paint_* calls — required by Rust borrow checker (08-02)
- CheckboxSize local enum (not WidgetSize) for wave isolation — Plan 08-08 will unify with WidgetSize (08-03)
- Checkbox box sizes: Sm=14px, Md=18px, Lg=22px — smaller than button heights, checkbox-specific (08-03)
- Toggle has no label field — users compose toggle() with TextElement for labeling (08-03)
- Always-paint pattern: child TextElement paint() called even when invisible (transparent color) to consume glyphon Buffer (08-03)
- Disabled dimming: multiply all Color.a by 0.5 — consistent 50% dimming across checkbox and toggle (08-03)
- AppLayout::new() accepts all child views as constructor args; Toast/ContextMenu initialized empty — populated at runtime (08-08)
- Dialog layer always included in AppLayout Stack unconditionally — DialogView renders 0-size div when None (08-08)
- ContextMenu positioned via padding wrapper (pt/pl on full-screen Div) in Stack overlay — no absolute CSS positioning needed (08-08)
- CheckboxSize unification deferred: local CheckboxSize enum remains in checkbox.rs, not unified with WidgetSize (08-08)
- RenderPass<'_> (elided lifetime) for render() — breaks lifetime coupling with self, safe because GPU objects live on GpuState outliving all render passes (08.1-01)
- Sorted single-batch approach for rect layers: one prepare() upload with draw range slicing per layer — avoids bind_group invalidation seen in prior multi-pass attempts (08.1-01)
- instance_count() getter returns u32 to match wgpu draw() API Range<u32> parameter type directly (08.1-01)
- render_layer() uses RenderPass<'_> unconstrained lifetime matching glyphon's own render() signature — enables use in multi-pass loops (08.1-02)
- TextRenderer pool grows on demand, never shrinks — TextRenderer::new() is GPU work (pipeline creation), reuse is essential (08.1-02)
- All layer prepares before any renders: ensures TextAtlas stabilizes before render phase begins (08.1-02)
- Flat rect buffer with Range<u32> range slicing per layer in render_frame() — single prepare() avoids bind_group invalidation (08.1-03)
- First render pass LoadOp::Clear, subsequent passes LoadOp::Load — Vulkan requires Clear on uninitialized texture; Load preserves prior layer content for correct occlusion (08.1-03)
- Single queue.submit() for entire multi-pass frame — Vulkan multiple submits cause hangs (08.1-03)
- Easing enum default is EaseOut — matches wgpu_client convention, most common for UI transitions (09-01)
- Duration/MotionPreference tokens not ported to ora::animation — TransitionConfig uses u32 ms directly (09-01)
- Tweenable trait uses lerp(&self, &Self, f32) -> Self — owned return type matches Tween<T> requirements (09-01)
- Spring zeta critically-damped branch uses epsilon check, not exact == 1.0 — avoids f32 edge case (09-01)
- Mirror types in ora::editor_adapter have no From impls — conversions deferred to wgpu_client where both type namespaces are available (09-02)
- EditorCommand defined alongside presentation types in editor_adapter::types — single module owns full adapter surface (09-02)
- Adapter boundary: ora views use &dyn EditorDataSource, never &core_editor::app::App (09-02)

### Pending Todos

None.

### Roadmap Evolution

- Phase 8.1 inserted after Phase 8: GPU Layered Rendering Pipeline — fix overlay z-ordering so command palette/dialog/toast correctly occlude lower-layer text (URGENT)
  - Discovered during Phase 8 UAT: single-pass renderer draws all rects then all text, causing editor text to bleed through overlay backgrounds
  - Prior fix attempts (multi-pass, two-pass) introduced rendering artifacts (sidebar content loss, empty overlays)
  - Requires focused research into wgpu render pass lifecycle and glyphon multi-prepare constraints

### Blockers/Concerns

- Unsafe code in OraWindow::render() should be revisited (not a blocker, but noted for future refactoring)
- Raw pointer in PaintContext for app_context (follows same pattern, safe during paint phase, but noted for potential refactoring)
- Action and button handlers lack context access - Prevents calling context methods (like set_theme) from user interactions. Workaround: 'T' key handled in event loop for demo. Should be addressed before Phase 7. (06-04)

### Known Issues

- Windows resize flickering: Brief black/white flicker during window resize on Windows is expected wgpu/winit swap chain reconfiguration behavior, not an ora bug (01-03)
- Scissor clipping infrastructure exists (PaintContext push_clip/pop_clip, SetScissor/ResetScissor PaintCommands) but not wired end-to-end: Div doesn't call push_clip for overflow:hidden, and GPU render_frame logs scissor commands instead of applying them (02-06). Will be completed when needed for overflow:hidden use cases.

## Session Continuity

Last session: 2026-03-25T11:27:16Z
Stopped at: Completed 09-01-PLAN.md — animation primitives (Easing, CubicBezier, Spring, Tween<T>, Tweenable) in ora::animation
Resume file: None
Next: Continue Phase 9 — 09-02 (EditorDataSource + mirror types) already on branch; proceed to next unexecuted plan

---
*State initialized: 2026-01-28*
*Last updated: 2026-03-25 after completing 09-01 (animation primitives port, Tween/Easing/CubicBezier/Spring)*
