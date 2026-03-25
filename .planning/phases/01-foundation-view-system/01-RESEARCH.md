# Phase 1: Foundation & View System - Research

**Researched:** 2026-01-28
**Domain:** Rust UI framework foundations (wgpu, winit, generational arenas, GPUI patterns)
**Confidence:** HIGH

## Summary

Phase 1 establishes the foundational architecture for the ora UI framework: winit event loop ownership, wgpu rendering pipeline, generational arena entity storage, and a GPUI-inspired view/element system. The research confirms that the chosen tech stack (winit 0.30+, wgpu, generational-arena, GPUI architectural patterns) is well-established in the Rust ecosystem with clear implementation patterns.

**Key findings:**
- Winit 0.30+ uses a trait-based `ApplicationHandler` pattern that requires careful integration with async wgpu initialization
- wgpu surface lifecycle requires `Arc<Window>` ownership and `pollster::block_on` for native initialization
- Generational arenas provide zero-unsafe entity storage with ABA problem prevention through generation counters
- GPUI's three-phase rendering (layout → prepaint → paint) separates concerns cleanly and enables efficient GPU-accelerated UI

**Primary recommendation:** Use `generational-arena` for entity storage (simpler, safer than slotmap), wrap wgpu initialization with `pollster::block_on` in winit's `resumed` callback, and follow GPUI's Entity/Model split pattern with context-mediated borrowing.

## Standard Stack

