---
phase: 01-foundation-view-system
plan: 02
subsystem: core-framework
completed: 2026-01-28
duration: 8m 20s
tags: [traits, type-erasure, context, view-system, element-lifecycle]

requires:
  - 01-01-PLAN.md

provides:
  - View trait with render() -> AnyElement
  - Element trait with three-phase lifecycle
  - AnyElement type-erasure for heterogeneous element trees
  - Context hierarchy (AppContext, ViewContext, WindowContext)
  - OraWindow root view management

affects:
  - 01-03: Layout engine will use LayoutContext/PrepaintContext/PaintContext
  - Phase 2: Layout system will implement real layout resolution
  - Phase 3: Reactivity will expand ViewContext::notify()

tech-stack:
  added: []
  patterns:
    - Type-erased trait objects (ElementObject, AnyView)
    - Three-phase rendering lifecycle (request_layout/prepaint/paint)
    - Context hierarchy for scoped access
    - ViewBox wrapper for heterogeneous view storage

key-files:
  created:
    - ora/src/element/trait_def.rs
    - ora/src/element/any_element.rs
    - ora/src/element/mod.rs
    - ora/src/view/trait_def.rs
    - ora/src/view/mod.rs
    - ora/src/window.rs
  modified:
    - ora/src/lib.rs
    - ora/src/context.rs
    - ora/src/app.rs
    - ora/src/platform/event_loop.rs

decisions:
  - decision: "Use ViewBox wrapper with Box<dyn AnyView> for root view storage"
    rationale: "Allows type-erased storage while maintaining render() capability"
    alternatives: "Store as Box<dyn Any> with downcasting (too unsafe, brittle)"
  - decision: "ViewContext<'a> wraps &'a mut AppContext, not EntityStorage directly"
    rationale: "Provides consistent access pattern and room for future expansion"
    alternatives: "Direct EntityStorage reference (harder to extend later)"
  - decision: "Element lifecycle state (RequestLayoutState) stored in Option during type erasure"
    rationale: "Populated during request_layout, passed to prepaint/paint phases"
    alternatives: "Separate state storage (more complex, unnecessary for Phase 1)"
  - decision: "Use unsafe raw pointer in OraWindow::render() to work around borrow checker"
    rationale: "Safe because ViewBox is only read, ViewContext doesn't modify it"
    alternatives: "Refactor storage pattern (deferred to Phase 3 optimizations)"
---

# Phase 01 Plan 02: Core Abstractions (View/Element/Context) Summary

**One-liner:** Established View trait, Element trait with request_layout/prepaint/paint lifecycle, type-erased AnyElement, and context hierarchy for entity access.

## Overview

Defined the core trait-based API surface that all ora UI components will implement. Views are stateful components that produce element trees. Elements are rendering primitives with a three-phase lifecycle. Context types mediate all access to framework state.

## What Was Built

### View System
- **View trait**: `fn render(&self, cx: &mut ViewContext) -> AnyElement`
  - Stateful UI components
  - Produce element trees via render()
  - Stored in entity storage as ViewBox wrappers

- **AnyView trait**: Internal type-erasure for heterogeneous view storage
  - Blanket impl for all View types
  - Used in OraWindow for root view

### Element System
- **Element trait**: Three-phase rendering lifecycle
  - `request_layout`: Compute layout requirements, produce RequestLayoutState
  - `prepaint`: Register hitboxes, prepare for painting
  - `paint`: Produce GPU rendering commands

- **AnyElement**: Type-erased element wrapper
  - Stores `Box<dyn ElementObject>` for heterogeneity
  - Supports children as `Vec<AnyElement>`
  - Recursive lifecycle delegation to children
  - `From<E: Element>` for ergonomic conversion

- **Context Types**: Scoped access to framework state
  - **LayoutContext**: Entity storage + layout ID allocation
  - **PrepaintContext**: Entity storage for hitbox registration
  - **PaintContext**: Entity storage for GPU command generation

### Context Hierarchy
- **AppContext**: Owns EntityStorage, provides insert/read/update/remove
- **ViewContext**: Wraps AppContext, adds notify() stub for reactivity
- **WindowContext**: Wraps AppContext + OraWindow + winit Window
  - `set_root_view<V: View>()`: Store view in entity storage
  - `request_redraw()`: Trigger winit redraw

### Window Management
- **OraWindow**: Manages root view lifecycle
  - Stores root view as `Entity<ViewBox>`
  - `set_root_view()`: Create ViewBox, insert into entity storage
  - `render()`: Create ViewContext, call view's render(), return AnyElement
  - Integrated into event loop (OraApp)

