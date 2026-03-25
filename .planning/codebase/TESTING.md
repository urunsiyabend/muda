# Testing Patterns

**Analysis Date:** 2026-01-28

## Test Framework

**Runner:**
- Rust built-in testing framework (no external test runner needed)
- Criterion (v0.5) for benchmarks with HTML reports

**Assertion Library:**
- Built-in `assert!()`, `assert_eq!()` macros
- No external assertion library (using Rust stdlib)

**Run Commands:**
```bash
cargo test                  # Run all tests in all workspace crates
cargo test --lib          # Run library tests only
cargo test core_editor    # Run core_editor tests
cargo test --benches      # Run benchmarks
cargo test --release      # Run tests in release mode
```

## Test File Organization

**Location:**
- Co-located with source code
- Tests are in same file as implementation using `#[cfg(test)]` modules

**Naming:**
- Test modules: `#[cfg(test)] mod tests { ... }`
- Test functions: `#[test] fn test_*(...)`
- Helper functions (non-test): regular snake_case names

**Structure:**
- Each module with significant logic has a `#[cfg(test)]` section at the end
- Conditional compilation keeps tests close to implementation
- Example: `core_editor/src/commands/dispatcher.rs` has tests module at end

**Files with tests identified:**
- `core_editor/src/commands/dispatcher.rs` - 6 test functions
- `core_editor/src/commands/editor_command.rs` - 2 test functions
- `core_editor/src/commands/edit_operation.rs` - 5 test functions
- `core_editor/src/commands/history.rs` - 4 test functions
- `core_editor/benches/test_data.rs` - 3 test functions

## Test Structure

**Suite Organization:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_context() -> (Document, EditorView, CommandHistory, EventBus) {
        let doc = Document::from_str("Hello World", None);
        let view = EditorView::new(doc.id());
        let history = CommandHistory::new();
        let event_bus = EventBus::new();
        (doc, view, history, event_bus)
    }

    #[test]
    fn test_insert_char() {
        let (mut doc, mut view, mut history, mut event_bus) = create_test_context();
        let mut dispatcher = CommandDispatcher::new();

        let mut ctx = CommandContext {
            view: &mut view,
            document: &mut doc,
            history: &mut history,
            event_bus: &mut event_bus,
        };

        ctx.view.move_caret_to(11, false);
        let result = dispatcher.dispatch(EditorCommand::InsertChar('!'), &mut ctx);

        assert_eq!(result, DispatchResult::Executed);
        assert_eq!(ctx.document.content(), "Hello World!");
        assert_eq!(ctx.view.caret_offset(), 12);
    }
}
```

**Patterns:**
- **Setup:** Helper function `create_test_context()` returns all needed components
- **Teardown:** Implicit (Rust drops values at end of scope)
- **Assertion:** `assert_eq!()` for equality checks, `assert!()` for boolean conditions

## Mocking

**Framework:**
- No external mocking library (std library sufficient for Rust)
- Manual test doubles and fixtures

**Patterns:**
- Test contexts bundle all dependencies together
- Example `CommandContext` used for testing provides mutable references to:
  - View state
  - Document state
  - Command history
  - Event bus
- No trait-based mocking; direct struct instantiation in tests

**Test Data Generation:**
- `core_editor/benches/test_data.rs` provides data generators
- Functions: `generate_rust_source()`, `generate_plain_text()`, `generate_json()`
- Constants: `TestSizes::TINY`, `TestSizes::SMALL`, `TestSizes::MEDIUM`, etc.

**Example from test_data.rs:**
```rust
#[test]
fn test_rust_source_size() {
    let content = generate_rust_source(10_000);
    assert!(content.len() >= 9_000 && content.len() <= 10_000);
}
```

**What to Mock:**
- Clipboard access (handled gracefully with `Option<Clipboard>`)
- File I/O (tested by creating actual files, or by catching errors)
- Syntax highlighting (tested separately from document operations)

**What NOT to Mock:**
- Core document operations (TextBuffer, Rope data structure)
- View state (caret, selection)
- Command history (tested as real state machine)
- Event bus (tested as real event emission)

## Fixtures and Factories

**Test Data:**
```rust
fn create_test_context() -> (Document, EditorView, CommandHistory, EventBus) {
    let doc = Document::from_str("Hello World", None);
    let view = EditorView::new(doc.id());
    let history = CommandHistory::new();
    let event_bus = EventBus::new();
    (doc, view, history, event_bus)
}
```

**Specialized Fixtures from test_data.rs:**
```rust
pub fn generate_rust_source(target_bytes: usize) -> String { ... }
pub fn generate_plain_text(target_bytes: usize) -> String { ... }
pub fn generate_json(target_bytes: usize) -> String { ... }
pub fn generate_typing_input(char_count: usize) -> Vec<char> { ... }

