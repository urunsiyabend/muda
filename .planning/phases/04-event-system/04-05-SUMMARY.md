---
phase: 04
plan: 05
subsystem: events
tags: [interaction, hover, active, focus, mouse-capture, visual-feedback]
dependencies:
  requires: [04-01, 04-02, 04-03, 04-04]
  provides: [interaction-state, hover-tracking, active-tracking, mouse-capture, interactive-styling]
  affects: [05-builder-api]
tech-stack:
  added: []
  patterns: [state-queries, interactive-styling, persistent-handles]
key-files:
  created:
    - ora/src/events/interaction.rs
    - ora/examples/interactive_demo.rs
  modified:
    - ora/src/context.rs
    - ora/src/platform/event_loop.rs
    - ora/src/elements/div.rs
    - ora/src/elements/text.rs
    - ora/src/element/trait_def.rs
decisions:
  - id: interaction-state-in-context
    what: Store InteractionState in AppContext alongside FocusState
    why: Centralized state management, consistent with focus management pattern
    alternatives: [per-window state, global static]
  - id: hover-active-path-includes-ancestors
    what: InteractionState tracks both element and ancestors in hover_path/active_path
    why: Enables parent elements to query if any child is hovered/active
    alternatives: [element-only tracking]
  - id: mouse-capture-for-drag
    what: MouseCapture struct with hitbox_id and button, released on mouse up
    why: Enables drag operations to continue outside element bounds
    alternatives: [implicit capture, per-element capture]
  - id: interactive-styling-in-div
    what: Div has hover_bg(), active_bg(), focus_ring() builder methods
    why: Declarative visual feedback, elements query state during paint
    alternatives: [external styling system, CSS-like rules]
  - id: text-non-interactive
    what: TextElement registers hitboxes with opaque:false
    why: Text is visual-only, shouldn't capture mouse events from parent buttons
    alternatives: [opaque text with event forwarding]
  - id: persistent-focus-handles
    what: FocusHandles created once and stored in view, not created every render
    why: Prevents FocusId explosion (60fps × 3 handles = 180 new IDs/sec)
    alternatives: [handle caching, ID deduplication]
metrics:
  duration: 50 minutes
  commits: 10
  files_changed: 9
  completed: 2026-01-29
---

# Phase 04 Plan 05: Hover/Active Tracking & Interactive Demo Summary

**One-liner:** Framework-managed interaction state with hover/active tracking, mouse capture, and full visual feedback loop

## What Was Built

### Core Features

**1. InteractionState (ora/src/events/interaction.rs)**
- MouseCapture struct: `{ hitbox_id, button }` for drag operations
- Hover tracking: `hovered`, `hover_path` HashSet (element + ancestors)
- Active tracking: `active`, `active_path` HashSet (element + ancestors)
- Methods: `update_hover()`, `set_active()`, `clear_active()`, `clear_all()`
- Queries: `is_hovered()`, `is_active()`, `hovered_hitbox()`, `active_hitbox()`
- Mouse capture: `capture_mouse()`, `release_mouse_capture()`, `is_mouse_captured()`

**2. Integration with AppContext (ora/src/context.rs)**
- Added `interaction_state: InteractionState` field
- Query methods: `is_hovered()`, `is_active()`, `capture_mouse()`, `release_mouse_capture()`
- Forwarded to ViewContext: `is_hovered()`, `is_active()`

**3. Event Loop Integration (ora/src/platform/event_loop.rs)**
- **CursorMoved:** Updates hover state, respects mouse capture, requests redraw
- **MouseInput Pressed:** Sets active state, logs press with HitboxId
- **MouseInput Released:** Clears active state, releases capture
- **Focused(false):** Clears all interaction state on window deactivation
- **Comprehensive logging:** Hover enter/exit/change, active press/release, Tab navigation

**4. Interactive Styling System (ora/src/elements/div.rs)**
- Builder methods: `hover_bg()`, `active_bg()`, `focus_ring()`
- Stores HitboxId and FocusId from prepaint phase
- Queries interaction state during paint phase
- Applies conditional styling: active > hover > focus > base
- Focus ring: 2px border in focus color when keyboard-focused

