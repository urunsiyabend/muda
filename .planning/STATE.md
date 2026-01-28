# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-01-28)

**Core value:** A single, authoritative UI toolkit that eliminates duplicated styling, enforces consistent design tokens, and provides a scalable GPUI-like component model for the entire GPU client.
**Current focus:** Phase 2 complete, ready for Phase 3: Reactive State System

## Current Position

Phase: 3 of 9 (Reactive State System) -- IN PROGRESS
Plan: 1 of 4 in current phase (plan 03-01 complete)
Status: Plan 03-01 complete
Last activity: 2026-01-29 - Completed 03-01-PLAN.md (Reactive State Foundation)

Progress: [█████████████▓░░] 75% Phase 3 (1 of 4 plans complete)

## Performance Metrics

**Velocity:**
- Total plans completed: 10 (Phase 1: 3, Phase 2: 6, Phase 3: 1)
- Average duration: ~7m per plan
- Total execution time: ~1.6 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 01-foundation-view-system | 3 | 23m 0s | 7m 40s |
| 02-layout-rendering-pipeline | 6 | ~55m | ~9m 10s |
| 03-reactive-state-system | 1 | 4m | 4m |

**Recent Trend:**
- Phase 3 Plan 01 executed smoothly with no deviations or issues
- All 11 existing unit tests continue passing
- Clean architecture with local effect buffering prevented borrow checker conflicts

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

### Pending Todos

None yet.

### Blockers/Concerns

- Unsafe code in OraWindow::render() should be revisited in Phase 3 (not a blocker, but noted for future refactoring)

### Known Issues

- Windows resize flickering: Brief black/white flicker during window resize on Windows is expected wgpu/winit swap chain reconfiguration behavior, not an ora bug (01-03)
- Scissor clipping infrastructure exists (PaintContext push_clip/pop_clip, SetScissor/ResetScissor PaintCommands) but not wired end-to-end: Div doesn't call push_clip for overflow:hidden, and GPU render_frame logs scissor commands instead of applying them (02-06). Will be completed when needed for overflow:hidden use cases.

## Session Continuity

Last session: 2026-01-29
Stopped at: Completed 03-01-PLAN.md
Resume file: None
Next: Plan 03-02 (Observer/Subscription System)

---
*State initialized: 2026-01-28*
*Last updated: 2026-01-29 after completing Plan 03-01 (Reactive State Foundation)*
