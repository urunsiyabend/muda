# Phase 5: Element Library - Research

**Researched:** 2026-01-29
**Domain:** Fluent builder APIs, layout containers, styled primitives, interactive elements
**Confidence:** MEDIUM

## Summary

Element libraries in modern UI frameworks center on fluent builder APIs that enable expressive, chainable method calls for element construction and styling. The standard approach uses the owned builder pattern (methods return `self` not `&mut self`) to enable natural chaining like `div().flex().gap(px(4)).bg(color)`, with each method consuming and returning the builder by value. This pattern combines readability with compile-time safety, making invalid states unrepresentable.

Layout systems follow CSS Flexbox semantics with `justify-content` for main-axis alignment and `align-items` for cross-axis alignment, using explicit direction configuration (`div().row()` or `div().column()`) rather than separate container types. Stack containers for z-axis layering rely on paint order (last child painted on top) rather than explicit z-index values, matching immediate-mode UI patterns where elements have no persistent identity between frames. Image elements require async loading with placeholders, texture caching to prevent memory leaks, and `object-fit` semantics (contain/cover) for scaling. Button components use variant systems (primary/secondary/ghost) with multiple states (enabled/hover/active/disabled/loading) to provide comprehensive interaction feedback.

Critical architectural decisions: builder methods must return `Self` (owned, not borrowed) for natural chaining; layout containers use CSS Flexbox alignment semantics for familiarity; stack z-order follows paint order (implicit, not explicit z-index); image textures must be cached and properly disposed to avoid GPU memory leaks; button variants should include hover/active states as part of the variant definition (not separate configuration) for consistency; spacing accepts both raw pixel values and design tokens (8px base unit scale) for flexibility during development.

**Primary recommendation:** Use owned builder pattern with methods returning `Self`, implement CSS Flexbox alignment semantics for layout containers, use paint-order-based z-layering for stack, implement async image loading with texture caching and proper disposal, and design button variants as complete state packages including hover/active styling.

## Standard Stack

This phase uses existing Rust ecosystem crates for image loading and async operations.

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| image | 0.25+ | Image decoding (PNG, JPEG, etc.) | De facto standard for image format decoding in Rust |
| wgpu | (current) | GPU texture management | Already used for rendering, textures integrate naturally |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tokio or async-std | (current) | Async image loading | Background image loading without blocking UI thread |
| lru | 0.12+ | LRU texture cache | Limit texture memory usage, evict least-recently-used textures |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Owned builder (return Self) | Borrowed builder (return &mut Self) | Borrowed requires binding, less natural chaining, but slightly lower overhead for large structs |
| Paint-order z-layering | Explicit z-index field | Z-index adds complexity, requires sorting on every frame in immediate mode UI |
| CSS Flexbox semantics | Custom alignment enum | Custom alignment creates learning curve, CSS Flexbox is familiar to web developers |

**Installation:**
```bash
cargo add image
cargo add lru
# async runtime already present (check Cargo.toml for tokio/async-std)
```

## Architecture Patterns

### Recommended Project Structure
```
wgpu_client/src/
├── elements/
│   ├── mod.rs              # Public API exports
│   ├── div.rs              # Div builder with styling
│   ├── text.rs             # Text builder
│   ├── button.rs           # Button with variants
│   ├── image.rs            # Image with async loading
│   ├── stack.rs            # Stack container (z-layering)
│   └── containers.rs       # Row/Column helpers
├── style/
│   ├── units.rs            # px(), pct(), rem() unit types
│   ├── alignment.rs        # JustifyContent, AlignItems enums
│   └── spacing.rs          # Spacing token enums (Sm, Md, Lg)
└── render/
    └── texture_cache.rs    # LRU texture cache with disposal
```

### Pattern 1: Owned Builder with Self Return
**What:** Builder methods consume and return `self` by value for natural chaining
**When to use:** All element builders (Div, Text, Button, Image, Stack)
**Example:**
```rust
// Source: Rust builder pattern best practices (rust-unofficial.github.io/patterns)
pub struct Div {
    style: StyleBuilder,
    children: Vec<AnyElement>,
}

impl Div {
    pub fn new() -> Self {
        Self {
            style: StyleBuilder::default(),
            children: Vec::new(),
        }
    }

    // Return Self (owned), not &mut Self
    pub fn bg(mut self, color: Color) -> Self {
        self.style.background = Some(color);
        self // Consume and return by value
    }

    pub fn padding(mut self, amount: impl Into<Length>) -> Self {
        self.style.padding = amount.into();
        self
    }

    pub fn child(mut self, element: impl Into<AnyElement>) -> Self {
        self.children.push(element.into());
        self
    }
}

// Usage: Natural chaining without bindings
div()
    .bg(red())
    .padding(px(16))
    .child(text("Hello"))
```

