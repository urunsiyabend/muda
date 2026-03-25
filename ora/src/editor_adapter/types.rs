//! Mirror presentation types for the editor adapter boundary.
//!
//! These types mirror the structures in `core_editor::view_model` with identical
//! field names and types. They are standalone types with no dependency on core_editor.
//!
//! The `From` conversions (core_editor types → these mirror types) are implemented
//! in `wgpu_client`, not here. This keeps ora free of core_editor as a dependency.

// =============================================================================
// TextStyle — semantic syntax highlighting tokens
// =============================================================================

/// Semantic text style tokens.
///
/// These represent *what* the text is, not *how* it should look.
/// The rendering layer maps these to concrete visual styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TextStyle {
    /// Normal text with no special styling.
    #[default]
    Normal,

    // === Selection & Cursor ===
    /// Selected text (highlighted region).
    Selection,

    // === Syntax Highlighting ===
    /// Language keyword (if, else, fn, struct, etc.).
    Keyword,
    /// String literal.
    String,
    /// Numeric literal.
    Number,
    /// Comment (single-line or multi-line).
    Comment,
    /// Type name or annotation.
    Type,
    /// Function or method name.
    Function,
    /// Variable or identifier.
    Variable,
    /// Operator (+, -, *, /, etc.).
    Operator,
    /// Punctuation (braces, parentheses, semicolons, etc.).
    Punctuation,
    /// Constant or enum variant.
    Constant,
    /// Module or namespace.
    Module,
    /// Attribute or annotation.
    Attribute,
    /// Macro invocation.
    Macro,

    // === UI Elements ===
    /// Line number in the gutter.
    LineNumber,
    /// Current line's line number (highlighted).
    CurrentLineNumber,
    /// Error indicator.
    Error,
    /// Warning indicator.
    Warning,
}

// =============================================================================
// VisualPosition — screen-space coordinate after viewport transformation
// =============================================================================

/// Position on screen (after viewport transformation).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VisualPosition {
    /// Row on screen (0-indexed from top of editor area).
    pub row: usize,
    /// Column on screen (0-indexed from left of text area).
    pub column: usize,
}

impl VisualPosition {
    pub fn new(row: usize, column: usize) -> Self {
        Self { row, column }
    }
}

// =============================================================================
// StyledSpan — a run of text with a semantic style
// =============================================================================

/// A styled span of text within a line.
#[derive(Clone, Debug)]
pub struct StyledSpan {
    /// The text content.
    pub text: String,
    /// Semantic style token (UI-agnostic).
    pub style: TextStyle,
}

impl StyledSpan {
    /// Creates a styled span with a semantic style.
    pub fn new(text: impl Into<String>, style: TextStyle) -> Self {
        Self { text: text.into(), style }
    }

    /// Creates an unstyled (normal) span.
    pub fn raw(text: impl Into<String>) -> Self {
        Self { text: text.into(), style: TextStyle::Normal }
    }

    /// Creates a selection-highlighted span.
    pub fn selection(text: impl Into<String>) -> Self {
        Self::new(text, TextStyle::Selection)
    }
}

// =============================================================================
// LinePresentation — a single rendered editor line
// =============================================================================

/// Presentation data for a single line in the editor.
#[derive(Clone, Debug)]
pub struct LinePresentation {
    /// The document line number (1-indexed for display).
    pub line_number: usize,
    /// Whether this is the line containing the cursor.
    pub is_current_line: bool,
    /// The styled spans that make up this line's content.
    pub spans: Vec<StyledSpan>,
}

impl LinePresentation {
    pub fn new(line_number: usize, is_current_line: bool) -> Self {
        Self { line_number, is_current_line, spans: Vec::new() }
    }

    pub fn with_spans(line_number: usize, is_current_line: bool, spans: Vec<StyledSpan>) -> Self {
        Self { line_number, is_current_line, spans }
    }
}

// =============================================================================
// GutterModel — line number gutter configuration
// =============================================================================

/// Model for the line number gutter.
#[derive(Clone, Debug, Default)]
pub struct GutterModel {
    /// Width of the gutter in characters (including separator).
    pub width: usize,
    /// Whether to show line numbers.
    pub visible: bool,
    /// Total number of lines in the document (for width calculation).
    pub total_lines: usize,
}

impl GutterModel {
    pub fn new(visible: bool, total_lines: usize) -> Self {
        let width = if visible {
            // digits + space + separator
            total_lines.to_string().len() + 2
        } else {
            0
        };
        Self { width, visible, total_lines }
    }

    /// Returns the width needed for just the line numbers (no separator).
    pub fn line_number_width(&self) -> usize {
        if self.visible { self.total_lines.to_string().len() } else { 0 }
    }
}