**5. PaintContext Enhancement (ora/src/element/trait_def.rs)**
- Added `interaction_state: &InteractionState` field
- Added `focus_state: &FocusState` field
- Query methods: `is_hovered()`, `is_active()`, `is_focused()`
- Enables elements to query state during paint for visual feedback

**6. Hitbox Registration (ora/src/elements/div.rs, text.rs)**
- **Div:** Registers hitbox with `opaque:true` (interactive, captures mouse)
- **TextElement:** Registers hitbox with `opaque:false` (visual-only, transparent to mouse)
- Fixes hover flickering when cursor moves over text inside buttons

**7. Focus Handle Management (ora/examples/interactive_demo.rs)**
- FocusHandles created once during view construction
- Stored persistently in view struct fields
- Cloned during render (not created fresh)
- Prevents FocusId explosion (stable IDs across 60fps continuous redraw)

**8. Interactive Demo**
- Counter with increment/decrement/reset actions (Arrow Up/Down, Ctrl+R)
- 3 focusable buttons with distinct colors (blue, green, orange)
- Visual feedback: hover (lighter), active (darker), focus (colored border)
- Tab/Shift+Tab navigation between buttons
- Demonstrates all event system features working together

## Deviations from Plan

### Auto-fixed Issues (Deviation Rules 1-3)

**1. [Rule 1 - Bug] Add logging for hover, active, and Tab navigation (965f0d6)**
- **Found during:** Initial verification - no logs visible
- **Issue:** User couldn't see interaction state changes happening
- **Fix:** Added logging in event_loop.rs for hover enter/exit/change, active press/release, Tab navigation with FocusId changes
- **Files modified:** ora/src/platform/event_loop.rs
- **Commit:** 965f0d6

**2. [Rule 2 - Missing Critical] Make action handlers actually modify counter state (e397e3b)**
- **Found during:** Initial verification - counter didn't update
- **Issue:** Action handlers had `'static` lifetime, couldn't access AppContext to mutate model
- **Fix:** Used Rc<RefCell<i32>> shared counter between view and handlers, workaround for action handler context limitation
- **Files modified:** ora/examples/interactive_demo.rs
- **Commit:** e397e3b

**3. [Rule 1 - Bug] Register hitboxes for Div and TextElement (6adbeed)**
- **Found during:** Second verification - no mouse interaction logs
- **Issue:** Elements didn't register hitboxes during prepaint, hit testing always returned None
- **Fix:** Added `cx.register_hitbox()` calls in Div.prepaint() and TextElement.prepaint(), stored HitboxId in state
- **Files modified:** ora/src/elements/div.rs, ora/src/elements/text.rs
- **Commit:** 6adbeed

**4. [Rule 2 - Missing Critical] Add focusable support to Div (feb9241)**
- **Found during:** Second verification - focus_order empty, no focusable elements
- **Issue:** Elements didn't register as focusable during prepaint
- **Fix:** Added `focus_handle: Option<FocusHandle>` field to Div, `focusable()` builder method, `cx.register_focusable()` call in prepaint
- **Files modified:** ora/src/elements/div.rs, ora/examples/interactive_demo.rs (added 3 focusable buttons)
- **Commit:** feb9241

**5. [Rule 2 - Missing Critical] Add interactive styling with visual feedback (78f2577)**
- **Found during:** Third verification - no visual changes, only logs
- **Issue:** Elements didn't query interaction state during paint to change colors
- **Fix:** Added `hover_bg()`, `active_bg()`, `focus_ring()` builders to Div; enhanced PaintContext with InteractionState and FocusState; Div.paint() queries state and applies conditional styling
- **Files modified:** ora/src/elements/div.rs, ora/src/element/trait_def.rs, ora/src/platform/event_loop.rs, ora/examples/interactive_demo.rs
- **Commit:** 78f2577

