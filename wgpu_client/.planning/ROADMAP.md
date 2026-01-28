# Roadmap: Muda IDE (wgpu_client)

## Overview

This roadmap takes the GPU-accelerated IDE from a broken state with critical rendering bugs to a stable, tested, and performant foundation. We'll fix the core rendering architecture first (disappearing elements, missing text, layout overflow), then establish visual regression testing to prevent regressions, and finally optimize startup time. Each phase builds on the previous to create a first-grade IDE experience.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Fix Core Rendering** - Resolve critical rendering bugs (disappearing elements, missing text, layout overflow)
- [ ] **Phase 2: Visual Interactions** - Add visual feedback for hover, click, focus, and navigation
- [ ] **Phase 3: Testing Infrastructure** - Set up visual regression testing framework
- [ ] **Phase 4: Component Snapshots** - Create baseline snapshots for all UI components
- [ ] **Phase 5: Startup Optimization** - Reduce startup time to under 300ms

## Phase Details

### Phase 1: Fix Core Rendering
**Goal**: All UI elements render correctly with no disappearing elements, visible text, and proper layout boundaries
**Depends on**: Nothing (first phase)
**Requirements**: FIX-01, FIX-02, FIX-03, FIX-04
**Success Criteria** (what must be TRUE):
  1. Sidebar icons and text remain visible after clicking (no progressive disappearance)
  2. Tab bar displays file names visibly and legibly
  3. Tab bar items are clickable with proper hit detection
  4. Text content stays within text area bounds (no overflow into tab bar or other regions)
  5. User can interact with all components without visual corruption
**Plans**: 4 plans in 3 waves

Plans:
- [x] 01-01-PLAN.md — Fix glyphon text rendering with single prepare() call
- [x] 01-02-PLAN.md — Add scissor rectangle clipping for component bounds
- [x] 01-03-PLAN.md — Add debug overlay infrastructure
- [x] 01-04-PLAN.md — Polish and verify visual appearance

### Phase 2: Visual Interactions
**Goal**: UI provides clear visual feedback for all user interactions
**Depends on**: Phase 1
**Requirements**: VIS-01, VIS-02, VIS-03, VIS-04, VIS-05
**Success Criteria** (what must be TRUE):
  1. Navigation between sidebar, tabs, and text area is seamless (no flicker, no lag)
  2. Dialogs render above all other content with proper z-ordering
  3. Components display hover states on mouse over
  4. Components display click feedback on mouse down
  5. Focused component is visually indicated
**Plans**: TBD

Plans:
- [ ] TBD

### Phase 3: Testing Infrastructure
**Goal**: Visual regression testing framework is operational and can capture screenshots
**Depends on**: Phase 2
**Requirements**: TEST-01, TEST-02
**Success Criteria** (what must be TRUE):
  1. Visual regression test framework is set up (insta + image-compare)
  2. Screenshot capture works for wgpu render output (render-to-texture)
  3. Tests can be run locally with cargo test
  4. Developer can review snapshot changes with cargo-insta review
**Plans**: TBD

Plans:
- [ ] TBD

### Phase 4: Component Snapshots
**Goal**: All UI components have baseline snapshots that detect rendering changes
**Depends on**: Phase 3
**Requirements**: TEST-03, TEST-04, TEST-05, TEST-06, TEST-07
**Success Criteria** (what must be TRUE):
  1. Baseline snapshots exist for sidebar component
  2. Baseline snapshots exist for tab bar component
  3. Baseline snapshots exist for text area component
  4. Baseline snapshots exist for status bar component
  5. Visual tests fail when rendering changes unexpectedly
**Plans**: TBD

Plans:
- [ ] TBD

### Phase 5: Startup Optimization
**Goal**: IDE starts quickly with no perceivable delay
**Depends on**: Phase 4
**Requirements**: FIX-05
**Success Criteria** (what must be TRUE):
  1. Startup time is under 300ms on desktop hardware
  2. GPU initialization does not block the main thread
  3. User sees responsive UI immediately on launch
**Plans**: TBD

Plans:
- [ ] TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4 → 5

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Fix Core Rendering | 4/4 | ✓ Complete | 2026-01-28 |
| 2. Visual Interactions | 0/TBD | Not started | - |
| 3. Testing Infrastructure | 0/TBD | Not started | - |
| 4. Component Snapshots | 0/TBD | Not started | - |
| 5. Startup Optimization | 0/TBD | Not started | - |