### Core Libraries

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| [winit](https://rust-windowing.github.io/winit/winit/index.html) | 0.30+ | Window creation & event loop | De facto standard for cross-platform windowing in Rust, used by wgpu, bevy, iced |
| [wgpu](https://wgpu.rs/) | Latest (MSRV 1.92) | GPU abstraction layer | Safe, pure-Rust graphics API targeting Vulkan/Metal/DX12/GL, WebGPU-compliant |
| [generational-arena](https://docs.rs/generational-arena/latest/generational_arena/) | Latest | Entity storage | Zero-unsafe arena allocator with generational indices, prevents ABA problem |
| [pollster](https://docs.rs/pollster/) | Latest | Async executor | Minimal async runtime for blocking on futures, recommended by winit maintainers for wgpu init |

### Supporting Libraries

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| typed-generational-arena | Latest | Type-safe generational indices | If compile-time type safety for Entity<T> handles is needed (optional enhancement) |
| thunderdome | Latest | Alternative arena | Performance-critical scenarios (8-byte keys, gladiatorial benchmarks) |
| slotmap | Latest | Secondary maps support | If multiple maps keyed by same entity ID are needed (Phase 3+) |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| generational-arena | slotmap | slotmap has more features (secondary maps) but contains unsafe code; generational-arena is simpler and 100% safe |
| generational-arena | slab | slab is faster but no generation tracking; dangling handles access wrong data (ABA problem) |
| pollster | async-std/tokio | Full async runtimes add significant dependencies; pollster is 10 LOC for blocking on futures |
| winit ApplicationHandler | deprecated poll_events | Old API doesn't work on web/mobile; new trait-based API is required for platform portability |

**Installation:**
```toml
[dependencies]
winit = "0.30"
wgpu = "latest"
generational-arena = "0.2"
pollster = "0.3"
```

## Architecture Patterns

### Recommended Project Structure
```
crates/ora/src/
├── app.rs              # App struct, builder pattern, ora::run() entry point
├── context/
│   ├── mod.rs          # Re-exports all context types
│   ├── app_context.rs  # AppContext (global state, entity storage access)
│   ├── view_context.rs # ViewContext (scoped to view, read/update/notify)
│   └── window_context.rs # WindowContext (window-level operations)
├── entity/
│   ├── mod.rs
│   ├── storage.rs      # Generational arena wrapper, typed arenas
│   └── handle.rs       # Entity<T>, Model<T> smart pointers
├── view/
│   ├── mod.rs
│   └── trait.rs        # View trait definition
├── element/
│   ├── mod.rs
│   ├── trait.rs        # Element trait (request_layout, prepaint, paint)
│   └── any_element.rs  # AnyElement type-erasure wrapper
├── window.rs           # Window struct, root view registration
└── platform/           # wgpu/winit integration
    ├── mod.rs
    ├── gpu.rs          # Instance, Device, Queue, Surface
    └── event_loop.rs   # ApplicationHandler implementation
```

### Pattern 1: Winit 0.30 + wgpu Integration

**What:** Integrate wgpu's async initialization with winit's synchronous `ApplicationHandler` trait using `pollster::block_on`.

**When to use:** Required for all native desktop apps using winit 0.30+ and wgpu.

**Example:**
```rust
// Source: https://github.com/rust-windowing/winit/discussions/3667
use std::sync::Arc;
use winit::application::ApplicationHandler;

struct App {
    window: Option<Arc<Window>>,
    gpu_state: Option<GpuState>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(event_loop.create_window(Window::default_attributes()).unwrap());

        // Use pollster to block on async wgpu init
        let gpu_state = pollster::block_on(GpuState::new(window.clone()));

        self.window = Some(window);
        self.gpu_state = Some(gpu_state);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        // Handle events
    }
}
```

**Key insight:** Store `Arc<Window>` to share lifetime with `wgpu::Surface<'window>`. The surface needs a reference to the window; using Arc allows cloning without lifetime annotations.

### Pattern 2: Generational Arena Entity Storage

**What:** Wrap generational-arena in typed storage with Entity<T> handles providing generation-checked access.

**When to use:** All state storage in the framework — views, models, any user data.

**Example:**
```rust
// Source: https://docs.rs/generational-arena/latest/generational_arena/
use generational_arena::{Arena, Index};

pub struct Entity<T> {
    index: Index,
    generation: u64,
    _phantom: PhantomData<T>,
}

pub struct EntityStorage {
    arenas: HashMap<TypeId, Box<dyn Any>>, // TypeId -> Arena<T>
}

impl EntityStorage {
    pub fn insert<T: 'static>(&mut self, value: T) -> Entity<T> {
        let arena = self.get_arena_mut::<T>();
        let index = arena.insert(value);
        Entity { index, generation: index.into_raw_parts().1, _phantom: PhantomData }
    }

    pub fn get<T: 'static>(&self, entity: Entity<T>) -> Option<&T> {
        let arena = self.get_arena::<T>()?;
        arena.get(entity.index) // Generation checked by arena
    }
}
```

**Key insight:** Type erasure via `Box<dyn Any>` + `TypeId` allows heterogeneous storage while maintaining type safety at the Entity<T> handle level.

### Pattern 3: GPUI-Style View/Element Split

**What:** Views are stateful entities implementing `render(&self, cx) -> AnyElement`. Elements are the rendering primitives with three-phase lifecycle.

**When to use:** All UI components in ora — views own state, elements own rendering.

**Example:**
```rust
// Source: GPUI patterns from https://github.com/zed-industries/zed/blob/main/crates/gpui/README.md
pub trait View: 'static {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl Element;
}

pub trait Element {
    type State; // Persistent state cached between frames

    fn request_layout(&mut self, cx: &mut LayoutContext) -> Self::State;
    fn prepaint(&mut self, state: &mut Self::State, cx: &mut PrepaintContext);
    fn paint(&mut self, state: &Self::State, cx: &mut PaintContext);
}

// Type-erased element for heterogeneous collections
pub struct AnyElement {
    element: Box<dyn ElementObject>,
}

trait ElementObject {
    fn request_layout(&mut self, cx: &mut LayoutContext) -> Box<dyn Any>;
    fn prepaint(&mut self, state: &mut dyn Any, cx: &mut PrepaintContext);
    fn paint(&mut self, state: &dyn Any, cx: &mut PaintContext);
}
```

**Key insight:** `AnyElement` uses trait objects for type erasure. The `State` associated type is erased to `Box<dyn Any>` at the boundary, allowing framework-cached state without generic parameters.

### Pattern 4: Builder Pattern for App Configuration

**What:** Fluent API for configuring the App before calling `run()` which takes ownership of the event loop.

**When to use:** API surface for ora::App — matches idiomatic Rust UI frameworks (iced, egui).

**Example:**
```rust
// Source: https://rust-unofficial.github.io/patterns/patterns/creational/builder.html
pub struct App {
    title: String,
    size: (u32, u32),
    on_open: Option<Box<dyn FnOnce(&mut WindowContext)>>,
}

impl App {
    pub fn new() -> Self {
        Self {
            title: "ora".into(),
            size: (800, 600),
            on_open: None,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.size = (width, height);
        self
    }

    pub fn on_open<F>(mut self, f: F) -> Self
    where F: FnOnce(&mut WindowContext) + 'static
    {
        self.on_open = Some(Box::new(f));
        self
    }

    pub fn run(self) -> ! {
        // Take ownership and run event loop
        ora::platform::run_event_loop(self)
    }
}
```

**Key insight:** Builder takes `self` by value and returns `Self`, enabling method chaining. The final `run()` consumes the builder and never returns (winit event loop).

### Pattern 5: Context-Mediated Borrowing

**What:** All entity access goes through context types (`cx.read()`, `cx.update()`), not direct mutable access. Prevents borrow checker conflicts.

**When to use:** Any time views need to read/update entity state or render child views.

**Example:**
```rust
// Source: GPUI patterns
impl ViewContext<'_, MyView> {
    pub fn read<T>(&self, entity: &Entity<T>) -> &T {
        self.app_context.entity_storage.get(entity).expect("dangling entity")
    }

    pub fn update<T, R>(&mut self, entity: &Entity<T>, f: impl FnOnce(&mut T, &mut ViewContext<Self>) -> R) -> R {
        // Temporarily remove from storage to provide &mut T
        let mut value = self.app_context.entity_storage.remove(entity);
        let result = f(&mut value, self);
        self.app_context.entity_storage.insert_at(entity, value);
        result
    }
}

// Usage in View::render
fn render(&mut self, cx: &mut ViewContext<Self>) -> impl Element {
    let child_state = cx.read(&self.child_entity);

    cx.update(&self.other_entity, |entity, cx| {
        entity.some_method();
    });

    // Render element tree
    div().child(text(child_state.text.clone()))
}
```

**Key insight:** The `update` closure gets both `&mut T` and `&mut ViewContext`, avoiding simultaneous borrows of AppContext. The arena pattern (remove → mutate → re-insert) is hidden behind the API.

### Anti-Patterns to Avoid

- **Direct Arena Indexing:** Never use `arena[entity]` which panics on dangling handles. Always use `arena.get(entity)` and handle `None`.
- **Storing &T References:** Don't store references to arena contents; they're invalidated on any mutation. Store `Entity<T>` handles instead.
- **Zero-Size Surfaces:** Never configure wgpu surface with width=0 or height=0; cache window size and skip rendering if zero.
- **Blocking the Event Loop:** Don't perform heavy computation in event handlers; winit expects quick return for smooth UI. Defer work to background tasks.
- **Manual RefCell Management:** Don't use `Rc<RefCell<T>>` for entity state. Use `Entity<T>` handles + context-mediated borrowing instead.

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Entity storage with handles | Custom arena with unsafe pointers | `generational-arena` crate | ABA problem, generation tracking, memory safety — already solved with zero unsafe |
| Async wgpu init with winit | Complex future executor | `pollster::block_on()` | Winit maintainers recommend it; 10 LOC, zero dependencies beyond futures |
| Window lifetime for Surface | Manual lifetime annotations | `Arc<Window>` | Surface needs 'window lifetime; Arc gives 'static with refcounting |
| Type-erased element tree | Custom vtable/Any casts | `Box<dyn Trait>` with associated types | Compiler generates vtables; performance acceptable for UI (not hot loop) |
| Event loop ownership | Custom event pump | winit `ApplicationHandler` trait | Cross-platform (web/mobile) requires this API; old `poll_events` deprecated |
| Surface reconfiguration | Manual validation | Check `SurfaceStatus::Suboptimal` | wgpu provides optimal reconfiguration timing; manual logic misses edge cases |

**Key insight:** Rust's ecosystem has mature solutions for GPU rendering and windowing. The main challenge is integrating APIs with mismatched async/sync boundaries (winit synchronous, wgpu async) — solved by `pollster`.

## Common Pitfalls

### Pitfall 1: Surface Lifetime Violations

**What goes wrong:** Creating `wgpu::Surface` with temporary `&Window` reference causes lifetime errors. The surface outlives the window borrow.

**Why it happens:** wgpu's `Surface<'window>` lifetime is tied to the window. If window is stored as `Option<Window>` and borrowed with `window.as_ref()`, the borrow ends when the expression ends, but surface needs the window alive.

**How to avoid:**
- Store window as `Arc<Window>`, pass `window.clone()` to surface creation
- Surface gets 'static lifetime via Arc, no lifetime annotations needed
- Example: `let surface = instance.create_surface(window.clone())?;`

**Warning signs:** Compile errors mentioning "lifetime may not live long enough" or "borrowed value does not live long enough" near surface creation.

**Sources:**
- https://github.com/rust-windowing/winit/discussions/3667
- https://users.rust-lang.org/t/lifetime-troubles-with-windows-and-surfaces/111220

### Pitfall 2: Zero-Area Surface Configuration

**What goes wrong:** Configuring wgpu surface with width=0 or height=0 causes panic or validation error. App crashes on window minimize or during startup.

**Why it happens:** Some platforms report zero size briefly during window creation or when minimized. wgpu cannot create a zero-sized swapchain.

**How to avoid:**
```rust
pub fn resize(&mut self, width: u32, height: u32) {
    if width > 0 && height > 0 {
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }
    // If zero, skip reconfigure; render nothing this frame
}
```

**Warning signs:** Panic with message "surface configuration failed" or "width/height must be non-zero" on resize events.

**Sources:**
- https://sotrh.github.io/learn-wgpu/beginner/tutorial2-surface/

### Pitfall 3: Dangling Entity Handles After Removal

**What goes wrong:** Storing `Entity<T>` handles, removing the entity from storage, then accessing the stale handle. Panics with generation mismatch or accesses wrong data.

**Why it happens:** Generational arenas reuse slots. If entity A is removed and entity B inserted in the same slot, old handles to A now point to B's slot but with wrong generation.

**How to avoid:**
- Always use fallible `get()` / `get_mut()` methods, not direct indexing
- Check `Option<&T>` result; `None` means dangling handle
- Design API to panic explicitly: `entity.read(cx)` panics with clear message "Entity<T> handle is invalid"
- For optional relationships, use `Option<Entity<T>>` and handle missing children gracefully

**Warning signs:** Panics mentioning "generation mismatch" or "index out of bounds" when accessing entities that were previously removed.

**Sources:**
- https://docs.rs/generational-arena/latest/generational_arena/

### Pitfall 4: Async wgpu Init in Synchronous resumed()

**What goes wrong:** `ApplicationHandler::resumed()` is not async, but `wgpu::Adapter::request_device()` is async. Compilation fails or forces unsafe workarounds.

**Why it happens:** Winit's event loop is fundamentally synchronous (must return quickly), but wgpu initialization queries GPU capabilities asynchronously.

**How to avoid:**
- Use `pollster::block_on()` for native targets: `pollster::block_on(async { ... })`
- For web, this doesn't work; need to split initialization into separate async path (deferred to Phase 2+)
- Winit maintainers explicitly recommend pollster for native wgpu integration

**Warning signs:** Compile error "async block cannot be awaited in synchronous function" in `resumed()` callback.

**Sources:**
- https://github.com/rust-windowing/winit/discussions/3667

### Pitfall 5: Forgetting Surface Reconfiguration on Resize

**What goes wrong:** Window resizes, but surface swapchain still has old dimensions. Rendering is stretched/cropped, or validation errors occur.

**Why it happens:** wgpu doesn't automatically reconfigure the surface on window resize; application must handle `WindowEvent::Resized` and call `surface.configure()`.

**How to avoid:**
```rust
match event {
    WindowEvent::Resized(new_size) => {
        self.resize(new_size.width, new_size.height);
    }
    // ...
}
```

**Warning signs:** Window resizes but content doesn't scale, or wgpu validation layers warn "surface size mismatch".

**Sources:**
- https://sotrh.github.io/learn-wgpu/beginner/tutorial2-surface/

### Pitfall 6: Box<dyn Element> Performance Paranoia

**What goes wrong:** Over-optimizing type erasure overhead prematurely, adding complex static dispatch mechanisms when Box<dyn Trait> is sufficient.

**Why it happens:** Rust developers know vtable dispatch is slower than monomorphization, assume UI rendering is too slow with dynamic dispatch.

**How to avoid:**
- **Accept Box<dyn Element>** for element trees — UI rendering is not a hot loop (60 FPS = 16ms budget, plenty of time)
- Vtable overhead is ~1-5ns per call; even 1000 elements = 5μs, negligible in 16ms frame
- GPU commands and rasterization dominate (milliseconds), not vtable lookups (nanoseconds)
- GPUI and other production frameworks use trait objects for elements; proven at scale in Zed

**Warning signs:** Attempting to make `Element` generic (`Element<T>`) or using proc macros to generate static dispatch wrappers before profiling shows a problem.

**Sources:**
- https://quinedot.github.io/rust-learning/dyn-trait-vs.html

## Code Examples

Verified patterns from official sources:

### Creating Window and wgpu Surface

```rust
// Source: https://sotrh.github.io/learn-wgpu/beginner/tutorial2-surface/
use std::sync::Arc;
use wgpu::*;
use winit::window::Window;

pub async fn new(window: Arc<Window>) -> Self {
    let size = window.inner_size();

    // Instance is the handle to our GPU
    let instance = wgpu::Instance::new(InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..Default::default()
    });

    // Surface is the part we draw to
    let surface = instance.create_surface(window.clone()).unwrap();

    // Adapter is the actual graphics card
    let adapter = instance.request_adapter(&RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        compatible_surface: Some(&surface),
        force_fallback_adapter: false,
    }).await.unwrap();

    // Device and Queue
    let (device, queue) = adapter.request_device(
        &wgpu::DeviceDescriptor {
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            label: None,
            memory_hints: Default::default(),
        },
        None,
    ).await.unwrap();

    // Surface configuration
    let surface_caps = surface.get_capabilities(&adapter);
    let surface_format = surface_caps
        .formats
        .iter()
        .find(|f| f.is_srgb())
        .copied()
        .unwrap_or(surface_caps.formats[0]);

    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Fifo, // VSync enabled
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };

    surface.configure(&device, &config);

    Self { window, surface, device, queue, config }
}
```

### Generational Arena Usage

```rust
// Source: https://docs.rs/generational-arena/latest/generational_arena/
use generational_arena::{Arena, Index};

let mut arena = Arena::new();

// Insert returns an Index (generation-tracked)
let rza = arena.insert("RZA");
let gza = arena.insert("GZA");

// Access with get() (returns Option)
assert_eq!(arena.get(rza), Some(&"RZA"));

// Remove and access fails gracefully
arena.remove(gza);
assert_eq!(arena.get(gza), None);

// Iteration over live entries
for (index, value) in &arena {
    println!("{:?}: {}", index, value);
}
```

### Basic Render Pass

```rust
// Source: https://docs.rs/wgpu/latest/wgpu/struct.RenderPass.html
let frame = surface.get_current_texture().unwrap();
let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
    label: Some("Render Encoder"),
});