### Pattern 2: Unit Types with Into Conversions
**What:** Explicit unit types (Px, Pct, Rem) with Into trait for ergonomics
**When to use:** All dimensional properties (width, height, padding, gap)
**Example:**
```rust
// Source: Tailwind-style unit system pattern
#[derive(Clone, Copy, Debug)]
pub enum Length {
    Px(f32),
    Pct(f32),
    Rem(f32),
}

// Constructor functions for ergonomics
pub fn px(value: f32) -> Length { Length::Px(value) }
pub fn pct(value: f32) -> Length { Length::Pct(value) }
pub fn rem(value: f32) -> Length { Length::Rem(value) }

// Accept Into<Length> for flexibility
impl Div {
    pub fn width(mut self, w: impl Into<Length>) -> Self {
        self.style.width = Some(w.into());
        self
    }
}

// Allow raw numbers to convert to pixels (common case)
impl From<f32> for Length {
    fn from(px: f32) -> Self { Length::Px(px) }
}

// Usage: Explicit when needed, implicit for pixels
div().width(px(200))     // Explicit px
div().width(pct(50))     // Explicit pct
div().width(200.0)       // Implicit px via From
```

### Pattern 3: CSS Flexbox Alignment Semantics
**What:** Use CSS terminology for alignment (justify-content, align-items)
**When to use:** Row and Column layout containers
**Example:**
```rust
// Source: CSS Flexbox specification (MDN)
#[derive(Clone, Copy, Debug, Default)]
pub enum JustifyContent {
    #[default]
    Start,       // Pack items at start of main axis
    End,         // Pack items at end of main axis
    Center,      // Center items on main axis
    SpaceBetween, // First/last flush, equal space between
    SpaceAround,  // Equal space around each item
    SpaceEvenly,  // Equal space between and around
}

#[derive(Clone, Copy, Debug, Default)]
pub enum AlignItems {
    Start,       // Align to start of cross axis
    End,         // Align to end of cross axis
    #[default]
    Center,      // Center on cross axis
    Stretch,     // Stretch to fill cross axis
}

impl Div {
    pub fn row(mut self) -> Self {
        self.style.flex_direction = FlexDirection::Row;
        self
    }

    pub fn justify(mut self, j: JustifyContent) -> Self {
        self.style.justify_content = j;
        self
    }

    pub fn items(mut self, a: AlignItems) -> Self {
        self.style.align_items = a;
        self
    }
}

// Usage: Familiar to web developers
div()
    .row()
    .justify(JustifyContent::Center)
    .items(AlignItems::Start)
```

### Pattern 4: Paint-Order-Based Z-Layering for Stack
**What:** Stack children paint in order, last child on top (no z-index)
**When to use:** Overlapping elements (background + foreground, modal + backdrop)
**Example:**
```rust
// Source: CSS painting order, immediate-mode UI pattern
pub struct Stack {
    children: Vec<AnyElement>,
    style: StyleBuilder,
}

impl Stack {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            style: StyleBuilder::default(),
        }
    }

    pub fn child(mut self, element: impl Into<AnyElement>) -> Self {
        self.children.push(element.into());
        self // Later children paint on top
    }
}

// In paint implementation
impl Element for Stack {
    fn paint(&mut self, cx: &mut PaintContext) {
        // Paint in order: first child = bottom, last child = top
        for child in &mut self.children {
            child.paint(cx);
        }
    }
}

// Usage: Order determines z-layering
stack()
    .child(div().bg(gray()))  // Background (bottom)
    .child(div().bg(white())) // Foreground (top)
```

### Pattern 5: Async Image Loading with Placeholders
**What:** Load images on background thread, show placeholder until ready
**When to use:** Image element with file or URL source
**Example:**
```rust
// Source: ratatui-image async loading pattern (crates.io/crates/ratatui-image)
pub struct Image {
    source: ImageSource,
    object_fit: ObjectFit,
    placeholder_color: Color,
    state: ImageState,
}

enum ImageState {
    Loading,
    Loaded(TextureId),
    Error,
}

impl Image {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        let source = ImageSource::File(path.into());
        // Trigger async load
        spawn_image_load(source.clone());

        Self {
            source,
            object_fit: ObjectFit::Contain,
            placeholder_color: Color::rgba(0.5, 0.5, 0.5, 1.0),
            state: ImageState::Loading,
        }
    }

    pub fn object_fit(mut self, fit: ObjectFit) -> Self {
        self.object_fit = fit;
        self
    }
}

// Async loading helper
fn spawn_image_load(source: ImageSource) {
    tokio::spawn(async move {
        match source {
            ImageSource::File(path) => {
                let img = image::ImageReader::open(path)?
                    .decode()?;
                // Upload to GPU, cache texture, notify UI
                upload_texture(img).await
            }
        }
    });
}
```

