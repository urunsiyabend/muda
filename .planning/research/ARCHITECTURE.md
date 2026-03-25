# Architecture Patterns: GPUI-like UI Framework

**Domain:** UI framework architecture (GPUI-inspired)
**Researched:** 2026-01-28

## Recommended Architecture

GPUI employs a layered architecture separating application state ownership, window management, element tree construction, layout computation, and GPU rendering. The ora framework should follow this proven pattern while adapting to the existing muda codebase structure.

```
┌─────────────────────────────────────────────────────────────┐
│                        Application                          │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   App       │  │   Entity<T>  │  │  Context<T>  │      │
│  │  (owns all  │─>│   (handle)   │  │  (scoped     │      │
│  │  entities)  │  │              │  │   access)    │      │
│  └─────────────┘  └──────────────┘  └──────────────┘      │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                      Window Layer                           │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   Window    │  │ DispatchTree │  │    Frame     │      │
│  │  (manages   │─>│ (event       │  │  (hit test,  │      │
│  │   root)     │  │  routing)    │  │   hitboxes)  │      │
│  └─────────────┘  └──────────────┘  └──────────────┘      │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                    Element Tree Layer                       │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   View      │  │   Element    │  │   Styled     │      │
│  │  (Render    │─>│   (paint,    │  │   (builder   │      │
│  │   trait)    │  │   layout)    │  │    API)      │      │
│  └─────────────┘  └──────────────┘  └──────────────┘      │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                      Layout Layer                           │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   Taffy     │  │  LayoutId    │  │   Bounds     │      │
│  │  (flexbox   │─>│  (computed   │  │  (final x,y, │      │
│  │   engine)   │  │   layout)    │  │    w,h)      │      │
│  └─────────────┘  └──────────────┘  └──────────────┘      │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                    Rendering Layer                          │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   Scene     │  │    Blade     │  │   Platform   │      │
│  │  (draw      │─>│  (graphics   │─>│  (Metal/     │      │
│  │   commands) │  │   abstraction)│  │   Vulkan/DX) │      │
│  └─────────────┘  └──────────────┘  └──────────────┘      │
└─────────────────────────────────────────────────────────────┘
```

### Component Boundaries

| Component | Responsibility | Communicates With | Owns |
|-----------|---------------|-------------------|------|
| **App** | Root context owning all entity data | Entity handles, Window | All Entity<T> data |
| **Context<T>** | Scoped entity access with notify/emit | App (deref), Entity<T> | Nothing (borrows from App) |
| **Window** | Window state, root view, event dispatch | App (via &mut), DispatchTree, Platform | Root Entity<View>, DispatchTree |
| **Entity<T>** | Handle to App-owned data | App (via update/read) | Reference count |
| **View (trait)** | Render method returning element tree | Context (during render) | View-specific state (T) |
| **Element (trait)** | Layout and paint implementation | Taffy (layout), Scene (paint) | Element-specific state |
| **DispatchTree** | Event routing from platform to elements | Window, Frame | Focus paths, event handlers |
| **Frame** | Hit testing and hitbox tracking | DispatchTree, Elements | Hitbox registry |
| **Taffy** | Flexbox layout computation | Elements (request_layout) | Layout tree, computed bounds |
| **Scene** | Retained draw command buffer | Elements (paint phase), Blade | Draw commands (quads, text, etc.) |
| **Blade** | Cross-platform graphics abstraction | Scene, Platform | GPU resources (buffers, textures) |

### Data Flow

**Frame Rendering Pipeline:**

```
1. EVENT ARRIVAL
   Platform event (mouse, keyboard)
   → Window receives PlatformInput
   → Normalized to GPUI event model

2. EVENT DISPATCH (Two-Phase)
   Window::dispatch_event()
   → Hit test via Frame (mouse) or focus path (keyboard)
   → CAPTURE phase: root → target
   → BUBBLE phase: target → root
   → Handler can stop_propagation()

3. STATE UPDATE
   Event handler calls cx.notify() or cx.emit()
   → Effect pushed to queue (not immediate)
   → Update completes
   → Effects flushed (observers notified)
   → Window::request_redraw() called

4. RENDER PHASE (Three-Phase)

   Phase 1: PREPAINT (Layout)
   → Window calls root_view.render(cx)
   → View::render() returns element tree
   → For each element:
      - Element::request_layout(cx) → Taffy
      - Returns LayoutId
      - Stores layout state (no Scene mutation)
   → Taffy computes all bounds
   → Frame updated with hitboxes

   Phase 2: PAINT (Scene Construction)
   → For each element in tree order:
      - Element::prepaint(cx, layout_id)
        - Commits bounds to Frame for hit testing
        - Returns paint state
      - Element::paint(cx, paint_state)
        - Writes to Scene (quads, text, images)
        - NO layout queries allowed
   → Scene contains all draw commands

   Phase 3: PRESENT (GPU Rendering)
   → Scene submitted to Blade
   → Blade generates GPU commands (Metal/Vulkan/DX)
   → Platform window presents frame (vsync)
```