// =============================================================================
// StatusPresentation — status bar data
// =============================================================================

/// Presentation data for the status line.
#[derive(Clone, Debug, Default)]
pub struct StatusPresentation {
    /// The document title (filename or "[New File]").
    pub title: String,
    /// Whether the document has unsaved changes.
    pub dirty: bool,
    /// Current cursor line (1-indexed for display).
    pub cursor_line: usize,
    /// Current cursor column (1-indexed for display).
    pub cursor_column: usize,
    /// The detected language.
    pub language: String,
    /// Total number of lines.
    pub total_lines: usize,
    /// Optional status message (for feedback like save errors, confirmations).
    pub message: Option<String>,
}

// =============================================================================
// CaretPresentation — cursor position and visibility
// =============================================================================

/// Visual representation of the cursor/caret.
#[derive(Clone, Debug, Default)]
pub struct CaretPresentation {
    /// Screen position of the caret.
    pub position: VisualPosition,
    /// Whether the caret is visible (within viewport).
    pub visible: bool,
}

// =============================================================================
// DialogPresentation — overlay dialog state
// =============================================================================

/// Dialog state for rendering overlays.
#[derive(Clone, Debug)]
pub enum DialogPresentation {
    /// No dialog shown.
    None,
    /// Confirmation dialog for unsaved changes.
    UnsavedChangesConfirmation {
        /// Description of the action (e.g., "Exit", "Close file", "Open file").
        action_description: String,
        /// Titles of documents with unsaved changes.
        unsaved_documents: Vec<String>,
    },
}

impl Default for DialogPresentation {
    fn default() -> Self {
        Self::None
    }
}

impl DialogPresentation {
    /// Creates an unsaved changes confirmation dialog.
    pub fn unsaved_changes(action: impl Into<String>, documents: Vec<String>) -> Self {
        Self::UnsavedChangesConfirmation {
            action_description: action.into(),
            unsaved_documents: documents,
        }
    }
}

// =============================================================================
// FileEntryPresentation — a single entry in the sidebar file explorer
// =============================================================================

/// Presentation data for a single file entry in the sidebar.
#[derive(Clone, Debug)]
pub struct FileEntryPresentation {
    /// The display name of the file/directory.
    pub name: String,
    /// Whether this is a directory.
    pub is_dir: bool,
    /// Whether this entry is currently selected.
    pub is_selected: bool,
}

impl FileEntryPresentation {
    pub fn new(name: String, is_dir: bool, is_selected: bool) -> Self {
        Self { name, is_dir, is_selected }
    }
}

// =============================================================================
// SidebarPresentation — full sidebar state
// =============================================================================

/// Presentation data for the sidebar file explorer.
#[derive(Clone, Debug, Default)]
pub struct SidebarPresentation {
    /// Whether the sidebar is visible.
    pub visible: bool,
    /// Whether the sidebar is focused.
    pub focused: bool,
    /// The base directory name (for title).
    pub directory_name: String,
    /// The visible file entries.
    pub entries: Vec<FileEntryPresentation>,
    /// Width of the sidebar in characters.
    pub width: usize,
}

// =============================================================================
// TabPresentation — a single tab in the tab bar
// =============================================================================

/// Presentation data for a single tab in the tab bar.
#[derive(Clone, Debug)]
pub struct TabPresentation {
    /// The view ID for this tab (used for switching views on click).
    pub view_id: u64,
    /// The display title (filename or "[New File]").
    pub title: String,
    /// Whether this tab is currently active.
    pub is_active: bool,
    /// Whether the document has unsaved changes.
    pub is_dirty: bool,
}

impl TabPresentation {
    pub fn new(view_id: u64, title: String, is_active: bool, is_dirty: bool) -> Self {
        Self { view_id, title, is_active, is_dirty }
    }
}

// =============================================================================
// FileTreeNode — hierarchical file tree node
// =============================================================================

/// A node in the file tree hierarchy.
#[derive(Clone, Debug)]
pub struct FileTreeNode {
    /// Display name (file or directory name).
    pub name: String,
    /// File extension (e.g., "rs", "js") for icon coloring. Empty for directories.
    pub extension: String,
    /// Whether this is a directory.
    pub is_dir: bool,
    /// Whether this directory is expanded (only meaningful for directories).
    pub is_expanded: bool,
    /// Nested children (only for directories).
    pub children: Vec<FileTreeNode>,
}