**6. [Rule 1 - Bug] Make TextElement non-interactive to fix hover flickering (8c45515)**
- **Found during:** Fourth verification - hover effect disappeared when cursor over button text
- **Issue:** Text hitboxes were opaque, capturing mouse events instead of parent button
- **Fix:** Changed TextElement.prepaint() from `register_hitbox(bounds, true)` to `register_hitbox(bounds, false)`
- **Files modified:** ora/src/elements/text.rs
- **Commit:** 8c45515

**7. [Rule 1 - Bug] Store FocusHandles persistently to fix focus cycling (c0cea10)**
- **Found during:** Fourth verification - FocusIds increasing by ~350 each Tab press
- **Issue:** FocusHandles created fresh every render() call (60fps = 180 new handles/sec)
- **Fix:** Stored 3 FocusHandles in InteractiveView struct, created once in main(), cloned (not created) during render
- **Files modified:** ora/examples/interactive_demo.rs
- **Commit:** c0cea10

## Technical Implementation Details

### Interaction State Architecture

**State tracking:**
```rust
pub struct InteractionState {
    hovered: Option<HitboxId>,
    hover_path: HashSet<HitboxId>,  // element + ancestors
    active: Option<HitboxId>,
    active_path: HashSet<HitboxId>,  // element + ancestors
    mouse_capture: Option<MouseCapture>,
}
```

**Event loop integration:**
- CursorMoved: Checks mouse capture first, then hit test, updates hover, logs changes
- MouseInput Pressed: Sets active state for hit element
- MouseInput Released: Clears active, releases capture
- Focused(false): Clears all state (window deactivation)

**Visual feedback flow:**
1. Prepaint: Element registers hitbox/focus, stores IDs in state
2. Paint: Element queries `cx.is_hovered(hitbox_id)`, `cx.is_active(hitbox_id)`, `cx.is_focused(focus_id)`
3. Paint: Element applies conditional styling based on state
4. Render: GPU draws with correct colors
5. Continuous: Redraw loop (60fps) picks up state changes instantly

### Critical Architectural Patterns Established

**1. Opaque vs Non-Opaque Hitboxes**
- Interactive elements (Div with focusable, buttons): `opaque:true`
- Visual-only elements (TextElement): `opaque:false`
- Hit testing finds topmost opaque hitbox, skips non-opaque
- Enables complex layouts where text doesn't interfere with button interaction

**2. Persistent Handle Storage**
- FocusHandles created once, stored in view struct
- Prevents ID explosion from continuous redraw (60fps)
- Pattern applies to any handle type that needs stability across frames
- Critical for focus management correctness

**3. Query-Based Visual Feedback**
- Elements store IDs from prepaint (when registered)
- Elements query state during paint (when rendering)
- Separation of concerns: registration vs consumption
- Enables declarative interactive styling

**4. State Priority System**
- Active > Hover > Focus > Base
- Only one background color applied (highest priority wins)
- Focus ring can coexist with hover/active background
- Clear, predictable visual hierarchy

## Testing & Verification

**Automated tests:** All existing tests pass (15 unit tests)

**Manual verification (user-approved):**
✓ Counter updates with Arrow Up/Down/Ctrl+R
✓ Buttons change color on hover (lighter shade)
✓ Buttons change color on click/hold (darker shade)
✓ Focused button shows colored border (focus ring)
✓ No hover flickering when cursor over button text
✓ Focus cycles correctly between 3 stable buttons
✓ FocusIds stay stable (logs show same IDs, not increasing)
✓ Hover + focus work together (lit background + border)
✓ Active + focus work together (pressed background + border)
✓ All logging works (hover, focus, active, actions)

## Next Phase Readiness

### What Phase 5 (Builder API) Can Build On

**1. Interactive styling foundation:**
- Div already has `hover_bg()`, `active_bg()`, `focus_ring()` patterns
- Phase 5 can extend to: `hover_border()`, `active_scale()`, `focus_shadow()`
- Pattern proven: store config in element, query state in paint