**Reactivity Flow (Observation):**

```
1. OBSERVER REGISTRATION
   entity_handle.observe(cx, |entity, cx| {
       // Callback when entity calls cx.notify()
   })
   → Observer stored in App
   → Tied to current entity or window lifetime

2. STATE MUTATION
   entity.update(cx, |entity_data, cx| {
       entity_data.field = new_value;
       cx.notify(); // Signal change
   })
   → Mutation happens
   → Notification effect queued

3. EFFECT FLUSH
   → After update completes
   → App flushes effect queue
   → All observers invoked with handles
   → Observers can trigger redraws

4. EVENT-BASED (Alternative)
   entity.subscribe(cx, |entity, event, cx| {
       // Handle specific event type
   })
   → entity.update(cx, |data, cx| {
       cx.emit(MyEvent { ... });
   })
   → Event routed to subscribers
```

## Patterns to Follow

### Pattern 1: Centralized Entity Ownership

**What:** App owns all entity data; handles provide access only with App context present.

**When:** Always for application state (models, views, services).

**Why:** Prevents distributed ownership issues in Rust. Enables centralized cleanup, observation, and lifetime management.

**Example:**
```rust
// In ora
pub struct App {
    entities: SlotMap<EntityId, Box<dyn Any>>,
    observers: HashMap<EntityId, Vec<Observer>>,
    // ... other state
}

pub struct Entity<T> {
    id: EntityId,
    _phantom: PhantomData<T>,
}

impl App {
    pub fn new_entity<T: 'static>(&mut self, data: T) -> Entity<T> {
        let id = self.entities.insert(Box::new(data));
        Entity { id, _phantom: PhantomData }
    }

    pub fn update<T: 'static, R>(
        &mut self,
        entity: &Entity<T>,
        f: impl FnOnce(&mut T, &mut Context<T>) -> R,
    ) -> R {
        let data = self.entities.get_mut(entity.id)
            .and_then(|any| any.downcast_mut::<T>())
            .expect("entity not found");
        let mut cx = Context::new(self, entity.id);
        f(data, &mut cx)
    }
}
```

### Pattern 2: Three-Phase Rendering

**What:** Separate layout computation (prepaint), scene construction (paint), and GPU submission (present).

**When:** Every frame for all elements.

**Why:** Allows layout to complete before paint begins. Enables hit testing against previous frame during current frame construction. Prevents read-after-write hazards.

**Example:**
```rust
// In ora
pub trait Element {
    type LayoutState;
    type PaintState;

    fn request_layout(
        &mut self,
        cx: &mut LayoutContext,
    ) -> (LayoutId, Self::LayoutState);

    fn prepaint(
        &mut self,
        cx: &mut PrepaintContext,
        layout_id: LayoutId,
        layout_state: &Self::LayoutState,
    ) -> Self::PaintState;

    fn paint(
        &mut self,
        cx: &mut PaintContext,
        paint_state: &Self::PaintState,
    );
}

// Usage in element tree traversal
fn render_tree(root: &mut dyn Element) {
    // Phase 1: Layout
    let (layout_id, layout_state) = root.request_layout(&mut layout_cx);
    taffy.compute_layout(layout_id);

    // Phase 2: Prepaint + Paint
    let paint_state = root.prepaint(&mut prepaint_cx, layout_id, &layout_state);
    root.paint(&mut paint_cx, &paint_state);

    // Phase 3: Present
    scene.submit_to_gpu();
}
```

### Pattern 3: Deferred Effect Execution

**What:** Notifications and events push to effect queue; flushed after update completes.

**When:** All observer notifications and event emissions.

**Why:** Prevents reentrancy issues where observer triggers mutation during notification loop. Ensures consistent state during update.

