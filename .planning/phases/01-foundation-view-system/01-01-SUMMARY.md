---
phase: 01-foundation-view-system
plan: 01
subsystem: core-infrastructure
tags: [wgpu, winit, generational-arena, entity-system, app-lifecycle]

requires: []
provides:
  - ora-crate-foundation
  - wgpu-rendering-surface
  - entity-storage-system
  - app-builder-api
affects:
  - 01-02 # Will use App and entity system for view rendering
  - 01-03 # Will use entity system for layout state
  - 02-01 # Will extend entity system for reactive models

tech-stack:
  added:
    - winit: "0.30"
    - wgpu: "23"
    - generational-arena: "0.2"
    - pollster: "0.4"
  patterns:
    - Builder pattern (App configuration)
    - Generational arena (type-safe entity storage)
    - ApplicationHandler (winit event loop ownership)

key-files:
  created:
    - ora/Cargo.toml
    - ora/src/lib.rs
    - ora/src/app.rs
    - ora/src/context.rs
    - ora/src/platform/mod.rs
    - ora/src/platform/gpu.rs
    - ora/src/platform/event_loop.rs
    - ora/src/entity/mod.rs
    - ora/src/entity/handle.rs
    - ora/src/entity/storage.rs
    - ora/examples/hello.rs
  modified:
    - Cargo.toml (workspace members)
    - Cargo.lock (dependencies)

key-decisions:
  - decision: "Use generational-arena for entity storage"
    rationale: "Provides stable handles with generation checking, prevents dangling references"
    alternatives: "slotmap, simple Vec with generations"
  - decision: "Ora owns winit event loop via ApplicationHandler"
    rationale: "Simplifies app lifecycle, matches modern winit 0.30 patterns"
    alternatives: "User controls event loop"
  - decision: "Use pollster::block_on for GPU initialization"
    rationale: "Simple blocking async for startup, avoids async runtime complexity"
    alternatives: "tokio/async-std runtime, manual futures"
  - decision: "Zero-size window guard in resize"
    rationale: "Prevents wgpu panic on window minimize/resize to 0x0"
    alternatives: "Allow panic, skip rendering"

duration: 280s
completed: 2026-01-28
---

# Phase 1 Plan 1: Foundation Infrastructure Summary

**One-liner:** Created ora crate with App builder, wgpu rendering surface, winit event loop ownership, and generational arena entity storage with typed Entity<T> handles.

## Performance

- **Duration:** 4 minutes 40 seconds
- **Start:** 2026-01-28 (epoch: 1769626720)
- **End:** 2026-01-28 (epoch: 1769627000)
- **Tasks completed:** 2/2
- **Files created:** 11
- **Files modified:** 2
- **Commits:** 1

## Accomplishments

### 1. Ora Crate Infrastructure
- Added `ora` as workspace member
- Set up crate structure with app, entity, platform modules
- Configured dependencies matching wgpu_client versions (winit 0.30, wgpu 23)
- Created hello example demonstrating basic window lifecycle

### 2. App Builder API
- Implemented `App` struct with builder pattern
- Methods: `new()`, `title()`, `size()`, `on_open()`, `run()`
- Default configuration: "ora" title, 800x600 size
- `on_open` callback receives `AppContext` for entity access
- `run()` method enters event loop and never returns (uses std::process::exit)

### 3. wgpu Rendering Surface
- `GpuState` struct encapsulates wgpu Instance, Device, Queue, Surface
- Async initialization via `pollster::block_on` in ApplicationHandler::resumed()
- Surface configuration with vsync (PresentMode::Fifo)
- Automatic sRGB format selection with fallback
- Zero-size guard prevents panic on window minimize (uses max(1, size))
- Stub render() method clears to dark background (r: 0.1, g: 0.1, b: 0.12)

### 4. Winit Event Loop Ownership
- `OraApp` implements `ApplicationHandler` for winit 0.30
- Handles resumed, window_event (Resized, CloseRequested, RedrawRequested)
- Surface reconfiguration on resize
- Continuous redraw loop for animation-ready rendering
- Surface error handling (Lost → reconfigure, OutOfMemory → exit)

### 5. Entity Storage System
- `EntityStorage` provides type-erased generational arenas per type
- `Entity<T>` typed handles with PhantomData for compile-time safety
- API: `insert()`, `read()`, `read_opt()`, `update()`, `remove()`
- Panics on dangling handles with descriptive error messages
- `AppContext` wraps EntityStorage for on_open callback access

## Task Commits

| Task | Description | Commit | Key Files |
|------|-------------|--------|-----------|
| 1-2 | Create ora crate with App builder, wgpu lifecycle, and entity storage | 5f2a0e6 | ora/src/app.rs, ora/src/platform/gpu.rs, ora/src/platform/event_loop.rs, ora/src/entity/storage.rs, ora/examples/hello.rs |

## Files Created/Modified