**2. Persistent handle pattern:**
- FocusHandle storage pattern established
- Phase 5 builder API can apply same pattern to other handles
- Critical for any state that needs stability across frames

**3. Opaque hitbox pattern:**
- Text vs Button distinction proven crucial
- Phase 5 can add `.interactive()` builder method to Div
- Default: Div is non-opaque (layout only), `.interactive()` makes it opaque

**4. PaintContext enhancement pattern:**
- Successfully added InteractionState and FocusState to PaintContext
- Phase 5 can add more state as needed (scroll position, animation state)
- Query methods proven scalable

### Blockers for Future Work

None - all core event system features complete and working.

### Concerns for Future Phases

**1. Performance with many hitboxes:**
- Current hit testing is O(n) linear scan in reverse
- 100+ elements may need spatial indexing (quadtree, R-tree)
- Monitor performance, optimize if needed

**2. Mouse capture edge cases:**
- Currently released on any mouse up
- Multi-button drag scenarios not tested
- Touch/pen input not considered

**3. Action handler context access:**
- Current workaround: Rc<RefCell<>> for shared state
- Proper solution: Action handlers receive &mut AppContext
- Requires redesigning ActionRegistry::dispatch signature

## Lessons Learned

**1. Visual feedback is essential for verification**
- Logs alone aren't enough - users need to see things change
- Interactive styling wasn't in original plan but was critical
- Always prototype with visible effects, not just console output

**2. Opaque hitbox distinction is critical**
- Text capturing mouse events broke button interaction completely
- Non-interactive elements must be transparent to hit testing
- This distinction should be explicit in the API (not implicit)

**3. Persistent handles are non-negotiable**
- Creating handles every frame (60fps) causes ID explosion
- Handles must be created once and stored persistently
- This is an architectural requirement, not an optimization

**4. Deviation rules work well for iterative development**
- 7 fixes applied automatically during verification
- Each fix committed atomically with clear reasoning
- User only needed to verify final result, not approve each fix

**5. Continuous redraw simplifies state updates**
- Counter changes picked up instantly (no manual redraw trigger)
- Interactive styling works because view re-renders every frame
- Trade-off: CPU usage vs developer experience (optimize later)

## Files Changed

**Created:**
- `ora/src/events/interaction.rs` (159 lines) - InteractionState with hover/active/capture tracking
- `ora/examples/interactive_demo.rs` (295 lines) - Complete interactive demo with buttons

**Modified:**
- `ora/src/events/mod.rs` - Export InteractionState and MouseCapture
- `ora/src/context.rs` - Add interaction_state field, query methods
- `ora/src/platform/event_loop.rs` - Update hover/active on mouse events, logging
- `ora/src/elements/div.rs` - Add interactive styling, hitbox registration, focusable support
- `ora/src/elements/text.rs` - Register non-opaque hitboxes
- `ora/src/element/trait_def.rs` - Add InteractionState and FocusState to PaintContext

## Commit History

1. **23f8d65** - feat(04-05): create interaction state tracking
2. **bf0c9d0** - feat(04-05): integrate interaction state and add queries
3. **50c1364** - feat(04-05): create interactive demo
4. **965f0d6** - fix(04-05): add logging for hover, active, and Tab navigation
5. **e397e3b** - fix(04-05): make action handlers actually modify counter state
6. **6adbeed** - fix(04-05): register hitboxes for Div and TextElement
7. **feb9241** - fix(04-05): add focusable support to Div and demo buttons
8. **78f2577** - feat(04-05): add interactive styling to Div with visual feedback
9. **8c45515** - fix(04-05): make TextElement non-interactive to fix hover flickering
10. **c0cea10** - fix(04-05): store FocusHandles persistently to fix focus cycling

---

**Phase 04 Plan 05 Status:** ✅ **COMPLETE**

All event system features implemented and verified. Interactive demo demonstrates full event loop with visual feedback. Critical architectural issues fixed. Ready for Phase 5 Builder API.
