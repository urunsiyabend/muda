# Coding Conventions

**Analysis Date:** 2026-01-28

## Naming Patterns

**Files:**
- Lowercase with underscores: `editor_command.rs`, `text_buffer.rs`, `dispatcher.rs`
- Module files use `mod.rs` for directory re-exports
- Test data generators: `test_data.rs`
- Benchmarks: `*_benchmarks.rs`

**Functions:**
- snake_case: `handle_insert_char()`, `move_caret_to()`, `create_test_context()`
- Command handlers: `handle_*` prefix for dispatcher methods
- Test functions: `test_*` prefix
- Helper functions in tests: lowercase descriptive names

**Variables:**
- snake_case for local variables and parameters
- Short names for loop iterators: `i`, `ch`
- Descriptive names for complex data: `affected_range`, `caret_offset`, `event_bus`
- Prefix conventions:
  - `new_*` for computed values: `new_offset`, `new_line`, `new_col`

**Types:**
- PascalCase: `Document`, `TextBuffer`, `EditorView`, `CommandDispatcher`, `TextPosition`
- Enum variants: PascalCase: `Executed`, `RequiresAppHandling`, `NoOp`
- Type aliases: PascalCase: `TextOffset`, `DocumentRevision`

**Module Paths:**
- Organized by domain: `commands`, `domain`, `view`, `view_model`, `syntax`, `events`
- Re-exports at crate root for commonly used types: see `lib.rs`

## Code Style

**Formatting:**
- Rust standard formatting conventions
- No specific `.rustfmt.toml` detected; uses Rust defaults
- Editor Edition 2024 (modern Rust standards)
- Consistent spacing and indentation across all files

**Linting:**
- No explicit `.clippy.toml` found; uses default clippy warnings
- Pattern: dead code is explicitly allowed with `#[allow(dead_code)]` when retained for backward compatibility
- Example: `app.rs` legacy accessor methods marked as deprecated with explanatory comments

## Import Organization

**Order:**
1. Standard library imports
2. External crate imports (e.g., `arboard::Clipboard`, `log::debug`)
3. Internal crate imports (e.g., `crate::commands::*`, `crate::domain::*`)

**Example from `dispatcher.rs`:**
```rust
use arboard::Clipboard;
use log::debug;

use crate::commands::{CommandHistory, EditOperation, EditorCommand};
use crate::commands::editor_command::{Direction, MoveScope};
use crate::domain::{Document, TextPosition, TextRange};
use crate::events::EventBus;
use crate::view::EditorView;
```

**Path Aliases:**
- No explicit path aliases configured; relative module paths used throughout

## Error Handling

**Patterns:**
- Result<T> for fallible operations: `std::io::Result<T>`
- Option<T> for optional values
- Early returns with `?` operator common in main paths
- Match expressions for explicit error handling in critical sections
- Graceful degradation: example in `app.rs` line 71-83 where file open errors fall back to new document

**Logging on errors:**
- Debug-level logging for recoverable errors: `log::debug!("Could not open file: {}", e)`
- Pattern: log then continue (don't panic on I/O errors)

## Logging

**Framework:** `log` crate (v0.4.29)

**Patterns:**
- `log::debug!()` for operation tracking and diagnostic info
- Consistent message format with context and values
- Examples:
  - `log::debug!("Opened file: {}", path)` - successful operations
  - `log::debug!("Could not open file from sidebar: {:?}, error: {}", path, e)` - error context
  - Used in command dispatch and file operations

## Comments

**When to Comment:**
- Module-level documentation: `//!` style comments at top of files explaining purpose
- Struct/enum documentation: `///` for public types explaining semantic meaning
- Implementation sections: `// === Section Name ===` comments for logical grouping
- Cross-cutting logic: Comments explaining "why" not just "what"
- Example from `app.rs`: "Cross-cutting Concerns" sections (lines 625+) group related functionality

**JSDoc/TSDoc Style:**
- Rust uses `///` doc comments (not JavaScript style)
- Every public type and function should have doc comment
- Examples found in:
  - `app.rs` - extensive `pub fn` documentation
  - `dispatcher.rs` - struct and enum documentation with purpose statements
  - `text_buffer.rs` - type alias documentation explaining semantic meaning

**Documentation Example:**
```rust
/// A scalar position in the buffer (character index).
pub type TextOffset = usize;

/// Human-facing location in (line, column) terms.
/// Both line and column are 0-indexed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct TextPosition {
    pub line: usize,
    pub column: usize,
}
```

## Function Design

**Size:**
- Functions range from 3-40 lines typically
- Dispatcher handlers are 5-20 lines
- Helper methods are 2-10 lines
- Complex logic (e.g., word movement) is 15-30 lines with internal comments

**Parameters:**
- Direct parameters for simple types: `ch: char`, `offset: usize`
- Struct references for complex state: `ctx: &mut CommandContext`
- Mutable references for state changes
- Pattern: context object (`CommandContext`) aggregates multiple related references

**Return Values:**
- `DispatchResult` enum for command dispatch results
- `Option<T>` for optional values
- `Result<T, E>` for fallible operations
- Direct values for simple queries (e.g., `usize` for offsets)

**Example pattern from `dispatcher.rs`:**
```rust
fn move_left_char(&self, ctx: &CommandContext) -> usize {
    let offset = ctx.view.caret_offset();
    if offset > 0 { offset - 1 } else { 0 }
}
```

## Module Design

**Exports:**
- `pub` for public API types and functions
- No `pub use` re-exports within modules; selective re-exports at crate root in `lib.rs`
- Library crates (`core_editor`) export public API; binary crates (`wgpu_client`) have private implementation

**Barrel Files:**
- `mod.rs` used for domain module organization
- Example: `crate/domain/mod.rs` re-exports all domain types
- Pattern in `crate/commands/mod.rs` - groups all command-related exports

**Module Structure:**
- Logical grouping by domain concern (commands, domain, view, etc.)
- Each module focuses on single responsibility
- Clear module boundaries prevent circular dependencies

## Design Patterns

**Command Pattern:**
- `EditorCommand` enum defines all possible commands
- `CommandDispatcher` routes and executes commands
- `CommandContext` aggregates state references for command handlers
- `CommandHistory` for undo/redo

**Builder Pattern:**
- `ViewModelBuilder` used to construct render models from domain state
- `Builder::build()` method constructs complex objects

**Observer Pattern:**
- `EventBus` for event publishing
- Domain objects emit events (e.g., `document.event_changed()`)
- Events use specific event types per domain object

**Context Object Pattern:**
- `CommandContext` aggregates multiple mutable references
- Avoids parameter explosion in handler functions
- Prevents lifetime issues with multiple mutable borrows

## Traits and Abstractions

**Common Traits:**
- `Default` impl for initializable types
- `Clone`, `Copy` for value types (e.g., `TextPosition`, `TextRange`)
- `Debug` for all types
- `PartialEq`, `Eq` for value types

**Trait Bounds:**
- `impl Into<String>` for flexible string conversions: `set_status_message(impl Into<String>)`
- Minimal trait bounds in function signatures

## Deprecation Pattern

**Legacy Code Management:**
- Deprecated methods marked with `#[allow(dead_code)]` and `/// **DEPRECATED:**` doc comments
- Explanation of why deprecated and recommended alternative provided
- Example in `app.rs` lines 685-734: legacy direct access methods with detailed comments

---

*Convention analysis: 2026-01-28*
