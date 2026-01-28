# Phase 1: Foundation & View System - Context

**Gathered:** 2026-01-28
**Status:** Ready for planning

<domain>
## Phase Boundary

Establish the ora crate with application lifecycle ownership (winit event loop, wgpu surface), centralized entity storage with typed handles, and a declarative view/element model with three-phase lifecycle. This phase delivers the skeleton that all subsequent phases build on. No real layout engine, no text rendering, no reactivity — just the structural foundation.

</domain>

<decisions>
## Implementation Decisions

### App entry & window setup
- `ora::run(app)` takes full ownership of the winit event loop — host hands off entirely
- Single window only for v1 (multi-window can be added later without API breakage)
- Builder pattern for App configuration: `App::new().title("muda").size(800, 600).on_open(|cx| { ... }).run()`
- Ora creates wgpu instance, adapter, device, queue, surface internally — fully self-contained, host doesn't touch wgpu
- Root view registered via `on_open` callback: `cx.set_root_view(MyRootView::new(cx))`

### Entity/Model storage API
- Generational arena storage — typed arenas per type T, Entity<T> is (index, generation)
- Context-mediated borrowing: `entity.read(cx)` / `entity.update(cx, |val, cx| { ... })`
- Two separate types: `Entity<T>` for plain storage, `Model<T>` wraps Entity and adds observation capabilities (Phase 3)
- Dangling handles panic on access — fail-fast with generation mismatch detection

### View trait contract
- `render(&self, cx: &mut ViewContext) -> AnyElement` — type-erased, boxed return
- Every View is implicitly backed by `Model<Self>` — always stateful, no stateless views
- Entity reference composition: parent holds `Entity<ChildView>`, renders child via context method
- Root view registered in `on_open` callback, not via generic type parameter

### Element lifecycle phases
- Elements can have optional persistent state via `BeforeLayout -> State` associated type, framework-cached between frames (like GPUI)
- Children declared as `Vec<AnyElement>` — uniform, type-erased collection
- `request_layout` calls `cx.request_layout(style, children_ids)` and receives a `LayoutId` back — layout engine owns constraint data
- Phase 1 paint produces stub rendering (colored rectangles via basic wgpu) to visually verify the lifecycle works end-to-end

### Claude's Discretion
- Internal arena implementation details (slab crate vs custom)
- Exact ViewContext API surface beyond read/update/notify
- Error handling strategy for wgpu initialization failures
- File/module organization within the ora crate
- Exact Element trait associated types and method signatures beyond what's specified

</decisions>

<specifics>
## Specific Ideas

- GPUI is the primary reference model — follow its patterns for Entity/Model split, context-mediated borrowing, and element lifecycle
- The builder pattern should feel like idiomatic Rust UI frameworks (similar to iced, egui app setup)
- Stub rendering in Phase 1 should be enough to see a colored rectangle in a window, proving the full lifecycle works from App::run() through View::render() through Element::paint()

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 01-foundation-view-system*
*Context gathered: 2026-01-28*
