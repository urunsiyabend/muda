# Muda IDE (wgpu_client)

## What This Is

A GPU-accelerated IDE frontend built with wgpu, designed to be performant and highly extensible. This crate provides the desktop GUI for the Muda editor, consuming the core_editor crate for text editing logic. The goal is a first-grade IDE experience with seamless navigation, responsive layouts, and visual polish.

## Core Value

The IDE renders correctly and performs smoothly — no visual glitches, no layout overflow, no disappearing elements. If the GUI is broken, nothing else matters.

## Requirements

### Validated

- Layered architecture (Application → State → Renderer → Components → Design System) — existing
- Component-based UI with GPU resource ownership — existing
- Design token system for consistent theming — existing
- Event-driven rendering with winit integration — existing
- Text rendering with glyphon glyph cache — existing
- Core editor integration (text buffer, cursors, commands) — existing

### Active

- [ ] Sidebar renders correctly (icons/text don't disappear on click)
- [ ] Tab bar displays file names visibly and clickably
- [ ] Text content area respects bounds (no overflow into tab bar)
- [ ] Startup performance is fast (no perceivable delay)
- [ ] Layout is responsive across different viewport sizes
- [ ] Navigation between components is seamless
- [ ] Visual regression testing infrastructure catches future breakages

### Out of Scope

- New features beyond bug fixes — ship working first, then extend
- Plugin system implementation — architecture exists, defer until GUI stable
- TUI client (ratatui_client) — separate crate, not in scope
- Core editor changes — consumed as dependency, not modified here

## Context

**Existing Codebase:**
- Component system: TextArea, Gutter, StatusBar, TabBar, Dialog, Sidebar, Caret (legacy) plus newer UI components
- Design system: tokens (colors, spacing, sizing), primitives (StyledRect, TextBlock), interaction state, layout, animation
- Rendering pipeline: event → state update → layout calculation → component preparation → GPU rendering → frame submission
- Dependencies: wgpu v23, winit v0.30, glyphon v0.7, pollster, bytemuck

**Known Issues:**
- Sidebar: clicking causes icons/text to disappear progressively
- Tab bar: file names not visible
- Text area: content overflows into tab bar region
- Startup: noticeable delay on initial launch
- General: multiple visual bugs requiring tedious manual verification

**Architecture Strengths:**
- Clean separation between core editor and rendering frontend
- Design system provides consistent theming primitives
- Component isolation (each owns GPU resources)
- Event-driven architecture suitable for responsive UI

## Constraints

- **Tech stack**: Rust, wgpu, winit, glyphon — established, not changing
- **Core editor**: External dependency — fix rendering, not core logic
- **GPU rendering**: All UI is GPU-rendered — no DOM, no browser testing tools
- **Verification**: Manual testing is slow — need automated visual regression

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Visual regression testing over unit tests | GPU-rendered UI needs screenshot comparison, not DOM assertions | — Pending |
| Fix bugs before new features | Broken GUI blocks all progress | — Pending |
| Maintain layered architecture | Extensibility depends on stable boundaries | — Pending |

---
*Last updated: 2026-01-28 after initialization*