### Pattern 6: LRU Texture Cache with Explicit Disposal
**What:** Cache textures by path/URL, evict LRU when limit reached, dispose GPU resources
**When to use:** Image element texture management
**Example:**
```rust
// Source: texture-cache crate pattern (docs.rs/texture-cache)
use lru::LruCache;

pub struct TextureCache {
    cache: LruCache<PathBuf, Texture>,
    device: Arc<wgpu::Device>,
}

impl TextureCache {
    pub fn new(device: Arc<wgpu::Device>, capacity: usize) -> Self {
        Self {
            cache: LruCache::new(capacity.try_into().unwrap()),
            device,
        }
    }

    pub fn get_or_load(&mut self, path: &Path) -> Option<&Texture> {
        if !self.cache.contains(path) {
            // Load and decode image
            let img = image::open(path).ok()?;
            let rgba = img.to_rgba8();

            // Create GPU texture
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                // ... texture config
            });

            // Insert into cache (may evict LRU)
            if let Some(evicted) = self.cache.push(path.to_owned(), texture) {
                // CRITICAL: Dispose GPU resources explicitly
                evicted.destroy();
            }
        }

        self.cache.get(path)
    }
}

// CRITICAL: On drop, dispose all textures
impl Drop for TextureCache {
    fn drop(&mut self) {
        for (_, texture) in self.cache.iter() {
            texture.destroy();
        }
    }
}
```

### Pattern 7: Button Variants as Complete State Packages
**What:** Each button variant defines all states (normal, hover, active) together
**When to use:** Button component with visual variants (primary, secondary, ghost)
**Example:**
```rust
// Source: shadcn-ui button variants pattern (shadcnstudio.com)
pub struct Button {
    label: String,
    variant: ButtonVariant,
    on_click: Option<Box<dyn Fn()>>,
}

pub enum ButtonVariant {
    Primary,
    Secondary,
    Ghost,
    Destructive,
}

impl ButtonVariant {
    fn style(&self, state: InteractionState) -> Style {
        match (self, state) {
            // Primary variant states
            (Self::Primary, InteractionState::Enabled) => Style {
                bg: blue(600),
                text: white(),
            },
            (Self::Primary, InteractionState::Hover) => Style {
                bg: blue(700),
                text: white(),
            },
            (Self::Primary, InteractionState::Active) => Style {
                bg: blue(800),
                text: white(),
            },

            // Secondary variant states
            (Self::Secondary, InteractionState::Enabled) => Style {
                bg: gray(200),
                text: gray(900),
            },
            // ... etc
        }
    }
}

// Usage: Variant includes all state styling
button("Submit")
    .primary()  // Sets variant, includes hover/active
    .on_click(|| { /* ... */ })
```

### Pattern 8: Spacing Tokens with Raw Value Fallback
**What:** Accept both design tokens (Spacing::Md) and raw pixels for flexibility
**When to use:** Gap, padding, margin properties during development
**Example:**
```rust
// Source: Design token scale patterns (8px base unit system)
#[derive(Clone, Copy, Debug)]
pub enum Spacing {
    Xs,   // 4px  (0.5 * base)
    Sm,   // 8px  (1.0 * base)
    Md,   // 16px (2.0 * base)
    Lg,   // 24px (3.0 * base)
    Xl,   // 32px (4.0 * base)
    Xxl,  // 48px (6.0 * base)
}

impl Spacing {
    pub fn to_px(&self) -> f32 {
        match self {
            Self::Xs => 4.0,
            Self::Sm => 8.0,
            Self::Md => 16.0,
            Self::Lg => 24.0,
            Self::Xl => 32.0,
            Self::Xxl => 48.0,
        }
    }
}

// Accept either Spacing token or raw f32
#[derive(Clone, Copy)]
pub enum Gap {
    Token(Spacing),
    Raw(f32),
}

impl From<Spacing> for Gap {
    fn from(s: Spacing) -> Self { Self::Token(s) }
}

impl From<f32> for Gap {
    fn from(px: f32) -> Self { Self::Raw(px) }
}

impl Div {
    pub fn gap(mut self, g: impl Into<Gap>) -> Self {
        self.style.gap = g.into();
        self
    }
}

// Usage: Tokens during production, raw during prototyping
div().gap(Spacing::Md)  // Token (16px)
div().gap(12.0)         // Raw pixels
```

### Anti-Patterns to Avoid

- **Borrowed builder pattern (&mut self return):** Requires binding (`let mut d = div(); d.bg(red); d.padding(8);`), less ergonomic than owned pattern chaining
- **Implicit pixel units:** Accepting raw numbers without unit constructors creates ambiguity (is `200` pixels or percent?). Use `px(200)` for clarity
- **Explicit z-index on stack children:** Adds complexity and sorting overhead. Paint order (last on top) is simpler and matches immediate-mode UI
- **Synchronous image loading:** Blocks UI thread during decode. Always load images asynchronously with placeholders
- **Unbounded texture cache:** GPU memory leaks. Use LRU cache with capacity limit and explicit texture disposal
- **Separate hover/active configuration:** Leads to inconsistent state definitions. Include all states in variant definition
- **Complex typestate builders for UI:** Typestate pattern enforces required fields at compile time but adds complexity. UI elements have sensible defaults, typestate overhead not justified

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Image format decoding | Custom PNG/JPEG decoder | `image` crate with `ImageReader::open().decode()` | Supports 10+ formats, battle-tested, handles edge cases (interlaced PNG, progressive JPEG, color profiles) |
| Texture upload to GPU | Manual buffer creation and copying | `wgpu::Device::create_texture_with_data()` | Handles alignment, format conversion, mipmap generation |
| LRU cache eviction | Custom linked list + hashmap | `lru` crate with `LruCache::new(capacity)` | Lock-free, O(1) operations, battle-tested |
| Async runtime for image loading | Custom thread pool | `tokio` or `async-std` runtime | Handles task scheduling, work stealing, cross-platform |
| Object-fit scaling math | Manual aspect ratio calculation | Reuse CSS object-fit semantics (contain/cover/fill) | Well-defined behavior, familiar to developers |