**Example:**
```rust
// In ora
pub struct Context<'a, T> {
    app: &'a mut App,
    entity_id: EntityId,
    _phantom: PhantomData<T>,
}

impl<T> Context<'_, T> {
    pub fn notify(&mut self) {
        // Queue effect, don't execute immediately
        self.app.effects.push(Effect::Notify(self.entity_id));
    }

    pub fn emit<E: 'static>(&mut self, event: E) {
        self.app.effects.push(Effect::Emit {
            source: self.entity_id,
            event: Box::new(event),
        });
    }
}

impl App {
    pub fn flush_effects(&mut self) {
        while let Some(effect) = self.effects.pop() {
            match effect {
                Effect::Notify(entity_id) => {
                    for observer in &self.observers[&entity_id] {
                        (observer.callback)(self);
                    }
                }
                Effect::Emit { source, event } => {
                    // Route event to subscribers
                }
            }
        }
    }
}
```

### Pattern 4: Two-Phase Event Dispatch

**What:** Events traverse element tree twice: capture (root→target), bubble (target→root).

**When:** All mouse and keyboard events.

**Why:** Allows parent components to intercept events before children handle them (capture), or react to unhandled events (bubble). Enables hierarchical shortcuts and event delegation.

**Example:**
```rust
// In ora
pub enum EventPhase {
    Capture,
    Bubble,
}

impl DispatchTree {
    pub fn dispatch_event(&self, event: &Event) -> bool {
        let path = match event {
            Event::Mouse { position, .. } => self.hit_test(*position),
            Event::Keyboard { .. } => self.focus_path.clone(),
        };

        // Capture phase: root to target
        for node in path.iter() {
            if node.handle_event(event, EventPhase::Capture) {
                return true; // Stopped propagation
            }
        }

        // Bubble phase: target to root
        for node in path.iter().rev() {
            if node.handle_event(event, EventPhase::Bubble) {
                return true;
            }
        }

        false
    }
}
```

### Pattern 5: Tailwind-Style Builder API

**What:** Fluent builder pattern for styling elements with semantic method names.

**When:** Constructing UI element trees in View::render().

**Why:** Concise, readable, type-safe styling. Familiar to web developers. Enables IDE autocomplete.

**Example:**
```rust
// In ora
impl Element for Div {
    fn render(self, cx: &mut RenderContext) -> impl Element {
        div()
            .flex()
            .flex_col()
            .justify_center()
            .items_center()
            .gap_4()
            .bg(cx.theme().colors.surface)
            .rounded_lg()
            .p_4()
            .child(
                text("Hello")
                    .text_lg()
                    .font_bold()
                    .text_color(cx.theme().colors.on_surface)
            )
    }
}

// Builder implementation
pub struct Div {
    style: Style,
    children: Vec<Box<dyn Element>>,
}

impl Div {
    pub fn flex(mut self) -> Self {
        self.style.display = Display::Flex;
        self
    }

    pub fn bg(mut self, color: Color) -> Self {
        self.style.background = Some(color);
        self
    }

    pub fn child(mut self, child: impl Element + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }
}
```

## Anti-Patterns to Avoid

### Anti-Pattern 1: Direct Scene Mutation in Prepaint

**What:** Writing draw commands to Scene during prepaint phase.

**Why bad:** Prepaint must only compute layout and commit hitboxes. Scene mutation belongs in paint phase. Violating this causes render order issues and hit test failures.

**Instead:** Store paint state during prepaint (colors, text content, etc.), write to Scene in paint phase.

**Example of wrong:**
```rust
// WRONG - mutating Scene in prepaint
fn prepaint(&mut self, cx: &mut PrepaintContext) {
    let bounds = cx.layout(self.layout_id);
    cx.scene.quad(bounds, self.color); // DON'T DO THIS
}
```

**Correct approach:**
```rust
// CORRECT - defer Scene mutation to paint
fn prepaint(&mut self, cx: &mut PrepaintContext) -> PaintState {
    let bounds = cx.layout(self.layout_id);
    PaintState { bounds, color: self.color }
}

fn paint(&mut self, cx: &mut PaintContext, state: &PaintState) {
    cx.scene.quad(state.bounds, state.color); // OK in paint phase
}
```

### Anti-Pattern 2: Synchronous Observer Execution

**What:** Calling observer callbacks immediately when cx.notify() is invoked.

**Why bad:** Creates reentrancy issues. Observer may mutate state while original update is still in progress. Can cause borrow checker violations or panic on RefCell.

**Instead:** Queue effects, flush after update completes using deferred execution pattern.

### Anti-Pattern 3: Multiple Taffy Layout Passes per Frame

**What:** Calling request_layout() multiple times for same element, or re-running layout after paint starts.

**Why bad:** Expensive. Taffy flexbox computation is O(n) in element count. Multiple passes waste CPU, cause frame drops.

**Instead:** Single layout pass in prepaint phase. Cache layout results. Only re-layout if window size changes or element tree structure changes.

