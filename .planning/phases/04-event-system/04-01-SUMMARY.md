---
phase: 04-event-system
plan: 01
subsystem: events
tags: [mouse-events, hit-testing, event-routing, winit]
requires: [03-reactive-state-system]
provides:
  - Mouse event type definitions (MouseMoveEvent, MouseDownEvent, MouseUpEvent, MouseScrollEvent)
  - Hitbox registration infrastructure in PrepaintContext
  - Z-order hit testing for mouse event routing
  - winit event translation to ora event types
affects: [04-02-focus-system, 04-03-input-handlers, 04-04-event-propagation]
tech-stack:
  added: []
  patterns: [event-routing, hit-testing, reverse-z-order]
key-files:
  created:
    - ora/src/events/mod.rs
    - ora/src/events/types.rs
    - ora/src/events/mouse.rs
  modified:
    - ora/src/element/trait_def.rs
    - ora/src/platform/event_loop.rs
    - ora/src/style/units.rs
    - ora/src/lib.rs
decisions:
  - Mouse event types follow winit structure (position, button, modifiers)
  - Hit testing uses reverse iteration (last painted = topmost)
  - Opaque flag on hitboxes allows pass-through behavior
  - HitboxId is u64 opaque handle for element identification
  - Rect.contains_point() uses half-open interval [origin, origin+size)
metrics:
  duration: 5m 35s
  tasks: 3
  commits: 3
  files_created: 4
  files_modified: 4
  lines_added: ~180
completed: 2026-01-29
---

# Phase 4 Plan 1: Event Types & Hit Testing Summary

**One-liner:** Created mouse event types and Z-order hit testing infrastructure for routing winit events to elements.

## Objective

Create the foundational event types and hit testing infrastructure needed for mouse event routing. Enable elements to register hitboxes during prepaint and perform reverse Z-order hit testing when mouse events arrive from winit.

## What Was Built

### Event Types Module (ora/src/events)

Created complete event type system with:
- **Common types** (types.rs): Point, Modifiers, MouseButton enum
- **Mouse events** (mouse.rs): MouseMoveEvent, MouseDownEvent, MouseUpEvent, MouseScrollEvent
- **Hit testing** (mouse.rs): Hitbox, HitboxId, hit_test() function

### Hitbox Registration Infrastructure

Extended PrepaintContext to support hitbox registration:
- Added `hitboxes: Vec<Hitbox>` to collect hitboxes in paint order
- Added `next_hitbox_id: u64` counter for generating unique HitboxId values
- Implemented `register_hitbox(bounds, opaque)` method
- Implemented `take_hitboxes()` for event loop to extract collected hitboxes

### Hit Testing & Event Wiring

Integrated hit testing into the event loop:
- Added `hitboxes: Vec<Hitbox>` and `cursor_position: Point` to OraApp
- Store hitboxes from prepaint phase for use in event routing
- Handle `WindowEvent::CursorMoved` with hit testing
- Handle `WindowEvent::MouseInput` (down/up) with button detection
- Map winit mouse buttons to ora MouseButton enum
- Log hit test results at trace level for debugging

### Supporting Infrastructure

Added `Rect.contains_point()` method to style::units::Rect for point-in-rectangle testing using half-open interval semantics (x >= origin.x && x < origin.x + width).

## How It Works

1. **Prepaint Phase:** Elements call `prepaint_cx.register_hitbox(bounds, opaque)` to register interactive regions
2. **Hitbox Collection:** PrepaintContext collects hitboxes in paint order (same order as visual rendering)
3. **Event Arrival:** winit fires CursorMoved or MouseInput events
4. **Hit Testing:** `hit_test(&hitboxes, point)` iterates in reverse order (last painted = topmost)
5. **Target Selection:** First opaque hitbox containing the point becomes the target
6. **Logging:** Hit test results logged at trace level (RUST_LOG=trace)

## Commits

| Hash    | Message                                                |
| ------- | ------------------------------------------------------ |
| 64ef121 | feat(04-01): create event types module                |
| c3a4108 | feat(04-01): add hitbox registration to PrepaintContext|
| 405d4a3 | feat(04-01): implement hit testing and wire mouse events|

## Testing

- `cargo check -p ora`: Compiles without errors
- `cargo test -p ora`: All 11 existing tests pass
- `cargo build --example reactive_demo`: Compiles successfully
- Event loop correctly handles mouse events (verified via trace logs)

## Decisions Made

1. **Mouse event structure follows winit:** Position, button, modifiers match winit's event model for natural translation
2. **Reverse Z-order hit testing:** Iterate hitboxes in reverse to find topmost (last painted) element first
3. **Opaque flag for pass-through:** Non-opaque hitboxes are skipped, allowing future transparent/pass-through behavior
4. **HitboxId as u64 opaque handle:** Simple, copyable identifier for elements to store and match
5. **Half-open interval for contains_point:** x >= origin.x && x < origin.x + width (standard rect semantics)

## Next Phase Readiness

**Blockers:** None

**Dependencies satisfied:**
- ✅ Event types defined and exported
- ✅ Hitbox registration available to elements
- ✅ Hit testing functional and integrated
- ✅ winit events translated to ora events

**Ready for:**
- Phase 04-02: Focus system (keyboard focus tracking)
- Phase 04-03: Input handlers (onclick, onmousemove element callbacks)
- Phase 04-04: Event propagation (bubbling, capture)

**Notes:**
- Current implementation logs hit test results but doesn't dispatch events to elements yet
- Elements don't yet register hitboxes (Div/TextElement need updates)
- Event handler callbacks will be added in 04-03

## Deviations from Plan

None - plan executed exactly as written.

## Performance Notes

- Execution time: 5m 35s
- Hit testing is O(n) where n = number of hitboxes, but reverse iteration finds topmost element quickly
- Hitbox collection happens once per frame during prepaint
- No allocations during hit testing (borrows Vec<Hitbox>)

## File Changes

**Created:**
- `ora/src/events/mod.rs` (11 lines) - Module exports
- `ora/src/events/types.rs` (43 lines) - Point, Modifiers, MouseButton
- `ora/src/events/mouse.rs` (59 lines) - Mouse events, Hitbox, hit_test()

**Modified:**
- `ora/src/element/trait_def.rs` (+22 lines) - Hitbox registration in PrepaintContext
- `ora/src/platform/event_loop.rs` (+37 lines) - Mouse event handling
- `ora/src/style/units.rs` (+8 lines) - Rect.contains_point()
- `ora/src/lib.rs` (+4 lines) - Event type exports

**Total:** 4 files created, 4 files modified, ~180 lines added

---

*Summary completed: 2026-01-29*
*Execution time: 5m 35s*
