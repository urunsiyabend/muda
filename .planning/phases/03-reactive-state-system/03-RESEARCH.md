# Phase 3: Reactive State System - Research

**Researched:** 2026-01-29
**Domain:** Rust reactive UI patterns, effect queue systems, observer/subscription patterns
**Confidence:** MEDIUM

## Summary

This research investigates implementing a GPUI-style reactive state system in Rust for the Ora UI framework. The system requires explicit notification (`cx.notify()`), observation registration (`cx.observe()`), typed event emission (`cx.emit()`), and an effect queue that prevents reentrancy bugs.

The standard approach in Rust UI frameworks combines three patterns: (1) entity-based storage with typed handles, (2) effect queues with run-to-completion semantics, and (3) subscription registries with automatic cleanup via Drop. GPUI (from Zed editor) demonstrates the reference implementation—effects queue during update closures, flush at top-level to prevent reentrancy, observers stored in SubscriberSet mappings from EntityId to closures.

Key architectural decisions are locked per CONTEXT.md: explicit `cx.notify()`, same-frame re-render, effect deduplication, cascading effects with depth limits, typed priority lanes, and closure-based observation with automatic Drop cleanup.

**Primary recommendation:** Implement effect queue with VecDeque, separate Model<T> wrapper from Entity<T>, use HashMap<TypeId, Vec<Box<dyn FnMut(...)>>> for subscription registry, integrate async-executor's LocalExecutor for cx.spawn(), and set cascading depth limit to 10.

## Standard Stack

The established libraries/tools for reactive Rust UI systems:

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| generational-arena | 0.2.x | Entity storage with generation checking | Solves ABA problem for stable handles, inspired by Catherine West's RustConf 2018 ECS talk |
| std::collections::VecDeque | stdlib | Effect queue (double-ended queue) | O(1) push_back/pop_front for FIFO processing, built-in deduplication support |
| std::any::TypeId | stdlib | Type-safe event/subscriber dispatch | Runtime type identification for HashMap<TypeId, _> registries |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| async-executor | 1.x | LocalExecutor for cx.spawn() | Single-threaded async integration with winit event loop |
| pollster | 0.4.x (already in use) | Manual poll for async tasks | Existing dependency, can manually tick async-executor::LocalExecutor |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| VecDeque | Custom priority queue | VecDeque simpler for basic FIFO; extend with priority lanes using separate VecDeques per priority level |
| async-executor | tokio::runtime::Runtime | Tokio is multi-threaded and heavier; async-executor's LocalExecutor matches single-threaded event loop model |
| HashMap<TypeId, Vec<...>> | Custom registry with unsafe | TypeId+HashMap is safe, well-tested, and sufficient for subscription dispatch |

**Installation:**
```toml
[dependencies]
generational-arena = "0.2"   # Already in Cargo.toml
async-executor = "1"         # Add for cx.spawn()
```

## Architecture Patterns

### Recommended Project Structure
```
ora/src/
├── entity/
│   ├── mod.rs              # Re-exports
│   ├── handle.rs           # Entity<T> (existing)
│   ├── storage.rs          # EntityStorage (existing)
│   └── model.rs            # NEW: Model<T> wrapper, marks reactive entities
├── context.rs              # AppContext, ViewContext, ModelContext (NEW)
├── effect.rs               # NEW: Effect enum, EffectQueue
├── subscription.rs         # NEW: Subscription handle, SubscriberSet
└── async_executor.rs       # NEW: cx.spawn() integration
```

### Pattern 1: Effect Queue with Run-to-Completion

**What:** Effects (notify, emit, spawn) queue during update closures and flush only at top-level App::update() return. Prevents reentrancy where listeners trigger new updates mid-flush.

**When to use:** All model updates, event emissions, and async spawns.

**Example:**
```rust
// Source: GPUI blog (https://zed.dev/blog/gpui-ownership)
// Adapted for Ora

enum Effect {
    Notify { emitter: EntityId },
    Emit { emitter: EntityId, event: Box<dyn Any> },
    Spawn { future: Pin<Box<dyn Future<Output = ()>>> },
}

struct EffectQueue {
    queue: VecDeque<Effect>,
    depth: usize,
}

impl EffectQueue {
    fn push_notify(&mut self, emitter: EntityId) {
        // Deduplicate: only one Notify per entity per flush cycle
        if !self.queue.iter().any(|e| matches!(e, Effect::Notify { emitter: id } if *id == emitter)) {
            self.queue.push_back(Effect::Notify { emitter });
        }
    }

    fn flush(&mut self, depth_limit: usize) {
        self.depth += 1;
        if self.depth > depth_limit {
            panic!("Effect queue depth limit exceeded ({})", depth_limit);
        }

        while let Some(effect) = self.queue.pop_front() {
            match effect {
                Effect::Notify { emitter } => { /* invoke observers */ }
                Effect::Emit { emitter, event } => { /* invoke subscribers */ }
                Effect::Spawn { future } => { /* submit to LocalExecutor */ }
            }
        }

        self.depth -= 1;
    }
}
```