{
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Render Pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: 0.1,
                    g: 0.2,
                    b: 0.3,
                    a: 1.0,
                }),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        ..Default::default()
    });

    // Set pipeline, bind groups, draw calls here
    render_pass.set_pipeline(&render_pipeline);
    render_pass.draw(0..3, 0..1); // Draw 3 vertices, 1 instance
}

queue.submit(std::iter::once(encoder.finish()));
frame.present();
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| EventLoop::poll_events() | ApplicationHandler trait | winit 0.28 → 0.30 | Web/mobile compatibility; all apps must migrate to trait-based API |
| wgpu 0.19 | wgpu (WebGPU-compliant) | Ongoing | API stabilizing; Surface/Device creation follows WebGPU spec |
| Manual async executors | pollster::block_on() | 2024 | Simpler wgpu initialization in winit; recommended pattern by maintainers |
| AppContext/WindowContext (old GPUI) | App/Window/Context<T> (new GPUI) | GPUI 0.2+ (2024) | Simpler context hierarchy; Entity<T> replaces Model<T>/View<T> |
| PresentMode::Immediate default | PresentMode::Fifo default | wgpu policy | VSync enabled by default for stutter-free rendering |

**Deprecated/outdated:**
- `EventLoop::poll_events()` — removed in winit 0.30, use `ApplicationHandler`
- Old GPUI `AppContext`/`ModelContext`/`ViewContext<T>` — replaced with `App`/`Context<T>`/`Window` in GPUI 0.2+
- `WindowBuilder` — deprecated in winit 0.30, use `Window::default_attributes()`