### Anti-Pattern 4: Storing Mutable References in Elements

**What:** Elements holding &mut references to external state.

**Why bad:** Rust borrow checker prevents this. Elements are short-lived (reconstructed each frame). Can't hold references across frames.

**Instead:** Use Entity<T> handles. Access via cx.read(entity) or cx.update(entity, |data, cx| ...).

### Anti-Pattern 5: Platform-Specific Code in Element Trait

**What:** Elements directly calling Metal or Vulkan APIs.

**Why bad:** Breaks cross-platform abstraction. Element should not know about GPU backend.

**Instead:** Elements write to Scene (abstract draw commands). Scene translates to Blade. Blade generates platform-specific GPU commands.

## Scalability Considerations

### At 100 Elements (Simple App)

| Concern | Approach |
|---------|----------|
| Layout | Direct Taffy flexbox, no optimization needed |
| Rendering | Single-pass Scene construction, trivial batching |
| Event Dispatch | Linear tree traversal, no index needed |
| Memory | Entity<T> in SlotMap, small overhead per entity |

**Bottlenecks:** None expected. Framework overhead dominates app logic.

### At 10K Elements (Large Document)

| Concern | Approach |
|---------|----------|
| Layout | Virtualization critical. Only layout visible elements (viewport culling). |
| Rendering | Scene batching by material/shader. Instanced rendering for repeated elements. |
| Event Dispatch | Spatial index for hit testing (R-tree or grid). Focus path remains linear. |
| Memory | Element pooling to avoid per-frame allocation. Reuse Taffy nodes where possible. |

**Bottlenecks:** Layout computation (Taffy), text shaping (glyphon), event dispatch (hit testing). Mitigation: viewport culling, dirty tracking, spatial indexing.

**Existing muda optimization:** Single-batch text rendering (collect all text, prepare once). Should preserve this in ora.

### At 1M Elements (Theoretical - Unlikely in UI)

| Concern | Approach |
|---------|----------|
| Layout | Mandatory virtualization. Only layout viewport + buffer. Infinite scrolling pattern. |
| Rendering | Aggressive batching, instancing, and culling. Offload to GPU where possible. |
| Event Dispatch | Hierarchical spatial index (quadtree). Cache hit test results. |
| Memory | Streaming architecture. Unload offscreen elements. Lazy entity creation. |

**Bottlenecks:** Memory bandwidth, GPU command buffer size, CPU→GPU transfer. Framework likely not viable at this scale without architectural changes (streaming, level-of-detail).

**Recommendation for ora:** Design for 10K element scale. Add profiling hooks to identify bottlenecks. Defer 1M optimization until proven necessary.

## Ora-Specific Adaptations

### Leverage Existing muda Infrastructure

**Current muda strengths to preserve:**

1. **RenderModel projection:** Keep ViewModelBuilder pattern. Ora elements consume RenderModel, not raw Document.

2. **Single-batch text rendering:** Existing UITextRenderer collects all text, prepares once with glyphon. Ora should expose similar batching API.

3. **Scissor clipping:** Five-phase rendering with explicit scissor rects prevents overflow. Ora should support per-element scissor regions.

4. **Design system tokens:** Existing design_system/ provides tokens. Ora elements should reference tokens, not hardcoded values.

5. **Component hierarchy:** TextArea, Gutter, etc. are separate components. Ora should enable similar modular composition.

### Integration Points with core_editor

**Ora framework responsibilities:**
- Element tree construction and lifecycle
- Layout computation (Taffy integration)
- Event dispatch (keyboard, mouse, focus)
- GPU rendering (wgpu integration via Blade or direct)
- Window management (winit integration)
- Reactivity (Entity/Model observation)

**core_editor responsibilities (unchanged):**
- Document and TextBuffer state
- EditorCommand dispatch
- Undo/redo history
- Syntax highlighting
- Workspace and view management
- RenderModel projection

**Bridge between ora and core_editor:**
```rust
// In ora
pub struct EditorView {
    model: Model<EditorViewModel>, // Ora reactive model
}

impl Render for EditorView {
    fn render(&mut self, cx: &mut ViewContext) -> impl Element {
        let render_model = self.model.read(cx).render_model.clone();

        // Ora elements consume RenderModel
        div()
            .flex_col()
            .child(gutter_component(&render_model.gutter))
            .child(text_area_component(&render_model.text_area))
            .child(status_line_component(&render_model.status))
    }
}

// In core_editor (unchanged)
pub struct App {
    workspace: Workspace,
    dispatcher: CommandDispatcher,
    // ...
}

impl App {
    pub fn build_render_model(&self, viewport_height: usize) -> RenderModel {
        // Existing projection logic
    }
}
```

