# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-26)

**Core value:** A functional, performant code editor built on ora's GPU-accelerated UI framework.
**Current focus:** v2.0 Functional Editor — Phase 12: File Operations (Phase 11 complete).

## Current Position

Phase: 12 — File Operations (in progress)
Plan: 02 of 3 (complete)
Status: In progress — Plan 02 done
Last activity: 2026-03-26 — Completed 12-02-PLAN.md (Ctrl+O native file dialog, async I/O, UTF-8 validation, BOM strip, Buffer Registry dedup)

Progress: [██████████░░░░░░░░░░░░░░░░░░░░░░░░░░] v2.0 Phase 12 in progress (10/~17 plans)

## Performance Metrics

**v1.0 Velocity (archived):**
- Total plans completed: 40 (Phase 1: 3, Phase 2: 6, Phase 3: 4, Phase 4: 5, Phase 5: 5, Phase 6: 4, Phase 7: 5, Phase 8: 8, Phase 8.1: 3, Phase 9: 9)
- Average duration: ~7.5m per plan
- Total execution time: ~5 hours 34 minutes

*Updated after each plan completion*

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
v1.0 decisions carried forward — see MILESTONES.md for full history.

### Pending Todos

None.

### Roadmap Evolution

- 2026-03-26: v2.0 roadmap created with Phases 10-16 (7 phases, 40 requirements)
  - Source material stated "34 requirements" but actual count is 40: FIX(5) + TAB(5) + FILE(6) + SIDE(6) + SEL(7) + FIND(8) + PERF(3)

### Blockers/Concerns

- Async file I/O return path: RESOLVED (12-02) — Full flow: dispatch_command queues PendingFileOp → poll_pending_file_ops() → LocalExecutor future → background thread reads file → adapter callback delivers result; rfd AsyncFileDialog + futures-lite yield_now pattern
- FileOpDataSource sub-trait: RESOLVED (12-02) — file-op callbacks isolated in FileOpDataSource sub-trait; EditorDataSource super-trait updated; PendingFileOp relocated to ora
- EditorDataSource trait pressure: RESOLVED (10-02) — split into BufferDataSource + CommandDispatcher + WindowDataSource with blanket super-trait
- Selection rendering: RESOLVED (10-03) — selection_ranges in LinePresentation, 5-layer Stack, span filter removed
- Idle GPU fixed (10-01): unconditional request_redraw at event_loop.rs removed; ControlFlow state machine now drives frame cadence

### Known Issues

- Windows resize flickering: Brief black/white flicker during window resize on Windows is expected wgpu/winit swap chain reconfiguration behavior, not an ora bug
- Window close (X button) doesn't check for dirty documents — exits without save dialog
- Dialog butonları (Save/Don't Save/Cancel) tıklanamıyor — hitbox/event wiring eksik
- Dispatch system fires handlers in both capture+bubble phases — all handlers must check `ctx.phase() == Bubble`

## Session Continuity

Last session: 2026-03-26T18:21:00Z
Stopped at: Completed 12-02-PLAN.md
Resume file: None
Next: Phase 12 Plan 03 — Save As / Save dialog integration

---
*State initialized: 2026-01-28*
*Last updated: 2026-03-26 after Phase 11 completion*