### Created
1. `ora/Cargo.toml` - Crate manifest with winit 0.30, wgpu 23, generational-arena 0.2
2. `ora/src/lib.rs` - Crate root re-exporting App, AppContext, run()
3. `ora/src/app.rs` - App builder struct with title/size/on_open/run methods
4. `ora/src/context.rs` - AppContext wrapping EntityStorage for callbacks
5. `ora/src/platform/mod.rs` - Platform module re-exports
6. `ora/src/platform/gpu.rs` - GpuState with wgpu resource management
7. `ora/src/platform/event_loop.rs` - OraApp ApplicationHandler implementation
8. `ora/src/entity/mod.rs` - Entity module re-exports
9. `ora/src/entity/handle.rs` - Entity<T> typed handle with PhantomData
10. `ora/src/entity/storage.rs` - EntityStorage with generational arenas
11. `ora/examples/hello.rs` - Minimal example opening a window

### Modified
1. `Cargo.toml` - Added ora to workspace members
2. `Cargo.lock` - Updated with ora dependencies

## Decisions Made

### 1. Generational Arena Entity Storage
**Decision:** Use `generational-arena` crate for entity storage with typed `Entity<T>` handles.

**Rationale:**
- Provides stable handles that survive entity removal
- Generation checking prevents use-after-free bugs
- Type-safe access via PhantomData
- Simple, proven design for entity-component systems

**Alternatives considered:**
- `slotmap`: Similar but different API
- Custom Vec-based storage: More work, less battle-tested
- Direct HashMap with IDs: No generation checking, easier to create bugs

**Impact:** All future systems (views, models, layouts) will use this entity pattern.

### 2. Ora Owns Event Loop
**Decision:** Ora takes full ownership of winit event loop via ApplicationHandler.

**Rationale:**
- Matches winit 0.30 best practices (ApplicationHandler trait)
- Simplifies user API - just call `App::run()` and done
- Enables future extensions (multiple windows, async events)
- Consistent with GPUI's ownership model

**Alternatives considered:**
- User controls event loop: More flexible but complex API
- Callback-based: Less idiomatic for modern winit

**Impact:** Users cannot directly access the event loop, but Ora provides all necessary hooks.

### 3. Pollster for GPU Initialization
**Decision:** Use `pollster::block_on()` to initialize wgpu asynchronously in resumed().

**Rationale:**
- Simple blocking async for one-time startup
- No need for full async runtime (tokio/async-std)
- wgpu initialization is fundamentally async (adapter/device requests)
- Blocking on startup is acceptable UX

**Alternatives considered:**
- Full async runtime: Overkill for this use case
- Manual future polling: Complex and error-prone

**Impact:** Startup is slightly blocking but simple. Future async features may need tokio.

### 4. Zero-Size Window Guard
**Decision:** Guard resize() to reject width=0 or height=0, using max(1, size) on init.

**Rationale:**
- wgpu panics on zero-size surface configuration
- Windows can minimize to 0x0 on some platforms
- Better to skip reconfigure than crash

**Alternatives considered:**
- Allow panic: Bad UX
- Skip rendering but allow surface: More complex state management

**Impact:** Window can minimize safely without crashes.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Combined Task 1 and Task 2 into single commit**
- **Found during:** Task execution
- **Issue:** Tasks 1 and 2 are tightly coupled - entity system is referenced in app.rs context, and splitting would create intermediate broken state
- **Fix:** Implemented all files from both tasks together and created single atomic commit
- **Files modified:** All files from both tasks
- **Commit:** 5f2a0e6
- **Rationale:** Atomic commits should be compilable. Splitting would leave crate in non-compiling state between commits.

## Issues Encountered

None. Build completed cleanly with zero errors and zero warnings.

## Next Phase Readiness

### Blockers
None identified.

### Concerns
None. Foundation is solid for view system implementation.

### Prerequisites for 01-02 (View System Core)
- ✅ ora crate compiles and runs
- ✅ wgpu surface renders successfully
- ✅ Entity<T> storage system operational
- ✅ App builder API working
- ✅ Event loop handles resize/close/redraw

### Recommendations for 01-02
1. Build View trait on top of Entity system established here
2. Use GpuState from platform::gpu for render pass initialization
3. Extend AppContext to WindowContext with view tree access
4. Consider adding render state (vertex buffers, pipelines) to GpuState

## Lessons Learned

1. **Generational arenas are perfect for UI entity management** - Type-safe handles with automatic dangling detection make entity lifecycle trivial.

2. **winit 0.30 ApplicationHandler is clean** - Much better than old event loop callback style. Ownership is clear.

3. **pollster::block_on is sufficient for startup** - No need to overcomplicate with full async runtime for simple initialization.

4. **Zero-size window guards are critical** - Easy to forget but prevents confusing crashes on minimize.

## Future Considerations

1. **Multiple windows**: ApplicationHandler supports this, but current design assumes single window. May need window ID tracking in future.

2. **Async rendering**: Currently blocking render loop. Future may want async frame scheduling.

3. **Entity system extensions**: May need entity relationships, queries, or component iteration for complex view trees.

4. **GPU resource management**: GpuState is minimal. Future plans may need texture/buffer pools, shader compilation caching.

## References

- Plan: `.planning/phases/01-foundation-view-system/01-01-PLAN.md`
- Commit: `5f2a0e6`
- Example: `ora/examples/hello.rs`
- Next: Plan 01-02 (View System Core)