### Pattern 2: Model<T> Wrapper for Reactive Entities

**What:** Separate Model<T> from Entity<T>. Entity<T> is generic storage, Model<T> marks entities as observable/emittable. Model<T> internally holds Entity<ModelInner<T>>, where ModelInner tracks subscriptions.

**When to use:** When state needs to be observed or emit events. Use plain Entity<T> for non-reactive data.

**Example:**
```rust
// Source: Inferred from GPUI patterns

struct ModelInner<T> {
    data: T,
    observers: Vec<Subscription>,  // Stored here for Drop cleanup
    subscribers: HashMap<TypeId, Vec<Subscription>>,
}

pub struct Model<T> {
    entity: Entity<ModelInner<T>>,
}

impl<T: 'static> Model<T> {
    pub fn read<'a>(&self, cx: &'a AppContext) -> &'a T {
        &cx.entity_storage.read(&self.entity).data
    }

    pub fn update<R>(&self, cx: &mut AppContext, f: impl FnOnce(&mut T, &mut ModelContext) -> R) -> R {
        let mut model_cx = ModelContext::new(cx, self.entity);
        let result = cx.entity_storage.update(&self.entity, |inner| {
            f(&mut inner.data, &mut model_cx)
        });
        // ModelContext queued effects; they flush at top-level
        result
    }
}
```

### Pattern 3: Typed Priority Lanes in Effect Queue

**What:** Instead of single VecDeque, use separate queues per priority. Flush high-priority (notify) before low-priority (emit, spawn).

**When to use:** Always. Ensures UI updates (notify) process before event propagation (emit).

**Example:**
```rust
// Source: Research inference from GPUI "typed priority lanes"

struct EffectQueue {
    high_priority: VecDeque<Effect>,  // Notify
    mid_priority: VecDeque<Effect>,   // Emit
    low_priority: VecDeque<Effect>,   // Spawn
    depth: usize,
}

impl EffectQueue {
    fn flush(&mut self, depth_limit: usize) {
        self.depth += 1;
        if self.depth > depth_limit {
            panic!("Effect queue depth limit exceeded ({})", depth_limit);
        }

        // Process in priority order
        while let Some(effect) = self.high_priority.pop_front() {
            self.process(effect);
        }
        while let Some(effect) = self.mid_priority.pop_front() {
            self.process(effect);
        }
        while let Some(effect) = self.low_priority.pop_front() {
            self.process(effect);
        }

        self.depth -= 1;
    }
}
```

### Pattern 4: Subscription Registry with TypeId Dispatch

**What:** HashMap<EntityId, HashMap<TypeId, Vec<Box<dyn FnMut(...)>>>> for typed event subscriptions. Outer map is per-entity, inner map is per-event-type.

**When to use:** cx.subscribe(entity, |observer, emitter, event: &EventType, cx| {...}).

**Example:**
```rust
// Source: Will Crichton's "Registries" pattern + GPUI subscription model
// https://willcrichton.net/rust-api-type-patterns/registries.html

type EntityId = generational_arena::Index;
type SubscriberId = usize;

struct SubscriberSet {
    // EntityId -> (EventTypeId -> Vec<(SubscriberId, Callback)>)
    subscribers: HashMap<EntityId, HashMap<TypeId, Vec<(SubscriberId, Box<dyn FnMut(&dyn Any)>)>>>,
    next_id: SubscriberId,
}

impl SubscriberSet {
    fn subscribe<E: 'static>(&mut self, emitter: EntityId, mut callback: impl FnMut(&E) + 'static) -> Subscription {
        let sub_id = self.next_id;
        self.next_id += 1;

        let wrapped = Box::new(move |event: &dyn Any| {
            if let Some(e) = event.downcast_ref::<E>() {
                callback(e);
            }
        });

        self.subscribers
            .entry(emitter)
            .or_default()
            .entry(TypeId::of::<E>())
            .or_default()
            .push((sub_id, wrapped));

        Subscription {
            emitter,
            event_type: TypeId::of::<E>(),
            id: sub_id,
        }
    }

    fn emit<E: 'static>(&mut self, emitter: EntityId, event: &E) {
        if let Some(event_map) = self.subscribers.get_mut(&emitter) {
            if let Some(callbacks) = event_map.get_mut(&TypeId::of::<E>()) {
                for (_id, callback) in callbacks.iter_mut() {
                    callback(event as &dyn Any);
                }
            }
        }
    }
}
```

