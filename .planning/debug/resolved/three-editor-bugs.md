---
status: fixing
trigger: "Three bugs: 40-line limit/no scroll, lines overlap when narrow, caret not visible"
created: 2026-03-25T00:00:00Z
updated: 2026-03-25T00:00:00Z
---

## Current Focus

hypothesis: Three separate bugs with clear root causes identified from code review
test: Fix each bug, cargo check, cargo test
expecting: All three issues resolved
next_action: Fix Bug 1 (viewport sizing), then Bug 2 (line overlap), then Bug 3 (caret)

## Symptoms

expected: Scrolling past 40 lines, fixed line heights when window narrow, visible caret
actual: Text disappears after line 40, lines overlap when narrow, no caret visible
errors: None (visual bugs)
reproduction: Type 40+ lines, resize window narrow, observe missing caret
started: Since GPU rendering was wired up

## Evidence

- timestamp: 2026-03-25T00:01:00Z
  checked: editor_root.rs render()
  found: viewport_lines hardcoded to 40; EditorDataSource has no resize_viewport method
  implication: Bug 1 root cause confirmed - viewport never updates with window size

- timestamp: 2026-03-25T00:02:00Z
  checked: text_area.rs render(), gutter.rs render()
  found: Each line div has h(px(LINE_HEIGHT)) = fixed 21px. Container uses grow(1.0) + h(pct(100.0))
  implication: Lines themselves are fixed height. The parent flex container may compress them.

- timestamp: 2026-03-25T00:03:00Z
  checked: text_area.rs render_caret()
  found: Caret is rendered but stored in _caret (unused variable). The render() method builds text_lines
         but never includes the caret element in the returned Div's children.
  implication: Bug 3 root cause confirmed - caret element is created but discarded

## Resolution

root_cause: |
  Bug 1: viewport_lines=40 hardcoded, no dynamic viewport sizing from window dimensions
  Bug 2: Text area uses grow(1.0) which can compress children below natural size
  Bug 3: Caret element computed but never added to the render tree (stored as _caret, unused)
fix: See implementation
verification: pending
files_changed: []
