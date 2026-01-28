# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-01-28)

**Core value:** A single, authoritative UI toolkit that eliminates duplicated styling, enforces consistent design tokens, and provides a scalable GPUI-like component model for the entire GPU client.
**Current focus:** Phase 1: Foundation & View System

## Current Position

Phase: 2 of 9 (Layout & Rendering Pipeline)
Plan: 1 of 5 in current phase
Status: In progress
Last activity: 2026-01-29 - Completed 02-01-PLAN.md

Progress: [███░░░░░░░] ~33%

## Performance Metrics

**Velocity:**
- Total plans completed: 4
- Average duration: 6m 45s
- Total execution time: 0.45 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 01-foundation-view-system | 3 | 23m 0s | 7m 40s |
| 02-layout-rendering-pipeline | 1 | 5m 0s | 5m 0s |

**Recent Trend:**
- Last 5 plans: 01-01 (4m 40s), 01-02 (8m 20s), 01-03 (~10m 0s), 02-01 (5m 0s)
- Trend: Good velocity maintained, Phase 2 started

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

### Pending Todos

None yet.

### Blockers/Concerns

- Unsafe code in OraWindow::render() should be revisited in Phase 3 (not a blocker, but noted for future refactoring)

### Known Issues

- Windows resize flickering: Brief black/white flicker during window resize on Windows is expected wgpu/winit swap chain reconfiguration behavior, not an ora bug (01-03)

## Session Continuity

Last session: 2026-01-29
Stopped at: Completed 02-01-PLAN.md (style type system & layout constraints)
Resume file: None
Next: Plan 02-02 (Flexbox Layout Engine)

---
*State initialized: 2026-01-28*
*Last updated: 2026-01-29 after 02-01 completion*