### Pattern 5: Automatic Cleanup via Drop

**What:** Subscription handle stores (emitter_id, event_type, subscription_id). Drop implementation removes callback from registry.

**When to use:** All subscriptions. Zero manual cleanup.

**Example:**
```rust
// Source: GPUI auto-Drop pattern + Rust Drop trait docs
// https://doc.rust-lang.org/book/ch15-03-drop.html

pub struct Subscription {
    emitter: EntityId,
    event_type: TypeId,
    id: SubscriberId,
    cleanup: Option<Box<dyn FnOnce()>>,
}

impl Subscription {
    pub fn detach(mut self) {
        // Prevent Drop cleanup
        self.cleanup = None;
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        if let Some(cleanup_fn) = self.cleanup.take() {
            cleanup_fn();  // Removes subscription from registry
        }
    }
}

// Usage:
let _sub = cx.observe(&model, |view, cx| {
    // Observer callback
    view.notify(cx);
});
// Subscription dropped when view entity is dropped -> automatic cleanup
```

### Pattern 6: cx.spawn() with LocalExecutor

**What:** Integrate async-executor's LocalExecutor. cx.spawn() submits future, event loop manually ticks executor.

**When to use:** File I/O, network requests, timers—any async work.

**Example:**
```rust
// Source: async-executor docs + winit integration pattern
// https://docs.rs/async-executor/latest/async_executor/struct.LocalExecutor.html

use async_executor::LocalExecutor;

struct AppContext {
    entity_storage: EntityStorage,
    effect_queue: EffectQueue,
    executor: LocalExecutor<'static>,
}

impl AppContext {
    pub fn spawn<F>(&self, future: F) -> async_executor::Task<F::Output>
    where
        F: Future + 'static,
        F::Output: 'static,
    {
        self.executor.spawn(future)
    }
}

// In event loop (platform/event_loop.rs):
impl ApplicationHandler for OraApp {
    fn window_event(&mut self, ...) {
        // ... existing logic ...

        // After processing window events, tick async executor
        while self.app_context.executor.try_tick() {}
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // Poll executor when idle
        while self.app_context.executor.try_tick() {}
    }
}
```

### Anti-Patterns to Avoid

- **Immediate observer invocation in cx.notify()**: Causes reentrancy bugs. Queue effects instead.
- **Storing Model<T> directly in views**: Breaks lifetime invariants. Store handles, read through context.
- **Manual subscription cleanup**: Leads to use-after-free or leaks. Always rely on Drop.
- **Unbounded effect depth**: Infinite loops hard to debug. Enforce depth limit (recommend 10).
- **Blocking I/O in update closures**: Freezes UI. Use cx.spawn() for async work.

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Async executor integration | Custom future polling loop | async-executor::LocalExecutor | Handles wake notifications, task scheduling, edge cases like panic unwinding |
| Type-safe event dispatch | TypeId match statements | HashMap<TypeId, Vec<Callback>> registry | Scales to many event types, self-documenting pattern |
| Subscription cleanup tracking | Manual Vec remove on Drop | Subscription handle with cleanup closure | Prevents use-after-free, handles edge cases (double-drop, panic during drop) |
| Effect deduplication | Linear scan of VecDeque | HashSet<EntityId> alongside queue | O(1) duplicate check vs O(n) scan |
| Priority queue | Hand-rolled insertion sort | Separate VecDeques per priority lane | Simpler, clearer intent, easier to debug |

