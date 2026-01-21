//! ViewModel: Render-ready projection of editor state.
//!
//! The ViewModel layer decouples rendering from domain internals.
//! It provides a clean interface for the UI to draw without directly
//! accessing Document or EditorView internals.

pub mod builder;

use ratatui::style::Style;

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
#[derive(Clone, Debug)]
pub struct StyledSpan {
    /// The text content.
    pub text: String,
    /// The style to apply.
    pub style: Style,
}

impl StyledSpan {
    pub fn new(text: impl Into<String>, style: Style) -> Self {
        Self {
            text: text.into(),
            style,
        }
    }

    pub fn raw(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: Style::default(),
        }
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
    /// Exit confirmation dialog for unsaved changes.
    ExitConfirmation,
}

impl Default for DialogPresentation {
    fn default() -> Self {
        Self::None
    }
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
    /// Active dialog (if any).
    pub dialog: DialogPresentation,
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
        assert_eq!(span.style, Style::default());
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
}
