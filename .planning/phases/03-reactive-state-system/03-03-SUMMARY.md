---
phase: 03-reactive-state-system
plan: 03
subsystem: reactive-state
tags: [events, subscriptions, observers, TypeId, Any, thread-local, cleanup]

# Dependency graph
requires:
  - phase: 03-02
    provides: Observer system with notify effects and ObserverSet
provides:
  - Typed event subscription system with SubscriberSet (per-entity events)
  - Global event bus for app-wide events (theme changes, window resize)
  - Thread-local cleanup queue for safe Subscription Drop
  - cx.subscribe() and cx.subscribe_global() APIs
  - Event dispatch in flush_effects with priority ordering
affects: [03-04, view-integration, event-driven-ui]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Type-erased event dispatch with TypeId and downcast_ref"
    - "Thread-local cleanup queue for Drop safety without AppContext access"
    - "Take/restore pattern for event subscribers during dispatch"
    - "Global event bus separate from per-entity subscribers"

key-files:
  created: []
  modified:
    - ora/src/subscription.rs
    - ora/src/effect.rs
    - ora/src/context.rs

key-decisions:
  - "Thread-local PENDING_CLEANUPS for Subscription Drop without AppContext"
  - "SubscriberSet uses nested HashMap: EntityId -> TypeId -> Vec<Subscriber>"
  - "GlobalEventBus separate from SubscriberSet (no emitter entity)"
  - "global_emit_queue field in EffectQueue for app-wide events"
  - "Cleanup processing at depth==1 in flush_effects loop"
  - "Subscribers receive &dyn Any and downcast to concrete event types"

patterns-established:
  - "Event subscription: cx.subscribe(&model, |event: &E, cx| { ... })"
  - "Global events: cx.subscribe_global::<ThemeChanged>(|event, cx| { ... })"
  - "Event emission: mcx.emit(MyEvent { data: 42 }) inside model.update()"
  - "Global emission: cx.emit_global(WindowResized { width, height })"

# Metrics
duration: 5min
completed: 2026-01-29
---

# Phase 03 Plan 03: Event Subscription System Summary

**Typed event subscriptions with SubscriberSet, GlobalEventBus, and thread-local cleanup queue for safe Drop handling**

## Performance

- **Duration:** 5 min
- **Started:** 2026-01-28T23:51:50Z
- **Completed:** 2026-01-29T02:56:40Z (UTC+3)
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments

- Implemented SubscriberSet with TypeId dispatch for per-entity typed event subscriptions
- Added GlobalEventBus for app-wide events (theme changes, window resize, etc.)
- Implemented thread-local cleanup queue for Subscription Drop safety
- Wired emit event dispatch into flush_effects with take/restore pattern
- Full cleanup: observers, subscribers, and global subscribers removed when Subscription dropped

## Task Commits

Each task was committed atomically:

1. **Tasks 1-2: Add SubscriberSet, GlobalEventBus, and wire subscription APIs** - `e9d1e93` (feat)
   - SubscriberSet with TypeId dispatch
   - GlobalEventBus for app-wide events
   - Thread-local cleanup queue (CleanupAction enum)
   - cx.subscribe() for entity-specific events
   - cx.subscribe_global() for global events
   - cx.emit_global() for app-wide emission
   - flush_effects dispatch for emit queue
   - Cleanup processing in flush cycle

## Files Created/Modified

- `ora/src/subscription.rs` - Added SubscriberSet, GlobalEventBus, thread-local cleanup queue, CleanupAction enum
- `ora/src/effect.rs` - Added global_emit_queue field and push_global_emit/drain_global_emit methods
- `ora/src/context.rs` - Added subscriber_set, global_event_bus fields; subscribe(), subscribe_global(), emit_global() methods; updated flush_effects to process cleanups and dispatch events

## Decisions Made

**Thread-local cleanup queue for Drop safety:**
- Subscription Drop cannot access AppContext (would require global static or complex lifetime management)
- Solution: CleanupAction enum stored in thread-local RefCell<Vec<CleanupAction>>
- flush_effects drains pending cleanups at depth==1 and applies them
- Enables safe Drop from any context (views, callbacks, scope exits)

**SubscriberSet nested HashMap structure:**
- `HashMap<EntityId, HashMap<TypeId, Vec<(SubscriptionId, SubscriberCallback)>>>`
- Enables efficient lookup: get entity's event map, then get event type's subscriber list
- remove_all_for_entity() cleanup when entity is removed

**GlobalEventBus separate from SubscriberSet:**
- Global events have no emitter entity
- Simpler structure: `HashMap<TypeId, Vec<(SubscriptionId, Callback)>>`
- Prevents confusion with sentinel entity IDs

**global_emit_queue in EffectQueue:**
- Separate from entity-specific emit queue
- Avoids need for sentinel GLOBAL_ENTITY_ID constant
- Cleaner separation of concerns

**Cleanup processing at depth==1:**
- Only process cleanups once at start of flush cycle
- Avoids redundant cleanup checks in cascading effect loops

**Type-erased event dispatch:**
- Subscribers receive `&dyn Any` and downcast to concrete types
- Wrapper closure in subscribe() handles downcast and calls typed callback
- Failed downcast is silent (subscriber ignores wrong event type)

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None. Implementation proceeded smoothly:
1. SubscriberSet and GlobalEventBus implemented as specified
2. Thread-local cleanup queue decouples Drop from AppContext
3. flush_effects extended to process cleanups and dispatch events
4. All tests pass (11 existing tests continue passing)

## Next Phase Readiness

**Ready for Plan 03-04 (View Integration):**
- Event subscription system complete
- cx.subscribe() available for views to subscribe to model events
- cx.subscribe_global() available for app-wide events
- flush_effects handles full lifecycle: cleanups → notify → emit → global-emit
- Subscribers receive typed event payloads
- Subscription cleanup prevents memory leaks

**Capabilities unlocked:**
- Views can subscribe to model events: `cx.subscribe(&file_watcher, |event: &FileChanged, cx| { ... })`
- Global event bus for system-wide notifications: `cx.subscribe_global::<ThemeChanged>(|theme, cx| { ... })`
- Models emit events during updates: `mcx.emit(FileChanged { path })`
- App emits global events: `cx.emit_global(WindowResized { width, height })`

**No blockers or concerns.**

---
*Phase: 03-reactive-state-system*
*Completed: 2026-01-29*