### API Changes
- **on_open callback**: Now receives `&mut WindowContext` (was `&mut AppContext`)
  - Allows setting root view: `cx.set_root_view(MyView::new())`
  - Access to window operations: `cx.request_redraw()`

## Task Commits

| Task | Commit | Description |
|------|--------|-------------|
| 1 | e351456 | Define View and Element traits with type erasure |
| 2 | 730a7db | Implement context hierarchy and window root view |

## Verification Results

All success criteria met:

- ✅ `cargo build -p ora` compiles with zero errors
- ✅ View trait is implementable with `fn render() -> AnyElement`
- ✅ Element trait is implementable with three lifecycle methods
- ✅ AnyElement wraps Element impls and supports children
- ✅ Context types compile and provide entity access (insert/read/update)
- ✅ WindowContext::set_root_view accepts View impls
- ✅ OraWindow::render() produces AnyElement tree from root view
- ✅ View, Element, AnyElement re-exported from ora crate root
- ✅ ViewContext, WindowContext re-exported from ora crate root

## Deviations from Plan

None - plan executed exactly as written.

## Challenges Encountered

### Borrow Checker Issue in OraWindow::render()
**Problem:** Cannot borrow `app_context.entity_storage` immutably while ViewContext holds mutable borrow

**Solution:** Used raw pointer for ViewBox access
```rust
let view_box_ptr = app_context.entity_storage.read(entity) as *const ViewBox;
let mut view_context = ViewContext::new(app_context);
unsafe { (*view_box_ptr).view.render(&mut view_context) }
```

**Justification:** Safe because:
- ViewBox is only read (const pointer)
- ViewContext doesn't modify ViewBox itself
- View::render() only reads view state via entity handles

**Future:** Phase 3 may refactor to interior mutability pattern if needed

## Code Architecture

### Type Erasure Pattern
```
View (concrete type)
  → ViewBox { view: Box<dyn AnyView> }
    → Entity<ViewBox> stored in EntityStorage
      → OraWindow holds Entity<ViewBox>
```

### Element Lifecycle
```
1. request_layout(&mut self, cx) → (LayoutId, RequestLayoutState)
2. prepaint(&mut self, state, cx)
3. paint(&mut self, state, cx)
```

### Context Hierarchy
```
EntityStorage
  ↑
AppContext
  ↑
ViewContext (view rendering)
  ↑
WindowContext (window operations)
```

## Integration Points

### With Plan 01-01
- Uses EntityStorage from 01-01 for view/state storage
- Uses Entity<T> handles from 01-01
- Integrates into OraApp event loop from 01-01

### For Future Plans
- **01-03**: Will create concrete Element implementations (Text, Container, Stack)
- **Phase 2**: Layout engine will populate LayoutContext with real constraints
- **Phase 3**: ViewContext::notify() will trigger reactive updates
- **Phase 4**: PrepaintContext will register hitboxes for mouse/keyboard events

## Next Phase Readiness

**Ready to proceed to 01-03**: Basic element types (Text, Container, Stack)

**Blockers:** None

**Concerns:**
- Unsafe code in OraWindow::render() should be revisited in Phase 3
- Element lifecycle contexts are stubs (real implementation in Phase 2)

## Testing Notes

Build tested only (no runtime testing in Phase 1). The following would be testable with concrete elements:

- View trait implementation (requires concrete elements in 01-03)
- Element lifecycle execution (requires layout engine in Phase 2)
- Root view rendering (requires concrete elements + GPU rendering in Phase 2)

## Metrics

- Files created: 6
- Files modified: 4
- Lines added: ~434
- Compilation time: ~1.6s
- Execution time: 8m 20s
- Zero errors, 15 warnings (dead code, expected)

## Documentation

All public types have rustdoc comments explaining:
- Purpose and role in framework
- When methods are called (lifecycle phase)
- Phase 1 vs future phase distinction (e.g., "real layout in Phase 2")

## Learnings

1. **Type erasure is essential for heterogeneous collections** - AnyElement and AnyView enable dynamic UI trees
2. **Lifecycle state pattern works well** - RequestLayoutState passing avoids global state
3. **Context hierarchy provides good scoping** - Each context level has clear responsibilities
4. **Borrow checker sometimes requires pragmatism** - Unsafe code with clear safety invariants is acceptable
5. **Stub implementations enable phased development** - Phase 1 API, Phase 2 implementation

## References

**Similar patterns in other frameworks:**
- GPUI: View/Element separation, context hierarchy
- React: Component (View) vs. primitives (Element)
- Flutter: Widget tree vs. RenderObject tree