**Key insight:** Image decoding and GPU texture management have numerous edge cases (color spaces, interlacing, alignment requirements, format quirks). The `image` crate and `wgpu` abstractions handle these correctly. Custom implementations will have bugs on uncommon formats or edge cases.

## Common Pitfalls

### Pitfall 1: Builder Methods Returning &mut Self Instead of Self
**What goes wrong:** Chaining requires intermediate bindings, code becomes verbose and unnatural
**Why it happens:** Attempting to optimize by avoiding moves, or following Java/C# builder patterns
**How to avoid:** Return `Self` (owned) from all builder methods. Rust's move semantics make this efficient (no deep copies)
**Warning signs:** Code like `let mut d = div(); d.bg(red); d.padding(8); d` instead of `div().bg(red).padding(8)`

### Pitfall 2: Forgetting to Dispose GPU Textures on Cache Eviction
**What goes wrong:** GPU memory leaks as textures accumulate. Long-running app crashes or slows down
**Why it happens:** LRU cache evicts textures from CPU-side map but GPU resources aren't freed
**How to avoid:** Call `texture.destroy()` when LRU cache evicts an entry. Also dispose all textures in cache's `Drop` impl
**Warning signs:** GPU memory usage increases over time. Performance degrades as more images are loaded. Eventual crash with out-of-memory error

### Pitfall 3: Synchronous Image Loading on UI Thread
**What goes wrong:** UI freezes while loading images, especially large images or slow I/O
**Why it happens:** Loading image in `render()` method or during element construction
**How to avoid:** Spawn async task to load image, show placeholder color until loaded, notify UI when ready
**Warning signs:** UI stutters or freezes when scrolling image-heavy content. Frame drops when images appear

### Pitfall 4: Not Caching Image Textures by Path
**What goes wrong:** Same image loaded and uploaded to GPU multiple times, wasting memory and bandwidth
**Why it happens:** Each Image element independently loads texture without checking if already loaded
**How to avoid:** Central TextureCache indexed by file path/URL. Image elements query cache before loading
**Warning signs:** GPU memory usage higher than expected. Same image load logs appear multiple times. Slower than expected image rendering

### Pitfall 5: Implementing Object-Fit with Incorrect Aspect Ratio Math
**What goes wrong:** Images stretched, squished, or cropped incorrectly
**Why it happens:** Off-by-one errors in aspect ratio calculation, swapping width/height, incorrect min/max
**How to avoid:** Use well-tested CSS object-fit semantics. Contain: scale to fit inside (letterbox). Cover: scale to fill (crop). Fill: stretch
**Warning signs:** Images appear distorted. Cover mode leaves gaps. Contain mode crops image

### Pitfall 6: Button Hover/Active States Not Matching Variant Style
**What goes wrong:** Primary button has secondary button's hover color. Inconsistent visual feedback
**Why it happens:** Hover/active styles configured separately from variant, easy to forget to update both
**How to avoid:** Each ButtonVariant defines all states (normal, hover, active, disabled) as a unit. No separate hover configuration
**Warning signs:** Button styles inconsistent between states. Hover looks wrong for variant

### Pitfall 7: Stack Children with Explicit Z-Index Causing Sort Every Frame
**What goes wrong:** Performance degrades as stack children increase. O(n log n) sort on every frame
**Why it happens:** Using explicit z-index field requires sorting children by z-index before painting
**How to avoid:** Use paint order for z-layering. Last child paints last (on top). No sorting needed
**Warning signs:** Frame time increases with number of stack children. Profiler shows time in sort function

### Pitfall 8: Accepting Raw Numbers for Dimensions Without Units
**What goes wrong:** Ambiguous whether `200` means pixels, percent, or rem. Hard to grep for pixel values
**Why it happens:** Trying to make API terser by allowing `div().width(200)` instead of `div().width(px(200))`
**How to avoid:** Require explicit units via constructor functions. Only implement `From<f32>` if pixels are the overwhelming default
**Warning signs:** Bugs where developer meant percent but got pixels (or vice versa). Unclear units in code review

## Code Examples

Verified patterns from official sources:

### Complete Div Builder with Fluent API
```rust
// Pattern: Owned builder with all common styling methods
// Source: Rust builder pattern (rust-unofficial.github.io/patterns)

use crate::style::*;
use crate::Color;

pub struct Div {
    style: StyleBuilder,
    children: Vec<AnyElement>,
    interaction: Option<InteractionHandlers>,
}

pub fn div() -> Div {
    Div::new()
}

impl Div {
    pub fn new() -> Self {
        Self {
            style: StyleBuilder::default(),
            children: Vec::new(),
            interaction: None,
        }
    }

    // Layout
    pub fn row(mut self) -> Self {
        self.style.flex_direction = FlexDirection::Row;
        self
    }

    pub fn column(mut self) -> Self {
        self.style.flex_direction = FlexDirection::Column;
        self
    }

    pub fn justify(mut self, j: JustifyContent) -> Self {
        self.style.justify_content = j;
        self
    }

    pub fn items(mut self, a: AlignItems) -> Self {
        self.style.align_items = a;
        self
    }

    pub fn gap(mut self, g: impl Into<Gap>) -> Self {
        self.style.gap = Some(g.into());
        self
    }

    // Sizing
    pub fn w(mut self, width: impl Into<Length>) -> Self {
        self.style.width = Some(width.into());
        self
    }

    pub fn h(mut self, height: impl Into<Length>) -> Self {
        self.style.height = Some(height.into());
        self
    }

    // Spacing
    pub fn p(mut self, padding: impl Into<Length>) -> Self {
        self.style.padding = Edges::all(padding.into());
        self
    }

    pub fn px(mut self, padding: impl Into<Length>) -> Self {
        let p = padding.into();
        self.style.padding.left = p;
        self.style.padding.right = p;
        self
    }

    pub fn py(mut self, padding: impl Into<Length>) -> Self {
        let p = padding.into();
        self.style.padding.top = p;
        self.style.padding.bottom = p;
        self
    }

    // Colors
    pub fn bg(mut self, color: Color) -> Self {
        self.style.background = Some(color);
        self
    }

    pub fn border(mut self, width: f32, color: Color) -> Self {
        self.style.border_width = width;
        self.style.border_color = Some(color);
        self
    }

    pub fn rounded(mut self, radius: f32) -> Self {
        self.style.border_radius = radius;
        self
    }

    // Children
    pub fn child(mut self, element: impl Into<AnyElement>) -> Self {
        self.children.push(element.into());
        self
    }

    pub fn children(mut self, elements: Vec<AnyElement>) -> Self {
        self.children.extend(elements);
        self
    }

    // Interaction
    pub fn on_click(mut self, handler: impl Fn() + 'static) -> Self {
        self.interaction.get_or_insert_with(Default::default)
            .on_click = Some(Box::new(handler));
        self
    }
}
```

### Image Element with Async Loading
```rust
// Pattern: Async image loading with placeholder and object-fit
// Source: ratatui-image async pattern + CSS object-fit

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct Image {
    source: ImageSource,
    object_fit: ObjectFit,
    placeholder_color: Color,
    texture_id: Option<TextureId>,
}

pub enum ImageSource {
    File(PathBuf),
    Bytes(Arc<[u8]>),
}

#[derive(Clone, Copy, Debug)]
pub enum ObjectFit {
    Contain,  // Scale to fit inside, preserve aspect ratio (letterbox)
    Cover,    // Scale to fill, preserve aspect ratio (crop)
    Fill,     // Stretch to fill (distort)
}

pub fn img(path: impl Into<PathBuf>) -> Image {
    Image::from_file(path.into())
}

pub fn img_from_bytes(bytes: impl Into<Arc<[u8]>>) -> Image {
    Image::from_bytes(bytes.into())
}

impl Image {
    pub fn from_file(path: PathBuf) -> Self {
        // Trigger async load
        let source = ImageSource::File(path);
        spawn_image_load(source.clone());

        Self {
            source,
            object_fit: ObjectFit::Contain,
            placeholder_color: Color::rgba(0.7, 0.7, 0.7, 1.0),
            texture_id: None,
        }
    }

    pub fn from_bytes(bytes: Arc<[u8]>) -> Self {
        let source = ImageSource::Bytes(bytes);
        spawn_image_load(source.clone());

        Self {
            source,
            object_fit: ObjectFit::Contain,
            placeholder_color: Color::rgba(0.7, 0.7, 0.7, 1.0),
            texture_id: None,
        }
    }

    pub fn object_fit(mut self, fit: ObjectFit) -> Self {
        self.object_fit = fit;
        self
    }

    pub fn placeholder_color(mut self, color: Color) -> Self {
        self.placeholder_color = color;
        self
    }
}

// Async loading implementation
fn spawn_image_load(source: ImageSource) {
    tokio::spawn(async move {
        let img_result = match &source {
            ImageSource::File(path) => {
                image::ImageReader::open(path)?.decode()
            }
            ImageSource::Bytes(bytes) => {
                image::load_from_memory(bytes)?
            }
        };

        // Upload to GPU via texture cache
        let texture_id = TEXTURE_CACHE.lock().unwrap()
            .upload_image(source, img_result);

        // Notify UI to re-render
        request_redraw();
    });
}

// Object-fit scaling calculations
impl ObjectFit {
    pub fn compute_bounds(&self, image_size: (f32, f32), container_size: (f32, f32)) -> Rect {
        let (img_w, img_h) = image_size;
        let (cont_w, cont_h) = container_size;

        match self {
            ObjectFit::Fill => {
                // Stretch to fill container
                Rect::new(0.0, 0.0, cont_w, cont_h)
            }
            ObjectFit::Contain => {
                // Scale to fit inside, preserve aspect ratio
                let img_aspect = img_w / img_h;
                let cont_aspect = cont_w / cont_h;

                let (w, h) = if img_aspect > cont_aspect {
                    // Image wider than container, fit width
                    (cont_w, cont_w / img_aspect)
                } else {
                    // Image taller than container, fit height
                    (cont_h * img_aspect, cont_h)
                };

                // Center in container
                let x = (cont_w - w) / 2.0;
                let y = (cont_h - h) / 2.0;
                Rect::new(x, y, w, h)
            }
            ObjectFit::Cover => {
                // Scale to fill, preserve aspect ratio, crop excess
                let img_aspect = img_w / img_h;
                let cont_aspect = cont_w / cont_h;

                let (w, h) = if img_aspect > cont_aspect {
                    // Image wider, fit height (crop sides)
                    (cont_h * img_aspect, cont_h)
                } else {
                    // Image taller, fit width (crop top/bottom)
                    (cont_w, cont_w / img_aspect)
                };

                // Center in container (excess will be clipped)
                let x = (cont_w - w) / 2.0;
                let y = (cont_h - h) / 2.0;
                Rect::new(x, y, w, h)
            }
        }
    }
}
```

