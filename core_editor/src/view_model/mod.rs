//! ViewModel: Render-ready projection of editor state.
//!
//! The ViewModel layer decouples rendering from domain internals.
//! It provides a clean interface for the UI to draw without directly
//! accessing Document or EditorView internals.
//!
//! # Styling Architecture
//!
//! This module uses a two-layer styling approach:
//!
//! 1. **Domain-level styles** ([`TextStyle`]): Semantic token types that describe
//!    *what* something is (keyword, comment, selection, etc.) without specifying
//!    *how* it should look.
//!
//! 2. **Backend-specific styles**: The rendering layer (e.g., `draw.rs` for ratatui)
//!    maps domain styles to concrete visual styles (colors, modifiers, etc.).
//!
//! This separation allows the ViewModel to remain backend-agnostic, making it
//! easier to support alternative rendering backends (GUI, web, etc.) in the future.

pub mod builder;

pub use builder::PendingAction;

// =============================================================================
// Domain-Level Styling (UI-Agnostic)
// =============================================================================

/// Semantic text style tokens.
///
/// These represent *what* the text is, not *how* it should look.
/// The rendering layer maps these to concrete visual styles.
///
/// # Future-Proofing
///
/// When adding a non-TUI frontend, create a style mapper that converts
/// `TextStyle` to the appropriate backend representation (e.g., CSS classes
/// for web, or NSAttributedString attributes for macOS).
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
    /// Error or warning indicator.
    Error,
    /// Warning indicator.
    Warning,
}

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

/// A styled span of text within a line.
///
/// # Styling Approach
///
/// Uses semantic `TextStyle` tokens that describe *what* the text is
/// (keyword, comment, selection, etc.) without specifying visual details.
/// The rendering backend maps these to concrete styles.
#[derive(Clone, Debug)]
pub struct StyledSpan {
    /// The text content.
    pub text: String,

    /// Semantic style token (UI-agnostic).
    /// Rendering backends map this to their native style representation.
    pub style: TextStyle,
}

impl StyledSpan {
    /// Creates a styled span with a semantic style.
    pub fn new(text: impl Into<String>, style: TextStyle) -> Self {
        Self {
            text: text.into(),
            style,
        }
    }

    /// Creates an unstyled (normal) span.
    pub fn raw(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: TextStyle::Normal,
        }
    }

    /// Creates a selection-highlighted span.
    pub fn selection(text: impl Into<String>) -> Self {
        Self::new(text, TextStyle::Selection)
    }
}

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
        Self {
            line_number,
            is_current_line,
            spans: Vec::new(),
        }
    }

    pub fn with_spans(line_number: usize, is_current_line: bool, spans: Vec<StyledSpan>) -> Self {
        Self {
            line_number,
            is_current_line,
            spans,
        }
    }
}

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
        Self {
            width,
            visible,
            total_lines,
        }
    }

    /// Returns the width needed for just the line numbers (no separator).
    pub fn line_number_width(&self) -> usize {
        if self.visible {
            self.total_lines.to_string().len()
        } else {
            0
        }
    }
}

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

/// Visual representation of the cursor/caret.
#[derive(Clone, Debug, Default)]
pub struct CaretPresentation {
    /// Screen position of the caret.
    pub position: VisualPosition,
    /// Whether the caret is visible (within viewport).
    pub visible: bool,
}

/// Dialog state for rendering overlays.
#[derive(Clone, Debug)]
pub enum DialogPresentation {
    /// No dialog shown.
    None,
    /// Confirmation dialog for unsaved changes.
    ///
    /// Contains information about the action being confirmed and
    /// which documents have unsaved changes.
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
        Self {
            name,
            is_dir,
            is_selected,
        }
    }
}

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

/// Presentation data for a single tab in the tab bar.
#[derive(Clone, Debug)]
pub struct TabPresentation {
    /// The display title (filename or "[New File]").
    pub title: String,
    /// Whether this tab is currently active.
    pub is_active: bool,
    /// Whether the document has unsaved changes.
    pub is_dirty: bool,
}

impl TabPresentation {
    pub fn new(title: String, is_active: bool, is_dirty: bool) -> Self {
        Self { title, is_active, is_dirty }
    }
}

/// Presentation data for the tab bar showing open documents.
#[derive(Clone, Debug, Default)]
pub struct TabBarPresentation {
    /// The list of open document tabs.
    pub tabs: Vec<TabPresentation>,
    /// Whether the tab bar should be shown (true if more than one document or always).
    pub visible: bool,
}

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
    pub scroll_y: usize,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gutter_model_width() {
        let gutter = GutterModel::new(true, 100);
        assert_eq!(gutter.line_number_width(), 3); // "100" = 3 digits
        assert_eq!(gutter.width, 5); // 3 + space + separator

        let gutter = GutterModel::new(true, 9);
        assert_eq!(gutter.line_number_width(), 1);
        assert_eq!(gutter.width, 3);

        let gutter = GutterModel::new(false, 1000);
        assert_eq!(gutter.width, 0);
    }

    #[test]
    fn test_styled_span() {
        let span = StyledSpan::raw("hello");
        assert_eq!(span.text, "hello");
        assert_eq!(span.style, TextStyle::Normal);
    }

    #[test]
    fn test_styled_span_semantic() {
        let span = StyledSpan::new("selected", TextStyle::Selection);
        assert_eq!(span.text, "selected");
        assert_eq!(span.style, TextStyle::Selection);

        let span = StyledSpan::selection("also selected");
        assert_eq!(span.style, TextStyle::Selection);
    }

    #[test]
    fn test_line_presentation() {
        let line = LinePresentation::new(1, true);
        assert_eq!(line.line_number, 1);
        assert!(line.is_current_line);
        assert!(line.spans.is_empty());
    }

    #[test]
    fn test_render_model_title() {
        let mut model = RenderModel::default();
        model.status.title = "test.rs".to_string();
        model.status.dirty = false;
        assert_eq!(model.display_title(), "test.rs");

        model.status.dirty = true;
        assert_eq!(model.display_title(), "*test.rs");
    }

    #[test]
    fn test_dialog_presentation_unsaved_changes() {
        let dialog = DialogPresentation::unsaved_changes("Exit", vec!["file.txt".to_string()]);
        if let DialogPresentation::UnsavedChangesConfirmation { action_description, unsaved_documents } = dialog {
            assert_eq!(action_description, "Exit");
            assert_eq!(unsaved_documents, vec!["file.txt"]);
        } else {
            panic!("Expected UnsavedChangesConfirmation");
        }
    }
}
