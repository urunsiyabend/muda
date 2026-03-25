---
phase: 08
plan: 01
name: widget-foundations
subsystem: elements
tags: [rust, widgets, input, button, widget-size, element-lifecycle]

dependency-graph:
  requires:
    - "07-editor-chrome-text-editing (TextElement, Element trait, PaintContext theme access)"
    - "06-design-system (ColorToken, Theme, WidgetSize config values)"
    - "05-element-library (composite element pattern, Button as reference)"
  provides:
    - "WidgetSize enum — sm/md/lg size tiers for all Phase 8 widgets"
    - "Button with size() builder method using WidgetSize"
    - "Input element — text input with placeholder, focus ring, themed colors"
  affects:
    - "08-02 through 08-08 (all subsequent widgets use WidgetSize for size tiers)"
    - "Future form/settings views consuming Input element"

tech-stack:
  added: []
  patterns:
    - "Composite element pattern: store TextElement internally for lifecycle control"
    - "Borrow-safe paint: collect theme colors into owned values before mutable cx ops"
    - "WidgetSize config() provides height/font/padding/border-radius for all size tiers"
    - "Builder API: constructor fn + method chaining for ergonomic element construction"

key-files:
  created:
    - ora/src/elements/input.rs
  modified:
    - ora/src/elements/mod.rs
    - ora/src/lib.rs
    - ora/src/elements/button.rs  # (Task 1, committed as 3deef53)

decisions:
  - "WidgetSize enum in button.rs (not a separate file) - co-located with first consumer, imported by all others"
  - "Input uses WidgetSize.height() as min_height (not fixed height) - allows content to expand"
  - "Text color: FgMuted for placeholder, FgPrimary for value - standard placeholder convention"
  - "Border: Accent when focused, FgMuted when hovered, Border when normal, BgSecondary when disabled"
  - "Disabled alpha: 0.5 on bg, 0.5 on text - matches Button disabled pattern"

metrics:
  duration: "98s (~1m 38s)"
  completed: "2026-03-02"
  tasks-completed: 2
  tests-added: 2
  tests-total: 56
---

# Phase 8 Plan 01: Widget Foundations Summary

**One-liner:** WidgetSize enum with sm/md/lg tiers plus Input composite element using themed placeholder/focus/disabled states.

## What Was Built

### Task 1 (pre-completed, commit 3deef53)
`WidgetSize` enum added to `ora/src/elements/button.rs` providing:
- Three size tiers: `Sm` (24px/12px/8px), `Md` (32px/13px/12px), `Lg` (40px/14px/16px)
- Methods: `height()`, `font_size()`, `padding_h()`, `padding_v()`, `border_radius()`
- `Button::size(WidgetSize)` builder method wired to all size configuration

### Task 2 (commit 570aa2e)
`ora/src/elements/input.rs` implementing `Input` widget (WIDGET-02):

**Struct fields:**
- `value: String` — current input value
- `placeholder: String` — placeholder text shown when value is empty
- `size: WidgetSize` — size tier (defaults to Md)
- `disabled: bool` — disables interaction and reduces alpha
- `focus_handle: Option<FocusHandle>` — optional keyboard focus registration
- `on_change: Option<Box<dyn Fn(&str) + 'static>>` — change callback
- `text_element: Option<TextElement>` — internal text child for lifecycle control

**Builder API:**
- `input(placeholder)` — constructor function
- `.value(v)` — set current value
- `.size(s)` — set size tier
- `.disabled(d)` — enable/disable
- `.focusable(handle)` — make keyboard focusable
- `.on_change(handler)` — attach change callback

**Three-phase Element lifecycle:**
- `request_layout`: Container style with WidgetSize padding/border-radius/min_height, 1px border. Shows value if non-empty else placeholder. Creates TextElement child, registers parent-child layout.
- `prepaint`: Registers opaque hitbox (captures mouse), registers focusable if handle provided, prepaints text child.
- `paint`: Resolves interaction state (is_focused, is_hovered). Collects theme colors into owned values (borrow-safe), paints styled rect with border, updates text color, paints text child. Disabled state applies 0.5 alpha.

**Theme color mapping:**
| State | BG | Border | Text |
|---|---|---|---|
| Normal | BgPrimary | Border | FgPrimary (value) / FgMuted (placeholder) |
| Hovered | BgPrimary | FgMuted | same |
| Focused | BgPrimary | Accent | same |
| Disabled | BgSecondary @50% | BgSecondary | FgPrimary/FgMuted @50% |

## Tests Added

- `test_input_creation` — verifies default values (empty value, Md size, not disabled, no handles)
- `test_input_builder` — verifies builder chain: placeholder, value, size(Lg), disabled(true)

Total test count: 56 (up from 54 before this plan).

## Deviations from Plan

None - plan executed exactly as written. Task 1 was pre-completed (3deef53) and verified present before Task 2 execution.

## Next Phase Readiness

- All subsequent Phase 8 widgets (Checkbox, Select, TreeItem, ListItem, Tab, Breadcrumb, CommandPalette) can import `WidgetSize` from `elements::button`
- `Input` element is ready for use in form views and settings panels
- Pattern established: composite element with internal TextElement, builder API, WidgetSize size tiers