### Build Order Implications

**Phase 1: Foundation (Must Exist First)**
1. App and Entity<T> system (centralized ownership)
2. Context types (App, Context<T>, AsyncContext)
3. Basic Element trait (without layout or paint)
4. Window wrapper (basic window creation, no rendering)

**Phase 2: Reactive System**
5. Observation mechanism (cx.observe, cx.notify)
6. Event emission (cx.emit, cx.subscribe)
7. Effect queue and flush logic

**Phase 3: Layout Engine**
8. Taffy integration (request_layout, compute_layout)
9. LayoutId and Bounds types
10. Prepaint phase implementation

**Phase 4: Rendering Pipeline**
11. Scene structure (draw command buffer)
12. Paint phase implementation
13. wgpu or Blade integration (GPU submission)
14. Platform abstraction (Metal/Vulkan/DX)

**Phase 5: Event System**
15. DispatchTree and focus management
16. Two-phase dispatch (capture/bubble)
17. Hit testing and hitbox tracking
18. Mouse and keyboard event normalization

**Phase 6: Element Library**
19. Basic elements (div, text, image)
20. Styled element API (builder pattern)
21. Layout elements (flex, stack, grid)
22. Interactive elements (button, input)

**Phase 7: Integration**
23. Bridge to core_editor (View wrapper)
24. Adapt existing components (TextArea, Gutter) to ora elements
25. Port design system to ora styling API
26. Migrate GpuRenderer to ora Window

**Critical Dependencies:**
- Taffy depends on Element trait (request_layout signature)
- Scene depends on Element trait (paint signature)
- DispatchTree depends on Frame (hitbox registry)
- Element library depends on all above (App, Layout, Paint, Event)
- Integration depends on Element library (can't migrate components without elements)

**Recommendation:** Build incrementally. Phase 1-3 can be prototyped without GPU rendering (text output or empty window). Phase 4 enables visual feedback. Phase 5 enables interaction. Phase 6 provides reusable building blocks. Phase 7 completes migration.

## Sources

### HIGH Confidence (Official GPUI Documentation)
- [GPUI Context Architecture](https://github.com/zed-industries/zed/blob/main/crates/gpui/docs/contexts.md) - App, Context<T>, Entity<T> types
- [GPUI Ownership and Data Flow](https://zed.dev/blog/gpui-ownership) - Centralized ownership pattern, observation mechanism
- [GPUI README](https://github.com/zed-industries/zed/blob/main/crates/gpui/README.md) - Three operational registers (Entity, View, Element)

### MEDIUM Confidence (Technical Articles and Community Documentation)
- [GPUI Technical Overview](https://beckmoulton.medium.com/gpui-a-technical-overview-of-the-high-performance-rust-ui-framework-powering-zed-ac65975cda9f) - GPU acceleration, hybrid immediate/retained mode
- [GPUI Framework DeepWiki](https://deepwiki.com/zed-industries/zed/2.2-gpui-framework) - Element trait, layout pipeline, rendering phases
- [Event Flow DeepWiki](https://deepwiki.com/zed-industries/zed/2.4-keybinding-and-action-dispatch) - Two-phase dispatch, event routing
- [Focus Management DeepWiki](https://deepwiki.com/zed-industries/zed/2.5-keybinding-and-action-system) - Focus paths, dispatch tree
- [Zed 120 FPS Blog](https://zed.dev/blog/120fps) - Metal pipeline optimization, rendering performance
- [Zed Videogame Blog](https://zed.dev/blog/videogame) - Leveraging GPU, 120 FPS target

### MEDIUM Confidence (Related Technologies)
- [Taffy Flexbox Engine](https://github.com/DioxusLabs/taffy) - Rust flexbox implementation used by GPUI
- [glyphon Text Renderer](https://github.com/grovesNL/glyphon) - wgpu text rendering, atlas packing

### LOW Confidence (Unverified, Needs Official Source Confirmation)
- Blade graphics abstraction details (mentioned in articles but not documented)
- GPUI 2026 API changes (App vs AppContext) - recent enough that official docs may be incomplete
- Scene structure internals (draw command format not publicly documented)

### Notes on Information Currency
- GPUI underwent major API changes in 2025-2026 (Context types replaced AppContext/WindowContext/ViewContext)
- Most comprehensive information comes from Zed's own blog posts and official docs
- Community documentation (DeepWiki) is recent (January 2026) but should be verified against source code
- Training data about GPUI is likely outdated given recent API changes