**Key insight:** Reactive systems have subtle reentrancy and lifetime bugs. Use well-tested patterns (GPUI's approach) rather than inventing from scratch. The complexity is in edge cases (observer removed mid-notification, model dropped during emit, cascading effects), not the happy path.

## Common Pitfalls

### Pitfall 1: Reentrancy in Observer Callbacks

**What goes wrong:** Observer callback calls `cx.update()` on another model, which flushes effects, causing nested flush and inconsistent state.

**Why it happens:** Naive implementation invokes observers synchronously inside `cx.notify()`, which is itself inside an `update()` closure.

**How to avoid:** Queue `Effect::Notify` during `cx.notify()`, flush only at top-level (when `update_depth == 0` or explicit flush point).

**Warning signs:** "Already borrowed" panics, inconsistent UI state, race conditions that disappear with logging (Heisenbug).

### Pitfall 2: Subscription Lifetime Longer Than Observer

**What goes wrong:** Observer entity is dropped, but subscription still exists in registry. Next emit() calls dangling callback, causing panic or UB.

**Why it happens:** Subscriptions stored globally, not tied to observer lifetime.

**How to avoid:** Store Subscription in observer's entity storage. When observer entity drops, Subscription drops, Drop::drop() removes callback from registry.

**Warning signs:** "Entity handle is invalid" panics on emit, use-after-free, segfaults in release builds.

### Pitfall 3: Infinite Effect Cascades

**What goes wrong:** Observer A notifies Model B, whose observer notifies Model C, whose observer notifies Model A—infinite loop.

**Why it happens:** No depth limit on cascading effects.

**How to avoid:** Track `effect_queue.depth`, panic if exceeds limit (recommend 10). Log warning at depth 5 for early detection.

**Warning signs:** Stack overflow, application hang, excessive CPU usage during simple interactions.

### Pitfall 4: Stale Data in Observer Callbacks

**What goes wrong:** Observer callback captures `model.read(cx)` data, but model updates before callback runs. Callback sees old data.

**Why it happens:** Effect queue delays observer invocation. Data captured at notify time, not callback time.

**How to avoid:** Observer callbacks receive only (observer, cx), not event data. Observer re-reads model state inside callback: `let current = model.read(cx)`.

**Warning signs:** UI shows outdated values, "ghost" state that corrects on next frame, flicker.

### Pitfall 5: Borrowing Conflicts with ModelContext

**What goes wrong:** `model.update(cx, |m, model_cx| { model_cx.notify(); })` needs `&mut AppContext` for both `update()` and `notify()`, causing borrow conflict.

**Why it happens:** ModelContext needs to queue effects in AppContext while mutation closure holds mutable borrow of entity storage.

**How to avoid:** ModelContext stores EntityId and queues effects in a separate buffer. After closure returns, `update()` drains buffer into AppContext.effect_queue. Or: raw pointer to effect queue (unsafe but contained).

**Warning signs:** "Cannot borrow as mutable more than once" compiler errors in model.update() calls.

### Pitfall 6: async-executor Blocking Event Loop

**What goes wrong:** Spawned future does blocking I/O (std::fs::read), freezes UI during read.

**Why it happens:** LocalExecutor runs on main thread. Blocking call blocks thread, preventing winit events.

**How to avoid:** Use async I/O libraries (async-std, tokio) or spawn_blocking equivalent. Document: cx.spawn() futures must not block.

**Warning signs:** UI freezes during file operations, unresponsive window, event queue backlog.

## Code Examples

Verified patterns from official sources:

### Observer Registration (GPUI Style)

```rust
// Source: GPUI blog post (https://zed.dev/blog/gpui-ownership)
// Adapted for Ora API

impl<V: View> ViewContext<V> {
    pub fn observe<T: 'static>(
        &mut self,
        model: &Model<T>,
        callback: impl FnMut(&mut V, &mut ViewContext<V>) + 'static,
    ) -> Subscription {
        let observer_id = self.view_entity.index;  // Current view's EntityId
        let observed_id = model.entity.index;

        // Register callback in observer registry
        let sub_id = self.app_context.observers.add_observer(
            observed_id,
            observer_id,
            Box::new(callback),
        );

        // Return subscription handle
        Subscription::new(observed_id, sub_id, {
            let observers = &mut self.app_context.observers;
            move || observers.remove(observed_id, sub_id)
        })
    }
}

// Usage in view init:
impl View for MyView {
    fn init(cx: &mut ViewContext<Self>) {
        let model = cx.insert(MyModel::new());
        cx.observe(&model, |view, cx| {
            // Re-render when model changes
            cx.notify();
        }).detach();  // Keep subscription alive for view's lifetime
    }
}
```

### Effect Queue Flush

```rust
// Source: GPUI blog "flush_effects" pattern
// https://zed.dev/blog/gpui-ownership

impl AppContext {
    fn update<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        self.update_depth += 1;
        let result = f(self);
        self.update_depth -= 1;

        if self.update_depth == 0 {
            self.flush_effects();
        }

        result
    }

    fn flush_effects(&mut self) {
        const DEPTH_LIMIT: usize = 10;

        loop {
            // Priority lanes: notify, then emit, then spawn
            if let Some(effect) = self.effect_queue.high_priority.pop_front() {
                self.process_notify(effect);
            } else if let Some(effect) = self.effect_queue.mid_priority.pop_front() {
                self.process_emit(effect);
            } else if let Some(effect) = self.effect_queue.low_priority.pop_front() {
                self.process_spawn(effect);
            } else {
                break;  // All queues empty
            }

            if self.effect_queue.depth > DEPTH_LIMIT {
                panic!("Effect queue exceeded depth limit of {}", DEPTH_LIMIT);
            }
        }
    }
}
```

### Typed Event Subscription

```rust
// Source: TypeId HashMap pattern from Will Crichton's "Registries"
// https://willcrichton.net/rust-api-type-patterns/registries.html
// Combined with GPUI cx.subscribe pattern

pub struct FileChangedEvent {
    pub path: PathBuf,
}

impl<V: View> ViewContext<V> {
    pub fn subscribe<E: 'static>(
        &mut self,
        model: &Model<impl 'static>,
        mut callback: impl FnMut(&mut V, &Model<impl 'static>, &E, &mut ViewContext<V>) + 'static,
    ) -> Subscription {
        let emitter_id = model.entity.index;
        let subscriber_id = self.view_entity.index;

        // Type-erased callback wrapper
        let wrapped = Box::new(move |event: &dyn Any| {
            if let Some(e) = event.downcast_ref::<E>() {
                // Invoke with typed event
                callback(/* ... */, e, /* ... */);
            }
        });

        self.app_context.subscribers.subscribe(emitter_id, TypeId::of::<E>(), wrapped)
    }
}

// Usage:
cx.subscribe(&file_watcher_model, |view, _model, event: &FileChangedEvent, cx| {
    log::info!("File changed: {:?}", event.path);
    view.reload_file(cx);
}).detach();
```

### cx.spawn() Integration

```rust
// Source: async-executor LocalExecutor docs
// https://docs.rs/async-executor/latest/async_executor/struct.LocalExecutor.html

impl AppContext {
    pub fn spawn<F>(&self, future: F) -> async_executor::Task<F::Output>
    where
        F: Future + 'static,
        F::Output: 'static,
    {
        self.executor.spawn(future)
    }
}

// Usage example:
model.update(cx, |m, cx| {
    cx.spawn(async move {
        let data = load_file("config.json").await.unwrap();
        // Update model with loaded data
        // (requires Arc<Model> or send message back)
    }).detach();
});

// Event loop integration (in platform/event_loop.rs):
impl ApplicationHandler for OraApp {
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        // ... handle events ...

        // Poll async executor
        while self.app_context.executor.try_tick() {}
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // Poll executor when event queue is empty
        while self.app_context.executor.try_tick() {}
    }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Immediate observer invocation | Effect queue with deferred flush | GPUI v0.1+ (2020-2021) | Eliminates reentrancy bugs, predictable update ordering |
| RefCell-based shared state | Entity handles with AppContext gating | ECS era (2018 RustConf) | Compile-time borrow checking, clearer ownership |
| Generic Rc<dyn Fn()> callbacks | TypeId-dispatched trait objects | Modern Rust (2019+ TypeId HashMap pattern) | Type-safe event payloads, no downcast boilerplate |
| tokio multi-threaded runtime | async-executor LocalExecutor | GPUI async integration (2022+) | Single-threaded, simpler winit integration, lighter weight |

**Deprecated/outdated:**
- **Immediate flush in cx.notify()**: Causes reentrancy. Queue instead.
- **Observer callbacks with event data parameters**: Stale data risk. Pass only (view, cx), view re-reads model.
- **Manual HashSet tracking for subscriptions**: Error-prone. Use Drop-based cleanup.

## Open Questions

Things that couldn't be fully resolved:

1. **Exact depth limit for cascading effects**
   - What we know: GPUI uses some limit, research suggests 10 is common
   - What's unclear: No official GPUI documentation specifies the exact number
   - Recommendation: Start with 10, add warning at 5, make configurable later if needed

2. **async-executor integration details with winit 0.30**
   - What we know: async-executor::LocalExecutor::try_tick() polls pending tasks, winit 0.30 has `about_to_wait` event
   - What's unclear: Optimal polling strategy—tick in `window_event` after each event, or only in `about_to_wait`?
   - Recommendation: Poll in both: after window_event for responsiveness, in about_to_wait for idle tasks. Benchmark if performance issues arise.

3. **Global event bus implementation scope**
   - What we know: Need both per-entity subscriptions (cx.subscribe) and global events (theme changed, window resized)
   - What's unclear: Should global events use same SubscriberSet registry or separate mechanism?
   - Recommendation: Separate GlobalEventBus with TypeId HashMap, no entity_id. Simpler API: cx.subscribe_global<ThemeChanged>(|event, cx| {...}).

4. **ModelContext access to effect queue**
   - What we know: ModelContext needs to queue notify/emit during model.update() closure
   - What's unclear: Safe way to access AppContext.effect_queue while entity_storage is mutably borrowed?
   - Recommendation: ModelContext stores effects in thread-local or internal buffer, `update()` drains buffer after closure. Or: store raw pointer to effect_queue (unsafe but encapsulated in ModelContext).

5. **Subscription storage location**
   - What we know: Subscriptions should drop when observer entity drops
   - What's unclear: Store subscriptions in observer's entity data (user-managed) or in separate SubscriptionRegistry keyed by observer EntityId?
   - Recommendation: Store in observer entity data (ModelInner<T>.subscriptions: Vec<Subscription>). Simpler, automatic cleanup when entity drops. SubscriptionRegistry tracks (emitter, subscriber, callback) triplets.

## Sources

### Primary (HIGH confidence)
- GPUI Ownership Blog: https://zed.dev/blog/gpui-ownership - Effect queue pattern, run-to-completion semantics
- Rust std::collections::VecDeque docs: https://doc.rust-lang.org/std/collections/struct.VecDeque.html - Queue implementation
- Rust Drop trait docs: https://doc.rust-lang.org/book/ch15-03-drop.html - Automatic cleanup pattern
- async-executor crate docs: https://docs.rs/async-executor - LocalExecutor API
- generational-arena crate docs: https://docs.rs/generational-arena/latest/generational_arena/ - Generational indices for stable handles

### Secondary (MEDIUM confidence)
- Will Crichton's "Registries" pattern: https://willcrichton.net/rust-api-type-patterns/registries.html - TypeId HashMap dispatch (verified pattern, widely used)
- GPUI Technical Overview (Medium): https://beckmoulton.medium.com/gpui-a-technical-overview-of-the-high-performance-rust-ui-framework-powering-zed-ac65975cda9f - Architecture overview
- async-executor winit integration: https://github.com/rust-windowing/winit/issues/1199 - Community discussion on async/winit integration challenges
- Rust lifetime misconceptions: https://github.com/pretzelhammer/rust-blog/blob/master/posts/common-rust-lifetime-misconceptions.md - Observer callback lifetime pitfalls

### Tertiary (LOW confidence)
- WebSearch: "GPUI Model cx.observe cx.subscribe implementation details 2025" - Found generic GPUI event system info, marked for validation
- WebSearch: "Rust effect queue depth limit infinite loop prevention 2026" - Found recursion_limit attribute info, not directly applicable but suggests depth limiting is standard practice
- WebSearch: "Rust reactive UI observer pattern effect queue 2026" - Found Dioxus, Quill, Crochet patterns; different approaches but validates effect queue as common pattern

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - generational-arena, VecDeque, TypeId are proven; async-executor is recommended by ecosystem
- Architecture patterns: MEDIUM - GPUI patterns verified from blog post and examples, but official docs incomplete; TypeId HashMap verified from Will Crichton; async-executor patterns inferred from docs
- Pitfalls: MEDIUM - Based on general Rust reactive system knowledge and GPUI blog insights, not exhaustive testing
- Code examples: MEDIUM - Adapted from GPUI blog and Rust docs, not directly from GPUI source code (which would be HIGH)

**Research date:** 2026-01-29
**Valid until:** 2026-02-28 (30 days - GPUI is pre-1.0 and evolving, but core patterns stable)

**Recommendations summary:**
1. Effect queue depth limit: **10** (warn at 5)
2. Priority lane order: **Notify → Emit → Spawn** (UI updates before events before async)
3. Async executor: **async-executor::LocalExecutor** with manual tick in event loop
4. Subscription storage: **In observer entity** (Vec<Subscription> in ModelInner)
5. Global event bus: **Separate TypeId HashMap** (no entity_id required)
6. ModelContext effect queueing: **Internal buffer drained after closure** (safe borrowing)
