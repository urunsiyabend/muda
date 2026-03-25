# Phase 6: Design System - Research

**Researched:** 2026-01-30
**Domain:** Design token systems, theming architecture, color/spacing/typography tokens
**Confidence:** HIGH

## Summary

Design token systems in 2026 follow a three-tier architecture: primitive/palette tokens (raw values), semantic tokens (purpose-based references to primitives), and component tokens (contextual overrides). The user has already decided on a hybrid approach combining semantic tokens with direct palette access, which aligns with industry best practices.

Modern Rust UI frameworks like GPUI Component demonstrate successful token implementation using enums for semantic access (`cx.theme().primary`) with a scale-based palette system underneath. The key architectural pattern is storing theme state in AppContext and propagating it through element context types (LayoutContext, PaintContext).

For comfortable long-session coding UIs, 2026 research emphasizes soft neutrals over pure black/white, desaturated colors in dark mode, and moderate contrast ratios. Typography scales commonly use 1.2 (minor third) ratio with line heights 1.4-1.5 for body text. Spacing scales favor 4px-based systems (Tailwind's approach) to avoid pixel-splitting issues.

**Primary recommendation:** Implement theme as a global resource in AppContext, create enum-based semantic token APIs with underlying palette functions, and provide context accessor methods (`cx.theme()`) that all element contexts delegate to.

## Standard Stack

Design token systems are typically hand-rolled in Rust UI frameworks due to tight integration with the rendering pipeline. No external dependencies are needed.

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| (built-in) | - | Theme token types | Tightly coupled to Style system |
| (built-in) | - | Theme context storage | Lives in AppContext |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| serde | latest | Theme serialization | If implementing theme file loading |
| serde_json | latest | Theme file format | If storing themes in JSON |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Enum-based tokens | String-based tokens | Strings lose type safety but gain dynamic loading |
| Global theme in AppContext | Per-element theme props | Props enable different themes per subtree but add complexity |
| Built-in palettes | External palette crate | External palette adds dependency for marginal benefit |

**Installation:**
```bash
# No external dependencies required for core design system
# Optional: theme file support
cargo add serde --features derive
cargo add serde_json
```

## Architecture Patterns

### Recommended Project Structure
```
ora/src/
├── theme/
│   ├── mod.rs           # Public API: Theme, ColorToken, etc.
│   ├── color.rs         # Color token enums and palette
│   ├── spacing.rs       # Spacing scale and sp() function
│   ├── typography.rs    # TextSize enum and font families
│   └── themes/          # Built-in theme definitions
│       ├── dark.rs
│       └── light.rs
├── style/
│   └── mod.rs           # Style struct (existing, unchanged)
└── context.rs           # AppContext gains theme storage
```

### Pattern 1: Three-Tier Token Architecture

**What:** Separate primitive values, semantic meanings, and component-specific overrides into distinct layers.

**When to use:** All design token systems. Industry standard since ~2020.

**Example:**
```rust
// Tier 1: Primitive palette (raw values)
pub fn gray_50() -> Color { Color::rgb(0.98, 0.98, 0.98) }
pub fn gray_900() -> Color { Color::rgb(0.11, 0.11, 0.11) }
pub fn blue_500() -> Color { Color::rgb(0.37, 0.62, 0.94) }

// Tier 2: Semantic tokens (purpose-based)
#[derive(Copy, Clone, Debug)]
pub enum ColorToken {
    BgPrimary,      // References gray_900 in dark, gray_50 in light
    FgPrimary,      // References gray_50 in dark, gray_900 in light
    Accent,         // References blue_500 in both
}

impl Theme {
    pub fn color(&self, token: ColorToken) -> Color {
        match (self.mode, token) {
            (ThemeMode::Dark, ColorToken::BgPrimary) => gray_900(),
            (ThemeMode::Light, ColorToken::BgPrimary) => gray_50(),
            // ... semantic mappings
        }
    }
}

// Tier 3: Component tokens (optional, for button variants, etc.)
// Implemented via variant-specific style functions (already exists in button.rs)
```

**Source:** [Design tokens explained | Contentful](https://www.contentful.com/blog/design-token-system/)

### Pattern 2: Context-Propagated Theme Access

**What:** Store theme in AppContext, expose via `cx.theme()` accessor in all context types.

**When to use:** When theme needs to be globally consistent and switchable at runtime.

**Example:**
```rust
// In AppContext
pub struct AppContext {
    theme: Theme,
    // ... existing fields
}

impl AppContext {
    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
        // Emit global ThemeChanged event to trigger re-render
        self.emit_global(ThemeChanged);
    }
}

// In PaintContext (and other context types)
pub struct PaintContext<'a> {
    app_context: &'a AppContext,  // existing field
    // ...
}

impl<'a> PaintContext<'a> {
    pub fn theme(&self) -> &Theme {
        self.app_context.theme()
    }
}

// Usage in elements
fn paint(&mut self, state: &mut State, cx: &mut PaintContext) {
    let bg_color = cx.theme().color(ColorToken::BgPrimary);
    // ...
}
```

**Source:** [GPUI Component Theme](https://docs.rs/gpui-component/latest/gpui_component/theme/index.html) demonstrates `cx.theme()` pattern

### Pattern 3: Spacing Token Functions

**What:** Provide both raw pixel values and token function (`sp(2)`) for spacing scale.

**When to use:** All spacing properties (padding, margin, gap).

**Example:**
```rust
// Spacing scale: 1=4px, 2=8px, 3=12px, etc.
pub fn sp(scale: i32) -> f32 {
    if scale < 0 {
        -((scale.abs() as f32) * 4.0)  // Negative spacing for overlap
    } else {
        (scale as f32) * 4.0
    }
}

// Usage: both styles are valid
div().padding(8)         // Raw pixels
div().padding(sp(2))     // Token (same result: 8px)
div().margin(sp(-2))     // Negative spacing: -8px
```

**Source:** [Space in Design Systems | Nathan Curtis](https://medium.com/eightshapes-llc/space-in-design-systems-188bcbae0d62)

### Pattern 4: Typography Token Bundling

**What:** Bundle font size and line height together in semantic text size tokens.

**When to use:** All text rendering to ensure consistent typographic rhythm.

**Example:**
```rust
#[derive(Copy, Clone, Debug)]
pub enum TextSize {
    // Semantic names (preferred)
    Body,       // 14px / 1.5 = 21px line height
    Small,      // 12px / 1.5 = 18px line height
    Code,       // 14px / 1.4 = 19.6px line height (tighter for monospace)
    Heading,    // 18px / 1.25 = 22.5px line height (tight for headings)

    // Scale names (available as alias)
    Xs,  // = Small
    Sm,  // = Body
    Md,  // = Heading
}

impl TextSize {
    pub fn font_size(&self) -> f32 {
        match self {
            TextSize::Body | TextSize::Sm => 14.0,
            TextSize::Small | TextSize::Xs => 12.0,
            TextSize::Code => 14.0,
            TextSize::Heading | TextSize::Md => 18.0,
        }
    }

    pub fn line_height(&self) -> f32 {
        match self {
            TextSize::Body | TextSize::Sm => 21.0,  // 1.5 ratio
            TextSize::Small | TextSize::Xs => 18.0,  // 1.5 ratio
            TextSize::Code => 19.6,  // 1.4 ratio for monospace
            TextSize::Heading | TextSize::Md => 22.5,  // 1.25 ratio
        }
    }
}
```

**Source:** [Typography tokens - Material Design 3](https://m3.material.io/styles/typography/type-scale-tokens)

### Anti-Patterns to Avoid

- **Hard-coded colors in elements:** Always use `cx.theme().color(Token)`, never `Color::rgb(...)` directly in element paint methods. Hard-coded colors cannot respond to theme changes.

- **Component-tied tokens:** Don't create `ButtonPrimaryBg`, `InputBorder`, etc. Keep semantic tokens general-purpose (`AccentPrimary`, `Border`) and let components select appropriate tokens.

- **Excessive token proliferation:** 12 semantic color tokens should cover 90%+ of UI. Don't create `BgSlightlyElevated`, `BgVeryElevated`, etc. Use palette access for edge cases instead.

- **Odd-numbered spacing scales:** Avoid 5px-based scales (5, 10, 15, 20). They cause pixel-splitting issues when centering elements. Use 4px or 8px bases.

**Source:** [Common Mistakes in Design Tokens Adoption](https://designtokens.substack.com/p/common-mistakes-in-design-tokens)

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Color blending/manipulation | Custom alpha/blend math | Color type methods or palette shades | Color math has accessibility implications (contrast ratios) |
| Font loading/fallback | Custom font system | System font stacks with fallback | System fonts are instant, web fonts add complexity |
| Token validation | Runtime checks | Type system (enums) | Compile-time safety prevents invalid token references |

**Key insight:** Design token systems are simple enough to build correctly in-house for Rust UI frameworks. External libraries add dependency weight for marginal benefit. The complexity is in design decisions (token naming, scale values), not implementation.

## Common Pitfalls

### Pitfall 1: Pure Black/White in Dark Mode

**What goes wrong:** Using pure black (#000000) backgrounds with pure white text creates excessive contrast, leading to eye strain during long coding sessions. Glowing halo effects appear around bright text.

**Why it happens:** Designers assume "dark mode = invert colors" without considering perceptual contrast.

**How to avoid:** Use soft off-blacks (gray_900 ~ #1C1C1E / rgb(0.11, 0.11, 0.12)) and off-whites (gray_50 ~ #F5F5F7 / rgb(0.96, 0.96, 0.97)). In 2026, comfortable dark mode palettes favor charcoal and deep navy backgrounds over pure black.

**Warning signs:** User feedback about eye strain, difficulty reading text for extended periods.

**Source:** [10 Dark Mode UI Best Practices 2026](https://www.designstudiouiux.com/blog/dark-mode-ui-design-best-practices/)

### Pitfall 2: Forgetting to Invalidate on Theme Change

**What goes wrong:** Theme switches but UI doesn't update until manual interaction or window resize.

**Why it happens:** Theme is stored as global state, but elements don't know to re-render when it changes.

**How to avoid:** Emit a global `ThemeChanged` event when `set_theme()` is called. Subscribe to this event in window render loop to trigger full re-paint.

```rust
pub struct ThemeChanged;

impl AppContext {
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
        self.emit_global(ThemeChanged);  // Triggers re-render
    }
}
```

**Warning signs:** Theme change requires window resize to take effect.

**Source:** Based on GPUI Component's ThemeRegistry pattern and reactive re-rendering requirements.

### Pitfall 3: Semantic Token Naming Ambiguity

**What goes wrong:** Token names like `Primary`, `Secondary` are ambiguous. Primary color? Primary background? Primary text?

**Why it happens:** Trying to minimize token count leads to overloaded names.

**How to avoid:** Use prefixed semantic names: `BgPrimary`, `FgPrimary`, `AccentPrimary`. The prefix clarifies the domain (background, foreground, accent). User's decision already avoids this pitfall.

**Warning signs:** Developers asking "which primary?" in code reviews, incorrect token usage in components.

**Source:** [Naming Tokens in Design Systems | Nathan Curtis](https://medium.com/eightshapes-llc/naming-tokens-in-design-systems-9e86c7444676)

### Pitfall 4: Insufficient Contrast for Accessibility

**What goes wrong:** Low-contrast "comfortable" palettes fail WCAG AA standards (4.5:1 for normal text, 3:1 for large text).

**Why it happens:** Optimizing for visual comfort without measuring contrast ratios.

**How to avoid:** Test semantic token pairings (FgPrimary on BgPrimary, etc.) with contrast checkers. Aim for 4.5:1 minimum. Document that palette access (escape hatch) requires manual contrast verification.

**Warning signs:** Accessibility audits fail, text readability issues for users with low vision.

**Source:** [Accessible Color Tokens for Enterprise Design Systems](https://www.aufaitux.com/blog/color-tokens-enterprise-design-systems-best-practices/)

### Pitfall 5: Theme Lifecycle Confusion

**What goes wrong:** Calling `cx.theme()` during layout calculation causes theme to be "baked in" to computed bounds, breaking theme switching.

**Why it happens:** Not understanding when theme should be accessed (paint time, not layout time).

**How to avoid:** Only access `cx.theme()` during **paint phase**. Layout should be theme-agnostic (computed from size/spacing tokens). Color tokens are paint-only.

**Warning signs:** Re-theming requires full layout recalculation, performance issues on theme change.

**Source:** Based on understanding of layout/paint separation in ora's architecture.

## Code Examples

Verified patterns from research and existing ora codebase:

### Defining a Theme Type

```rust
// ora/src/theme/mod.rs
use crate::style::Color;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ThemeMode {
    Dark,
    Light,
}

pub struct Theme {
    mode: ThemeMode,
}

impl Theme {
    pub fn dark() -> Self {
        Self { mode: ThemeMode::Dark }
    }

    pub fn light() -> Self {
        Self { mode: ThemeMode::Light }
    }

    pub fn color(&self, token: ColorToken) -> Color {
        use ColorToken::*;
        match (self.mode, token) {
            // Dark mode mappings
            (ThemeMode::Dark, BgPrimary) => palette::gray_900(),
            (ThemeMode::Dark, BgSecondary) => palette::gray_800(),
            (ThemeMode::Dark, FgPrimary) => palette::gray_50(),
            (ThemeMode::Dark, Accent) => palette::blue_500(),

            // Light mode mappings
            (ThemeMode::Light, BgPrimary) => palette::gray_50(),
            (ThemeMode::Light, BgSecondary) => palette::gray_100(),
            (ThemeMode::Light, FgPrimary) => palette::gray_900(),
            (ThemeMode::Light, Accent) => palette::blue_600(),

            // ... other tokens
            _ => Color::transparent(),
        }
    }

    // Direct palette access (escape hatch)
    pub fn palette(&self, palette: PaletteColor) -> Color {
        palette::get(palette)
    }
}
```

**Source:** Hybrid approach pattern from user's CONTEXT.md decisions, verified against [GPUI Component theme structure](https://docs.rs/gpui-component/latest/gpui_component/theme/index.html)

### Palette Scale Functions

```rust
// ora/src/theme/color.rs
use crate::style::Color;

// Neutral grays (11-step scale, 50-950)
pub fn gray_50() -> Color { Color::rgb(0.98, 0.98, 0.98) }   // #FAFAFA
pub fn gray_100() -> Color { Color::rgb(0.96, 0.96, 0.96) }  // #F5F5F5
pub fn gray_200() -> Color { Color::rgb(0.93, 0.93, 0.93) }  // #EEEEEE
// ... through gray_900
pub fn gray_900() -> Color { Color::rgb(0.13, 0.13, 0.13) }  // #212121
pub fn gray_950() -> Color { Color::rgb(0.06, 0.06, 0.06) }  // #0F0F0F

// Blue accent scale
pub fn blue_500() -> Color { Color::rgb(0.13, 0.59, 0.95) }  // #2196F3 (primary accent)
pub fn blue_600() -> Color { Color::rgb(0.12, 0.53, 0.90) }  // Darker for light mode

// Function-based palette access (alternative to individual functions)
pub fn get(color: PaletteColor) -> Color {
    match color {
        PaletteColor::Gray50 => gray_50(),
        PaletteColor::Gray100 => gray_100(),
        // ... etc
    }
}
```

**Source:** Tailwind CSS-style palette scale, verified as modern standard via [Material Design 3](https://m3.material.io/styles/color/roles) and [Backbase Design System](https://designsystem.backbase.com/design-tokens/semantic-colors)

### Context Integration

```rust
// ora/src/context.rs (additions)
use crate::theme::Theme;

pub struct AppContext {
    theme: Theme,
    // ... existing fields
}

impl AppContext {
    pub fn new(entity_storage: EntityStorage) -> Self {
        Self {
            theme: Theme::dark(),  // Default theme
            // ... existing initialization
        }
    }

    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
        self.emit_global(ThemeChanged);
    }
}

// Propagate to PaintContext
impl<'a> PaintContext<'a> {
    pub fn theme(&self) -> &Theme {
        self.app_context.theme()
    }
}
```

**Source:** Context propagation pattern from [Dioxus context documentation](https://dioxuslabs.com/learn/0.7/essentials/basics/context/) and existing ora context architecture.

### Using Theme in Elements

```rust
// ora/src/elements/button.rs (refactored to use theme)
impl Element for Button {
    fn paint(&mut self, state: &mut State, cx: &mut PaintContext) {
        let bounds = cx.bounds(state.layout_id);

        // Determine button state
        let button_state = /* ... existing logic ... */;

        // Use theme tokens instead of hard-coded colors
        let bg_color = match (self.variant, button_state) {
            (Primary, Enabled) => cx.theme().color(ColorToken::AccentPrimary),
            (Primary, Hover) => cx.theme().color(ColorToken::AccentHover),
            (Secondary, Enabled) => cx.theme().color(ColorToken::BgSecondary),
            // ... etc
        };

        let mut style = Style::default();
        style.background = Background::Solid(bg_color);
        style.border_radius = Corners::all(sp(1)); // 4px using spacing token

        cx.paint_styled_rect(&style, &bounds);
    }
}
```

**Source:** Adapted from existing button.rs implementation with theme token substitution.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| CSS Variables | Design tokens with type safety | ~2020 | Rust UI frameworks use enums instead of string-based CSS vars for compile-time safety |
| Pure black/white | Soft neutrals (gray_900/gray_50) | ~2024-2025 | Reduces eye strain, feels more premium, better for long sessions |
| 16px body text | 14px body text for dense UIs | Varies by domain | Coding UIs favor information density over maximum readability |
| Fixed spacing values | 4px-based token scales | ~2019 (Tailwind) | Prevents pixel-splitting, easier mental math |
| Line height 1.5 universal | Ratio varies by text type | ~2022 | Headings use 1.2-1.25, body uses 1.4-1.5, improves rhythm |

**Deprecated/outdated:**
- **String-based token access:** `theme.get("bg-primary")` replaced by enum `theme.color(ColorToken::BgPrimary)` for type safety
- **Single line height ratio:** Modern systems vary ratio by text purpose (display, heading, body, code)
- **5px or 6px spacing base:** Now recognized as problematic due to odd-number centering issues

**Source:** Aggregate of research findings across multiple design system sources.

## Open Questions

Things that couldn't be fully resolved:

1. **Theme Persistence Mechanism**
   - What we know: localStorage is common for web, OS settings for desktop apps
   - What's unclear: Whether ora should persist theme choice, and if so, where (config file? OS integration?)
   - Recommendation: Start with no persistence (startup theme only). Add persistence in future phase if needed. Use `set_theme()` API for runtime switching, which is sufficient.

2. **Color Palette Exact Values**
   - What we know: Modern palettes favor soft neutrals, desaturated blues for accents, 11-step scales (50-950)
   - What's unclear: Exact hex values that feel "modern and comfortable for long coding sessions"
   - Recommendation: Start with Material Design 3-inspired grays (not too blue-tinted) and standard blue accent. Iterate based on user feedback. Reference: gray_900 ~ #212121, gray_50 ~ #FAFAFA, blue_500 ~ #2196F3.

3. **Custom Theme Override API**
   - What we know: User decided "custom themes via overrides only" (not full custom palettes)
   - What's unclear: API shape for overrides - per-token override, or palette swap?
   - Recommendation: Defer to implementation phase. Simplest approach: `Theme::dark().override_token(ColorToken::Accent, custom_color)` builder pattern.

4. **Theme Event Granularity**
   - What we know: Global `ThemeChanged` event triggers re-render
   - What's unclear: Should event carry old/new theme for diffing, or just signal change?
   - Recommendation: Simple signal-only event is sufficient. Full re-paint is cheap enough, no need for diffing optimization yet.

## Sources

### Primary (HIGH confidence)
- [GPUI Component Theme Documentation](https://docs.rs/gpui-component/latest/gpui_component/theme/index.html) - Rust UI theme implementation patterns
- [GPUI Component Theme Guide](https://longbridge.github.io/gpui-component/docs/theme) - Runtime theme switching and registry patterns
- [Design tokens explained | Contentful](https://www.contentful.com/blog/design-token-system/) - Three-tier token architecture
- [Naming Tokens in Design Systems | Nathan Curtis](https://medium.com/eightshapes-llc/naming-tokens-in-design-systems-9e86c7444676) - Token naming conventions
- [Typography tokens - Material Design 3](https://m3.material.io/styles/typography/type-scale-tokens) - Typography scale and line height ratios
- [Space in Design Systems | Nathan Curtis](https://medium.com/eightshapes-llc/space-in-design-systems-188bcbae0d62) - Spacing scale best practices

### Secondary (MEDIUM confidence)
- [Designing semantic colors for your system](https://imperavi.com/blog/designing-semantic-colors-for-your-system/) - Semantic color token patterns
- [Accessible Color Tokens for Enterprise Design Systems](https://www.aufaitux.com/blog/color-tokens-enterprise-design-systems-best-practices/) - Contrast ratio requirements
- [Color tokens: guide to light and dark modes](https://medium.com/design-bootcamp/color-tokens-guide-to-light-and-dark-modes-in-design-systems-146ab33023ac) - Dark/light theme switching implementation
- [10 Dark Mode UI Best Practices 2026](https://www.designstudiouiux.com/blog/dark-mode-ui-design-best-practices/) - Dark mode color palette recommendations
- [Common Mistakes in Design Tokens Adoption](https://designtokens.substack.com/p/common-mistakes-in-design-tokens) - Anti-patterns and pitfalls
- [Dioxus Context Documentation](https://dioxuslabs.com/learn/0.7/essentials/basics/context/) - Context propagation patterns in Rust UI

### Tertiary (LOW confidence - industry trends)
- [Modern App Colors 2026](https://webosmotic.com/blog/modern-app-colors/) - 2026 color trends (soft neutrals, warm tones)
- [Design Systems Typography Guide](https://www.designsystems.com/typography-guides/) - General typography best practices
- Multiple design system documentation sources (VA.gov, Backbase, GitLab Pajamas) for token organization patterns

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - No external dependencies needed, architecture is clear from GPUI Component example
- Architecture: HIGH - Three-tier token system is industry standard, context propagation verified in existing ora codebase
- Pitfalls: HIGH - Common mistakes well-documented across multiple authoritative sources
- Color palette values: MEDIUM - Trends clear (soft neutrals, desaturated accents), exact hex values are taste/iteration
- Typography ratios: HIGH - Line height ranges (1.2-1.5) verified across Material Design, design system guides
- Spacing scale: HIGH - 4px base is proven standard (Tailwind, multiple design systems)

**Research date:** 2026-01-30
**Valid until:** 2026-03-01 (30 days - design token patterns are stable)
