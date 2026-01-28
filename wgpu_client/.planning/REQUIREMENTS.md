# Requirements: Muda IDE (wgpu_client)

**Defined:** 2026-01-28
**Core Value:** The IDE renders correctly and performs smoothly — no visual glitches, no layout overflow, no disappearing elements.

## v1 Requirements

Requirements for this milestone. Each maps to roadmap phases.

### Rendering Bug Fixes

- [ ] **FIX-01**: Sidebar icons and text remain visible after clicking (no progressive disappearance)
- [ ] **FIX-02**: Tab bar displays file names visibly and legibly
- [ ] **FIX-03**: Tab bar items are clickable with proper hit detection
- [ ] **FIX-04**: Text content stays within text area bounds (no overflow into tab bar or other regions)
- [ ] **FIX-05**: Startup time is under 300ms (currently 500ms-2s)

### Visual Quality

- [ ] **VIS-01**: Navigation between sidebar, tabs, and text area is seamless (no flicker, no lag)
- [ ] **VIS-02**: Dialogs render above all other content with proper z-ordering
- [ ] **VIS-03**: Components display hover states on mouse over
- [ ] **VIS-04**: Components display click feedback on mouse down
- [ ] **VIS-05**: Focused component is visually indicated

### Testing Infrastructure

- [ ] **TEST-01**: Visual regression test framework is set up (insta + image-compare)
- [ ] **TEST-02**: Screenshot capture works for wgpu render output (render-to-texture)
- [ ] **TEST-03**: Baseline snapshots exist for sidebar component
- [ ] **TEST-04**: Baseline snapshots exist for tab bar component
- [ ] **TEST-05**: Baseline snapshots exist for text area component
- [ ] **TEST-06**: Baseline snapshots exist for status bar component
- [ ] **TEST-07**: Visual tests fail when rendering changes unexpectedly

## v2 Requirements

Deferred to future milestone. Tracked but not in current roadmap.

### Layout

- **LAYOUT-01**: Layout adapts responsively across different viewport sizes
- **LAYOUT-02**: Window resizing maintains proper proportions

### CI/CD

- **CI-01**: Visual regression tests run automatically on every commit
- **CI-02**: Failed visual tests upload artifacts for review
- **CI-03**: GitHub Actions workflow for automated testing

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| New features beyond bug fixes | Ship working first, then extend |
| Plugin system implementation | Architecture exists, defer until GUI stable |
| TUI client (ratatui_client) | Separate crate, not in scope |
| Core editor changes | Consumed as dependency, not modified here |
| Real-time layout animations | GPU overhead, not essential for stability |
| Advanced animation system | Polish after bugs are fixed |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| FIX-01 | Phase 1 | Pending |
| FIX-02 | Phase 1 | Pending |
| FIX-03 | Phase 1 | Pending |
| FIX-04 | Phase 1 | Pending |
| VIS-01 | Phase 2 | Pending |
| VIS-02 | Phase 2 | Pending |
| VIS-03 | Phase 2 | Pending |
| VIS-04 | Phase 2 | Pending |
| VIS-05 | Phase 2 | Pending |
| TEST-01 | Phase 3 | Pending |
| TEST-02 | Phase 3 | Pending |
| TEST-03 | Phase 4 | Pending |
| TEST-04 | Phase 4 | Pending |
| TEST-05 | Phase 4 | Pending |
| TEST-06 | Phase 4 | Pending |
| TEST-07 | Phase 4 | Pending |
| FIX-05 | Phase 5 | Pending |

**Coverage:**
- v1 requirements: 17 total
- Mapped to phases: 17
- Unmapped: 0

---
*Requirements defined: 2026-01-28*
*Last updated: 2026-01-28 after roadmap creation*
