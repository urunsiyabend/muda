# Phase 3: Reactive State System - Context

**Gathered:** 2026-01-29
**Status:** Ready for planning

<domain>
## Phase Boundary

Add observation mechanism, effect queue, and Model change tracking for reactive UI updates. Views observe models and automatically re-render when state changes. Entities can emit typed events to communicate with subscribers. Event routing (mouse/keyboard) and focus management are Phase 4.

</domain>

<decisions>
## Implementation Decisions

### Change granularity
- Explicit `cx.notify()` — developer calls notify inside `model.update(cx, |m, cx| { ... cx.notify() })` after mutations. No implicit dirty-tracking.
- `cx.notify()` only available through ModelContext (inside update closures). Mutations and notification are co-located.
- All observers re-render in the same frame — marked dirty and re-rendered in the current frame's flush cycle after the event handler completes. No 1-frame desync.
- Multiple `notify()` calls on the same model in one update cycle are deduplicated — results in a single re-render pass per model per flush.

### Effect ordering & batching
- Effects (notify, emit) queue up during update closures and flush after the closure returns. Prevents reentrancy.
- Cascading effects allowed with a depth limit — flushed effects can produce new effects, but stop after N levels (e.g., 10) with panic/warning to prevent infinite loops.
- Typed priority lanes in the effect queue — notify effects flush before emit effects (or similar priority ordering). Not a single FIFO.
- Include `cx.spawn()` for async effects — runs a future that can update models when it completes. Needed for file I/O, network, etc.

### Subscription API
- `cx.observe(handle, |view, cx| { ... })` — explicit closure-based observation. GPUI style.
- Subscriptions set up once at view creation (init/constructor), not re-established every render.
- Automatic cleanup via Drop — subscriptions stored in the view's entity, removed when the entity is dropped. Zero manual cleanup.
- Observe callback receives notification only (view, cx) — just a signal to re-render. View reads current model state during its next render(). No stale data risk.

### Event emission
- `cx.emit(MyEvent { ... })` — entity calls emit inside an update closure. Subscribers receive typed events. GPUI pattern.
- Both per-entity subscriptions AND a global event bus for app-wide events (theme changed, window resized, etc.).
- Events carry typed payloads — events are structs with data: `cx.emit(FileChanged { path })`. Rich communication between components.
- Event subscriptions use the same auto-Drop cleanup as observe — tied to subscribing entity's lifetime. Consistent API across observe and subscribe.

### Claude's Discretion
- Exact depth limit number for cascading effects
- Priority lane ordering specifics (which effect types before others)
- Internal data structures for the effect queue and subscription registry
- How `cx.spawn()` integrates with the winit event loop (tokio, async-executor, or manual polling)
- Global event bus implementation details (trait object registry, TypeId dispatch, etc.)

</decisions>

<specifics>
## Specific Ideas

- Follow GPUI patterns: explicit `cx.notify()`, `cx.observe()`, `cx.emit()`, `cx.subscribe()` — the developer model should feel familiar to someone who's read Zed's source
- ModelContext constrains where notify() can be called — co-location of mutation and notification is a design principle, not just convenience
- Same-frame re-render after notify ensures UI consistency — no frame where one view is updated but another observing the same model is stale

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 03-reactive-state-system*
*Context gathered: 2026-01-29*