### Button with Variants
```rust
// Pattern: Button variants as complete state packages
// Source: shadcn-ui button component pattern

pub struct Button {
    label: String,
    variant: ButtonVariant,
    disabled: bool,
    loading: bool,
    on_click: Option<Box<dyn Fn()>>,
}

#[derive(Clone, Copy, Debug)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Ghost,
    Destructive,
}

pub fn button(label: impl Into<String>) -> Button {
    Button::new(label.into())
}

impl Button {
    pub fn new(label: String) -> Self {
        Self {
            label,
            variant: ButtonVariant::Primary,
            disabled: false,
            loading: false,
            on_click: None,
        }
    }

    pub fn primary(mut self) -> Self {
        self.variant = ButtonVariant::Primary;
        self
    }

    pub fn secondary(mut self) -> Self {
        self.variant = ButtonVariant::Secondary;
        self
    }

    pub fn ghost(mut self) -> Self {
        self.variant = ButtonVariant::Ghost;
        self
    }

    pub fn destructive(mut self) -> Self {
        self.variant = ButtonVariant::Destructive;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    pub fn on_click(mut self, handler: impl Fn() + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

impl ButtonVariant {
    /// Get style for this variant in the given interaction state
    pub fn style(&self, state: InteractionState) -> ButtonStyle {
        use InteractionState::*;

        match (self, state) {
            // Primary variant
            (Self::Primary, Enabled) => ButtonStyle {
                bg: Color::rgb(0.25, 0.45, 0.85),  // blue-600
                text: Color::WHITE,
                border: None,
            },
            (Self::Primary, Hover) => ButtonStyle {
                bg: Color::rgb(0.20, 0.38, 0.75),  // blue-700
                text: Color::WHITE,
                border: None,
            },
            (Self::Primary, Active) => ButtonStyle {
                bg: Color::rgb(0.15, 0.30, 0.65),  // blue-800
                text: Color::WHITE,
                border: None,
            },
            (Self::Primary, Disabled) => ButtonStyle {
                bg: Color::rgba(0.25, 0.45, 0.85, 0.5),  // blue-600 with opacity
                text: Color::rgba(1.0, 1.0, 1.0, 0.6),
                border: None,
            },

            // Secondary variant
            (Self::Secondary, Enabled) => ButtonStyle {
                bg: Color::rgb(0.95, 0.95, 0.96),  // gray-200
                text: Color::rgb(0.1, 0.1, 0.1),    // gray-900
                border: None,
            },
            (Self::Secondary, Hover) => ButtonStyle {
                bg: Color::rgb(0.90, 0.90, 0.91),  // gray-300
                text: Color::rgb(0.1, 0.1, 0.1),
                border: None,
            },
            (Self::Secondary, Active) => ButtonStyle {
                bg: Color::rgb(0.85, 0.85, 0.86),  // gray-400
                text: Color::rgb(0.1, 0.1, 0.1),
                border: None,
            },
            (Self::Secondary, Disabled) => ButtonStyle {
                bg: Color::rgba(0.95, 0.95, 0.96, 0.5),
                text: Color::rgba(0.1, 0.1, 0.1, 0.4),
                border: None,
            },

            // Ghost variant (transparent with hover)
            (Self::Ghost, Enabled) => ButtonStyle {
                bg: Color::TRANSPARENT,
                text: Color::rgb(0.1, 0.1, 0.1),
                border: None,
            },
            (Self::Ghost, Hover) => ButtonStyle {
                bg: Color::rgba(0.0, 0.0, 0.0, 0.05),
                text: Color::rgb(0.1, 0.1, 0.1),
                border: None,
            },
            (Self::Ghost, Active) => ButtonStyle {
                bg: Color::rgba(0.0, 0.0, 0.0, 0.1),
                text: Color::rgb(0.1, 0.1, 0.1),
                border: None,
            },
            (Self::Ghost, Disabled) => ButtonStyle {
                bg: Color::TRANSPARENT,
                text: Color::rgba(0.1, 0.1, 0.1, 0.4),
                border: None,
            },

            // Destructive variant
            (Self::Destructive, Enabled) => ButtonStyle {
                bg: Color::rgb(0.85, 0.20, 0.20),  // red-600
                text: Color::WHITE,
                border: None,
            },
            (Self::Destructive, Hover) => ButtonStyle {
                bg: Color::rgb(0.75, 0.15, 0.15),  // red-700
                text: Color::WHITE,
                border: None,
            },
            (Self::Destructive, Active) => ButtonStyle {
                bg: Color::rgb(0.65, 0.10, 0.10),  // red-800
                text: Color::WHITE,
                border: None,
            },
            (Self::Destructive, Disabled) => ButtonStyle {
                bg: Color::rgba(0.85, 0.20, 0.20, 0.5),
                text: Color::rgba(1.0, 1.0, 1.0, 0.6),
                border: None,
            },
        }
    }
}
```