pub struct TestSizes;
impl TestSizes {
    pub const TINY: usize = 1_000;          // 1 KB
    pub const SMALL: usize = 10_000;        // 10 KB
    pub const MEDIUM: usize = 100_000;      // 100 KB
    pub const LARGE: usize = 1_000_000;     // 1 MB
    pub const HUGE: usize = 10_000_000;     // 10 MB
}
```

**Location:**
- Test helpers: End of each module in `#[cfg(test)] mod tests`
- Benchmark data: `core_editor/benches/test_data.rs`

## Coverage

**Requirements:**
- No explicit coverage target enforced
- Tests focus on core command dispatching and data structure operations

**View Coverage:**
```bash
cargo tarpaulin                    # Measure coverage (if tarpaulin installed)
cargo llvm-cov                     # Alternative coverage tool
```

## Test Types

**Unit Tests:**
- **Scope:** Individual functions and command handlers
- **Approach:** Test single operation in isolation
- Examples:
  - `test_insert_char()` - single character insertion
  - `test_move_cursor()` - cursor movement validation
  - `test_select_all()` - selection operation
- Located in: `dispatcher.rs`, `edit_operation.rs`, `history.rs`

**Integration Tests:**
- **Scope:** Coordination between dispatcher, document, and view
- **Approach:** Test full command execution through context
- Example from `dispatcher.rs`:
```rust
#[test]
fn test_insert_char() {
    let (mut doc, mut view, mut history, mut event_bus) = create_test_context();
    let mut dispatcher = CommandDispatcher::new();
    let mut ctx = CommandContext { ... };

    // Test that insert updates document AND caret position
    dispatcher.dispatch(EditorCommand::InsertChar('!'), &mut ctx);
    assert_eq!(ctx.document.content(), "Hello World!");
    assert_eq!(ctx.view.caret_offset(), 12);
}
```

**Benchmarks:**
- **Framework:** Criterion (v0.5 with HTML reports)
- **Location:** `core_editor/benches/`
- **Configured in Cargo.toml:**
```toml
[[bench]]
name = "editor_benchmarks"
harness = false

[[bench]]
name = "text_buffer_benchmarks"
harness = false

[[bench]]
name = "syntax_benchmarks"
harness = false

[[bench]]
name = "render_benchmarks"
harness = false
```
- **Run:** `cargo bench --bench editor_benchmarks`

**E2E Tests:**
- Not currently used (UI-layer testing would be in client crates)
- Integration tests cover the domain logic thoroughly

## Common Patterns

**Async Testing:**
- Not currently used (core_editor is synchronous)
- Would use `#[tokio::test]` if async operations introduced

**Error Testing:**
```rust
#[test]
fn test_undo_delete() {
    let mut buffer = TextBuffer::from_str("Hello World");
    let op = EditOperation::delete(5, " World");
    op.execute(&mut buffer);
    op.undo(&mut buffer);
    assert_eq!(buffer.to_string(), "Hello World");
    assert_eq!(op.cursor_after_undo(), 11);
}
```
Pattern: Test both success path and undo/recovery path

**State Verification:**
- After each operation, verify multiple state changes
- Pattern: assert on document content, cursor position, and selection state
```rust
assert_eq!(result, DispatchResult::Executed);
assert_eq!(ctx.document.content(), "Hello World!");
assert_eq!(ctx.view.caret_offset(), 12);
```

## Testing Best Practices

**Setup/Teardown:**
- Use helper functions (`create_test_context()`) for reusable setup
- No explicit teardown needed (RAII drops values automatically)

**Isolation:**
- Each test creates fresh context
- No shared state between tests
- Command history starts empty for each test

**Naming:**
- Test names describe what is tested: `test_insert_char`, `test_move_cursor`, `test_undo_delete`
- Helper names are clear: `create_test_context`, `generate_rust_source`

**Assertions:**
- Multiple assertions per test are acceptable (verify related state changes)
- Use equality checks for concrete values
- Use boolean checks for state flags

---

*Testing analysis: 2026-01-28*
