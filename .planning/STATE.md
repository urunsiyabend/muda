# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-01-28)

**Core value:** A single, authoritative UI toolkit that eliminates duplicated styling, enforces consistent design tokens, and provides a scalable GPUI-like component model for the entire GPU client.
**Current focus:** Phase 4 complete, ready for Phase 5: Element Library

## Current Position

Phase: 6 of 9 (Design System)
Plan: 4 of 5 in current phase
Status: In progress
Last activity: 2026-01-30 - Completed 06-04-PLAN.md (Element Theme Integration)

Progress: [████████████████████░░] 95% Phase 6 (4 of 5 plans complete)

## Performance Metrics

**Velocity:**
- Total plans completed: 27 (Phase 1: 3, Phase 2: 6, Phase 3: 4, Phase 4: 5, Phase 5: 5, Phase 6: 4)
- Average duration: ~7.6m per plan
- Total execution time: ~4 hours 40 minutes

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 01-foundation-view-system | 3 | 23m 0s | 7m 40s |
| 02-layout-rendering-pipeline | 6 | ~55m | ~9m 10s |
| 03-reactive-state-system | 4 | ~21m | ~5m 15s |
| 04-event-system | 5 | ~74m | ~15m |
| 05-element-library | 5 | ~34m | ~6m 48s |
| 06-design-system | 4 | ~42m | ~10m 30s |

**Recent Trend:**
- Phase 6 Design System: **IN PROGRESS** (4 of 5 plans)
- Plan 06-04: Element Theme Integration (24m) - Button uses theme tokens, runtime theme switching demo with 'T' key
- Plan 06-03: Theme Context Integration (7m) - Theme in AppContext, ThemeChanged event, theme() on all contexts
- Plan 06-01: Color Token Foundation (6m 38s) - PaletteColor with 11-step gray scale, Theme struct, 12 semantic ColorToken variants
- Plan 06-02: Spacing and Typography Tokens (4m) - sp() spacing scale, TextSize with bundled font/line-height
- Phase 5 Element Library: **COMPLETE** (5 of 5 plans) ✅
- All unit tests continue passing (15 tests)

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

### Pending Todos

None.

### Blockers/Concerns

- Unsafe code in OraWindow::render() should be revisited (not a blocker, but noted for future refactoring)
- Raw pointer in PaintContext for app_context (follows same pattern, safe during paint phase, but noted for potential refactoring)
- Action and button handlers lack context access - Prevents calling context methods (like set_theme) from user interactions. Workaround: 'T' key handled in event loop for demo. Should be addressed before Phase 7. (06-04)

### Known Issues

- Windows resize flickering: Brief black/white flicker during window resize on Windows is expected wgpu/winit swap chain reconfiguration behavior, not an ora bug (01-03)
- Scissor clipping infrastructure exists (PaintContext push_clip/pop_clip, SetScissor/ResetScissor PaintCommands) but not wired end-to-end: Div doesn't call push_clip for overflow:hidden, and GPU render_frame logs scissor commands instead of applying them (02-06). Will be completed when needed for overflow:hidden use cases.

## Session Continuity

Last session: 2026-01-30
Stopped at: Completed 06-04-PLAN.md (Element Theme Integration)
Resume file: None
Next: Continue Phase 6 (Design System) - Plan 06-05 remaining (final plan in phase).

---
*State initialized: 2026-01-28*
*Last updated: 2026-01-30 after completing Phase 6 Plan 04 (06-04-SUMMARY.md)*