## Open Questions

Things that couldn't be fully resolved:

1. **GPUI Element State Caching**
   - What we know: GPUI elements have `BeforeLayout -> State` associated type cached between frames
   - What's unclear: Exact mechanism for state invalidation (when does framework drop cached state?)
   - Recommendation: Start with simple `type State = ()` (no caching) in Phase 1; add state caching in Phase 2 after basic rendering works

2. **Context API Surface Beyond read/update**
   - What we know: ViewContext provides `read<T>`, `update<T>`, `notify()` for entity access
   - What's unclear: Full API for registering root view, spawning child windows, focus management
   - Recommendation: Implement minimal API (`set_root_view`, `read`, `update`) in Phase 1; expand in later phases

3. **Stub Rendering Approach**
   - What we know: Phase 1 paint should produce colored rectangles to verify lifecycle
   - What's unclear: Whether to use wgpu pipeline with vertex buffers or simpler clear-color-only approach
   - Recommendation: Use colored rectangles via basic vertex buffer + pipeline (closer to real rendering in Phase 2)

4. **Error Handling Strategy**
   - What we know: Rust conventions prefer `Result` for recoverable errors, panic for bugs
   - What's unclear: Whether ora APIs should return `Result` (user-facing errors like "failed to init GPU") or panic (framework bugs)
   - Recommendation: Panic for dangling entity access (programmer error), return `Result` for GPU initialization failures (hardware issues)