### Stack Container with Paint-Order Z-Layering
```rust
// Pattern: Stack with implicit z-order from paint order
// Source: CSS painting order + immediate-mode UI pattern

pub struct Stack {
    children: Vec<AnyElement>,
    style: StyleBuilder,
}

pub fn stack() -> Stack {
    Stack::new()
}

impl Stack {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            style: StyleBuilder::default(),
        }
    }

    pub fn child(mut self, element: impl Into<AnyElement>) -> Self {
        self.children.push(element.into());
        self // Last child added will paint last (on top)
    }

    pub fn children(mut self, elements: Vec<AnyElement>) -> Self {
        self.children.extend(elements);
        self
    }

    // Stack-specific styling (size, padding, etc.)
    pub fn w(mut self, width: impl Into<Length>) -> Self {
        self.style.width = Some(width.into());
        self
    }

    pub fn h(mut self, height: impl Into<Length>) -> Self {
        self.style.height = Some(height.into());
        self
    }
}

impl Element for Stack {
    fn request_layout(&mut self, cx: &mut LayoutContext) -> (LayoutId, StackState) {
        // Request layout for all children
        let mut child_layouts = Vec::new();
        for child in &mut self.children {
            let layout_id = child.request_layout(cx);
            child_layouts.push(layout_id);
        }

        // Stack layout: all children positioned at origin, size = max of children
        let layout_id = cx.request_layout(&self.style);

        (layout_id, StackState { child_layouts })
    }

    fn prepaint(&mut self, state: &mut StackState, cx: &mut PrepaintContext) {
        let bounds = cx.bounds(state.layout_id);

        // Position all children at the same origin (overlapping)
        for (child, &child_layout_id) in self.children.iter_mut().zip(&state.child_layouts) {
            cx.set_child_bounds(child_layout_id, bounds);
            child.prepaint(cx);
        }
    }

    fn paint(&mut self, state: &mut StackState, cx: &mut PaintContext) {
        // Paint in order: first child = bottom, last child = top
        // This naturally creates z-layering without explicit z-index
        for child in &mut self.children {
            child.paint(cx);
        }
    }
}

// Usage example: Modal with backdrop
fn modal_with_backdrop() -> impl Element {
    stack()
        // Backdrop (painted first, bottom layer)
        .child(
            div()
                .w(pct(100))
                .h(pct(100))
                .bg(Color::rgba(0.0, 0.0, 0.0, 0.5))
        )
        // Modal content (painted last, top layer)
        .child(
            div()
                .w(px(400))
                .h(px(300))
                .bg(Color::WHITE)
                .rounded(8.0)
                .p(px(24))
                .child(text("Modal content"))
        )
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Borrowed builder (&mut Self) | Owned builder (Self) | ~2020 (Rust API guidelines) | More ergonomic chaining, no intermediate bindings needed |
| Implicit units (200 = px) | Explicit units (px(200), pct(50)) | ~2022 (type safety trend) | Clearer intent, prevents percent/pixel confusion bugs |
| Custom alignment enums | CSS Flexbox semantics (justify-content, align-items) | ~2021 (web platform convergence) | Familiar to web developers, well-documented behavior |
| Separate Button and ButtonVariant components | Variant enum with state-included styling | ~2023 (design system patterns) | Ensures visual consistency, prevents mismatched states |
| Synchronous image loading | Async loading with placeholders | ~2020 (responsive UI requirement) | Non-blocking, smooth UX even with slow I/O |

**Deprecated/outdated:**
- **Borrowed builders:** Less ergonomic than owned builders in Rust. Use owned pattern (return `Self`)
- **Implicit units:** Ambiguous and error-prone. Always use `px()`, `pct()`, `rem()` constructors
- **Unbounded texture caching:** GPU memory leaks. Use LRU cache with capacity limit
- **Explicit z-index fields on elements:** Adds sorting overhead in immediate-mode UI. Use paint order for z-layering
- **Synchronous image decode on UI thread:** Blocks rendering. Use async loading with placeholders

## Open Questions

Things that couldn't be fully resolved:

1. **Optimal LRU cache capacity for textures**
   - What we know: Depends on image sizes and available GPU memory. Typical range: 50-200 textures
   - What's unclear: How to dynamically adjust based on device capabilities and actual texture sizes
   - Recommendation: Start with fixed capacity (100 textures), add telemetry to track eviction rate and GPU memory usage

2. **Method name length tradeoff (w/h vs width/height)**
   - What we know: User context says "mix approach" - short for very common (.bg(), .w(), .h()), verbose for others (.padding(), .margin())
   - What's unclear: Exact threshold for "very common" - is justify() too long? Is rounded() too obscure for shorthand?
   - Recommendation: Short names for properties used in >50% of elements: .w(), .h(), .bg(), .p(), .m(), .gap(). Verbose for rest: .padding(), .justify(), .items(), .rounded()

3. **Image placeholder color default**
   - What we know: Should be neutral, distinguishable from content. Common: light gray or skeleton pattern
   - What's unclear: Should placeholder match theme (light mode vs dark mode)?
   - Recommendation: Use medium gray rgba(0.7, 0.7, 0.7, 1.0) as default. Allow override via .placeholder_color() for theme matching

4. **Stack z-ordering with nested stacks**
   - What we know: Paint order determines z-layering. Last child paints on top
   - What's unclear: If nested stack has z-ordering, does parent's paint order or child's paint order dominate?
   - Recommendation: Standard painting order - parent paints children in order, recursively. Nested stack's internal ordering is independent, but entire nested stack paints in parent's order

5. **Spacing token enforcement vs raw pixel flexibility**
   - What we know: User wants both tokens (Spacing::Md) and raw pixels (12.0) during development
   - What's unclear: Should raw pixels be allowed in production builds? How to enforce token usage?
   - Recommendation: Accept both during development. Add optional linting/warning system for production builds. Type accepts both, lint warns on Gap::Raw usage

## Sources

### Primary (HIGH confidence)
- CSS Flexbox Aligning Items specification: https://developer.mozilla.org/en-US/docs/Web/CSS/Guides/Flexible_box_layout/Aligning_items
- CSS object-fit specification: https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/object-fit
- Rust builder pattern documentation: https://rust-unofficial.github.io/patterns/patterns/creational/builder.html
- image crate documentation: https://docs.rs/image/
- wgpu texture management: https://docs.rs/wgpu/latest/wgpu/ (Texture section)

### Secondary (MEDIUM confidence)
- Fluent builder API patterns in Rust: https://www.ruststepbystep.com/how-to-implement-a-fluent-interface-and-builder-in-rust/
- ratatui-image async loading pattern: https://crates.io/crates/ratatui-image
- shadcn-ui button variants: https://shadcnstudio.com/docs/components/button
- Tailwind CSS padding/spacing utilities: https://tailwindcss.com/docs/padding
- Design token spacing scales: https://designsystem.digital.gov/design-tokens/spacing-units/
- CSS z-index and stacking contexts: https://web.dev/learn/css/z-index
- Button component states and variants: https://www.nngroup.com/articles/button-states-communicate-interaction/

### Tertiary (LOW confidence - WebSearch only, flag for validation)
- Builder pattern ownership tradeoffs: https://blog.logrocket.com/build-rust-api-builder-pattern/
- wgpu memory leak issues: https://github.com/gfx-rs/wgpu/issues/5397 (context for texture disposal importance)
- GPUI component library examples: https://longbridge.github.io/gpui-component/

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - image crate and wgpu are well-documented and battle-tested
- Architecture patterns: MEDIUM - Builder patterns verified via Rust docs, layout semantics via CSS specs, but GPUI-specific patterns adapted from web search (not direct GPUI documentation)
- Pitfalls: MEDIUM - Texture disposal verified via wgpu issue tracker, builder pattern pitfalls from Rust community, object-fit math from CSS spec
- Implementation details: LOW - LRU cache capacity, method naming threshold, placeholder colors are judgment calls requiring experimentation

**Research date:** 2026-01-29
**Valid until:** ~30 days (builder API patterns are stable, but ora implementation may reveal ergonomic issues)
