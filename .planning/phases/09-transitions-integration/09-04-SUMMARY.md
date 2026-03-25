---
phase: 09-transitions-integration
plan: 04
subsystem: animation
tags: [rust, transitions, tween, animation, appcontext, refcell]

# Dependency graph
requires:
  - phase: 09-01
    provides: Tween<T>, TweenState, Tweenable, Easing — the interpolation primitives TransitionState wraps

provides:
  - TransitionId(u64): stable cross-frame identity, explicitly assigned by callers
  - TransitionSpec: per-property transition configs (bg, opacity, color, position)
  - TransitionConfig: duration_ms + Easing for a single property
  - TransitionState: per-element tween storage with advance_bg/opacity/color methods
  - TransitionRegistry: HashMap<TransitionId, TransitionState> with get_or_create, has_active_transitions, cleanup_completed
  - AppContext.transition_registry: RefCell<TransitionRegistry> with has_active_transitions() method

affects:
  - 09-06 (Div paint-time interpolation — reads TransitionSpec, calls advance_bg via registry)
  - future renderer integration (polling has_active_transitions() to drive continuous redraws)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - RefCell interior mutability for PaintContext access to mutable AppContext fields
    - Instant-tween initialization pattern (start+update on first call) for zero-jump first-frame behavior
    - Config-per-retarget pattern (new Tween per target change, not retarget on existing) ensures duration stays correct

key-files:
  created:
    - ora/src/animation/transition.rs
  modified:
    - ora/src/animation/mod.rs
    - ora/src/context.rs

key-decisions:
  - "TransitionId explicitly assigned by callers — no auto-generation from hitbox IDs (prevents stale-id bugs)"
  - "TransitionRegistry in AppContext uses RefCell<T> — PaintContext holds *const AppContext but needs mutable registry access during paint"
  - "advance_* methods create new Tween per target change (not retarget) — preserves duration from current config, avoids zero-duration silent failure"
  - "advance_* calls start() then update() immediately — 0ms tweens complete on first frame, no false active-transition signals"
  - "cleanup_completed() retains only active (Playing) tweens — prevents unbounded HashMap growth"

patterns-established:
  - "Instant-tween seed: create Tween::new(from=target, to=target, duration) then start()+update() so first call is idempotent"
  - "Per-retarget tween creation: capture current value, new Tween with fresh duration/easing from config, avoids duration bleed-through"

# Metrics
duration: 6min
completed: 2026-03-25
---

# Phase 09 Plan 04: Transition Infrastructure Summary

**TransitionRegistry (per-element tween storage keyed by stable TransitionId) and AppContext integration via RefCell for paint-time mutable access**

## Performance

- **Duration:** ~6 min
- **Started:** 2026-03-25T11:30:48Z
- **Completed:** 2026-03-25T11:36:21Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments

- `TransitionId`, `TransitionConfig`, `TransitionSpec`, `TransitionState`, `TransitionRegistry` types in `ora/src/animation/transition.rs`
- `advance_bg`, `advance_opacity`, `advance_color` methods on `TransitionState` with correct tween lifecycle (new tween per target change, immediate update to handle 0ms configs)
- `TransitionRegistry` added to `AppContext` via `RefCell<TransitionRegistry>` for interior mutability during paint
- `AppContext::has_active_transitions()` convenience method for renderer polling
- 7 unit tests covering idle state, first-call behavior, retarget activation, registry persistence, and cleanup

## Task Commits

1. **Task 1: TransitionSpec, TransitionState, TransitionRegistry types** - `9d9adc2` (feat)
2. **Task 2: Add TransitionRegistry to AppContext** - `9ad96f0` (feat)

## Files Created/Modified

- `ora/src/animation/transition.rs` — All transition types + 7 unit tests (created, 348 lines)
- `ora/src/animation/mod.rs` — Added `pub mod transition` + re-exports for all 5 public types
- `ora/src/context.rs` — Added `transition_registry: RefCell<TransitionRegistry>` field, init in `new()`, `has_active_transitions()` method

## Decisions Made

- `TransitionId` explicitly assigned by callers (no auto-generation) — avoids stale-id bugs when elements are recreated each frame with different hitbox IDs
- `RefCell<TransitionRegistry>` in `AppContext` — PaintContext holds `*const AppContext` (immutable pointer) but paint() must advance tweens; RefCell provides safe interior mutability
- New `Tween::new()` per target change, not `retarget()` — `retarget()` preserves the existing tween's duration, which would be zero for freshly-initialized tweens; creating a new tween ensures duration always comes from the current `TransitionConfig`
- `start()` followed immediately by `update()` in `advance_*` — ensures 0ms tweens complete in the same frame they are created, preventing spurious `has_active_transitions()` returns

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed zero-duration tween not completing on first frame**

- **Found during:** Task 1 (advance_* method implementation and testing)
- **Issue:** Plan specified using `tween.retarget()` for target changes; `retarget()` preserves duration from original tween — instant tweens have `Duration::ZERO` so the retargetted tween also has 0 duration, but the Playing state check detected it as active
- **Fix:** Replaced retarget-on-existing-tween with create-new-tween-per-target-change pattern; also added immediate `update()` call after `start()` so 0ms tweens transition to Completed state within the same `advance_*` call
- **Files modified:** `ora/src/animation/transition.rs`
- **Verification:** All 7 transition tests pass including `test_cleanup_removes_inactive` and `test_has_active_transitions_true_after_retarget`
- **Committed in:** 9d9adc2 (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - Bug)
**Impact on plan:** Fix was necessary for correct tween lifecycle. No scope creep.

## Issues Encountered

Three test iteration cycles before all 7 tests passed:
1. Initial instant-tween approach with `Tween::instant()` — retarget preserved 0-duration, breaking retarget tests
2. Fixed by creating new tween per retarget — but initial call still left tween in Playing state for 200ms
3. Final fix: added `tween.update()` immediately after `start()` in the target-changed path

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `TransitionRegistry` is ready for Plan 09-06 (Div paint-time interpolation)
- Plan 09-06 will call `cx.transition_registry.borrow_mut().get_or_create(id).advance_bg(target, &spec.bg.unwrap())` during Div paint
- `has_active_transitions()` is ready for renderer integration (request_redraw loop when transitions in flight)
- No blockers

---
*Phase: 09-transitions-integration*
*Completed: 2026-03-25*