impl FileTreeNode {
    /// Create a file node with the given name and extension.
    pub fn file(name: impl Into<String>, extension: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            extension: extension.into(),
            is_dir: false,
            is_expanded: false,
            children: vec![],
        }
    }

    /// Create a directory node with the given name, expanded state, and children.
    pub fn dir(name: impl Into<String>, expanded: bool, children: Vec<FileTreeNode>) -> Self {
        Self {
            name: name.into(),
            extension: String::new(),
            is_dir: true,
            is_expanded: expanded,
            children,
        }
    }
}

// =============================================================================
// FileTreePresentation — full file tree state
// =============================================================================

/// Presentation data for the file tree.
#[derive(Clone, Debug, Default)]
pub struct FileTreePresentation {
    /// Root-level nodes.
    pub roots: Vec<FileTreeNode>,
    /// Index of the currently selected entry in the flattened list.
    pub selected_index: Option<usize>,
}

// =============================================================================
// TabBarPresentation — all open tabs
// =============================================================================

/// Presentation data for the tab bar showing open documents.
#[derive(Clone, Debug, Default)]
pub struct TabBarPresentation {
    /// The list of open document tabs.
    pub tabs: Vec<TabPresentation>,
    /// Whether the tab bar should be shown.
    pub visible: bool,
}

// =============================================================================
// RenderModel — the complete per-frame render payload
// =============================================================================

/// The complete render-ready model for the editor UI.
///
/// This struct contains everything the UI needs to draw a frame,
/// without any direct access to domain objects.
#[derive(Clone, Debug, Default)]
pub struct RenderModel {
    /// The visible lines to render.
    pub visible_lines: Vec<LinePresentation>,
    /// Gutter configuration.
    pub gutter: GutterModel,
    /// Caret position for cursor rendering.
    pub caret: CaretPresentation,
    /// Status line data.
    pub status: StatusPresentation,
    /// Tab bar data for open documents.
    pub tab_bar: TabBarPresentation,
    /// Active dialog (if any).
    pub dialog: DialogPresentation,
    /// Sidebar presentation data.
    pub sidebar: SidebarPresentation,
    /// Viewport scroll offset (for reference).
    pub scroll_x: usize,
    /// Viewport scroll offset (for reference).
    pub scroll_y: usize,
    /// Sub-line vertical scroll offset in pixels (0.0 .. LINE_HEIGHT).
    ///
    /// Set by the ora event loop's pixel accumulator for smooth scrolling.
    /// Views apply this as a negative vertical shift to the text content,
    /// producing smooth pixel-level scrolling between logical line boundaries.
    pub scroll_y_offset_px: f32,
}

impl RenderModel {
    /// Returns the title with dirty indicator.
    pub fn display_title(&self) -> String {
        if self.status.dirty {
            format!("*{}", self.status.title)
        } else {
            self.status.title.clone()
        }
    }
}

// =============================================================================
// EditorCommand — commands dispatched from UI to editor backend
// =============================================================================

/// Commands that the UI can dispatch to the editor backend.
///
/// Mirrors `core_editor::commands::EditorCommand` variants used in
/// `wgpu_client/src/input.rs`. The `From` conversion is implemented in
/// `wgpu_client`, keeping this type free of core_editor.
#[derive(Clone, Debug)]
pub enum EditorCommand {
    // --- Cursor movement ---
    /// Move cursor in a direction, optionally extending the selection.
    MoveCursor {
        direction: CursorDirection,
        scope: MoveScope,
        extend_selection: bool,
    },

    // --- Text editing ---
    /// Insert a single character at the cursor.
    InsertChar(char),
    /// Insert a string of text at the cursor.
    InsertText(String),
    /// Insert a newline at the cursor.
    InsertNewline,
    /// Delete the character before the cursor (backspace).
    Backspace,
    /// Delete the character after the cursor (delete key).
    Delete,

    // --- Clipboard ---
    /// Copy selected text to clipboard.
    Copy,
    /// Cut selected text to clipboard.
    Cut,
    /// Paste from clipboard.
    Paste,

    // --- Selection ---
    /// Select all text in the document.
    SelectAll,

    // --- History ---
    /// Undo the last action.
    Undo,
    /// Redo the previously undone action.
    Redo,

    // --- File operations ---
    /// Save the current document.
    Save,

    // --- View toggles ---
    /// Toggle line number visibility.
    ToggleLineNumbers,

    // --- Scrolling ---
    /// Scroll the viewport by the given number of lines (positive = down).
    Scroll(i32),
}

/// Direction for cursor movement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CursorDirection {
    Left,
    Right,
    Up,
    Down,
}

/// Scope (granularity) for cursor movement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveScope {
    /// Single character / single line.
    Char,
    /// Word boundary.
    Word,
    /// Beginning or end of line.
    Line,
    /// One page (viewport height) up or down.
    Page,
    /// Beginning or end of document.
    Document,
}