## Sources

### Primary (HIGH confidence)

**Official Documentation:**
- [winit Documentation](https://rust-windowing.github.io/winit/winit/index.html) - Current API reference
- [wgpu Learn Tutorial](https://sotrh.github.io/learn-wgpu/) - Surface setup, rendering pipeline
- [generational-arena crate](https://docs.rs/generational-arena/latest/generational_arena/) - API reference
- [wgpu PresentMode](https://docs.rs/wgpu/latest/wgpu/enum.PresentMode.html) - VSync configuration
- [wgpu RenderPass](https://docs.rs/wgpu/latest/wgpu/struct.RenderPass.html) - Drawing API

**GitHub Discussions:**
- [winit 0.30.0 with wgpu · Discussion #3667](https://github.com/rust-windowing/winit/discussions/3667) - Integration patterns
- [Zed GPUI README](https://github.com/zed-industries/zed/blob/main/crates/gpui/README.md) - Architecture overview
- [Zed GPUI Contexts Docs](https://github.com/zed-industries/zed/blob/main/crates/gpui/docs/contexts.md) - Context API

### Secondary (MEDIUM confidence)

**Technical Articles:**
- [GPUI Technical Overview](https://beckmoulton.medium.com/gpui-a-technical-overview-of-the-high-performance-rust-ui-framework-powering-zed-ac65975cda9f) - Three-phase rendering
- [Rust Builder Pattern](https://rust-unofficial.github.io/patterns/patterns/creational/builder.html) - Design patterns
- [RefCell and Interior Mutability](https://doc.rust-lang.org/book/ch15-05-interior-mutability.html) - Borrowing patterns
- [To panic! or Not to panic!](https://doc.rust-lang.org/book/ch09-03-to-panic-or-not-to-panic.html) - Error handling

**Community Resources:**
- [Arenas in Rust](https://manishearth.github.io/blog/2021/03/15/arenas-in-rust/) - Arena patterns
- [dyn Trait Performance](https://quinedot.github.io/rust-learning/dyn-trait-vs.html) - Type erasure tradeoffs
- [LogRocket: Create and Manage Windows with Winit](https://blog.logrocket.com/create-manage-windows-rust-app-with-winit/) - Winit patterns

### Tertiary (LOW confidence)

- [DeepWiki GPUI Framework](https://deepwiki.com/zed-industries/zed/2.2-ui-framework-(gpui)) - Community documentation (may be outdated)
- General web search results about Rust UI patterns (marked for validation in implementation phase)

## Metadata

**Confidence breakdown:**
- Standard stack: **HIGH** - Official docs confirm winit/wgpu/generational-arena are established, maintained, widely used
- Architecture: **HIGH** - Patterns verified from GPUI source, winit/wgpu official examples
- Pitfalls: **HIGH** - Sourced from GitHub issues, official tutorial warnings, maintainer recommendations
- GPUI specifics: **MEDIUM** - GPUI is evolving (0.2+ API changes); patterns from README may not match latest internal APIs

**Research date:** 2026-01-28
**Valid until:** ~2026-02-28 (30 days for stable stack; wgpu/winit stable, generational-arena unchanged for years)
