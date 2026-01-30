//! Editor Chrome Demo: Integration showcase of all Phase 7 views
//!
//! Demonstrates:
//! 1. TabBarView - horizontal tab bar with dirty indicator and close buttons
//! 2. StatusBarView - bottom status bar with cursor position, language
//! 3. SidebarView - collapsible file explorer panel with visible toggle
//! 4. GutterView - line number gutter
//! 5. TextAreaView - syntax-highlighted text with caret and current line highlight
//! 6. DialogView - modal overlay for unsaved changes
//! 7. Theme switching (dark/light)
//!
//! Keyboard controls:
//! - 'T': Toggle theme (dark/light)
//! - 'D': Toggle dialog visibility
//! - 'S': Toggle sidebar collapsed/expanded
//! - Ctrl+Tab: Cycle to next tab
//! - Ctrl+Shift+Tab: Cycle to previous tab (same as Ctrl+Tab for now)

use ora::{
    AnyElement, App, Div, Model, View, ViewContext,
    pct, px,
    ColorToken,
    TabBarView, StatusBarView, SidebarView, GutterView, TextAreaView, DialogView,
    stack,
};
use core_editor::view_model::{
    TabBarPresentation, TabPresentation, StatusPresentation,
    SidebarPresentation, GutterModel, LinePresentation, StyledSpan,
    DialogPresentation, CaretPresentation, VisualPosition, TextStyle,
    RenderModel,
};

/// Mock editor state containing all presentation data for Phase 7 views.
#[derive(Clone)]
struct MockEditorState {
    /// Tab bar presentation data.
    tab_bar: TabBarPresentation,
    /// Status bar presentation data.
    status: StatusPresentation,
    /// Gutter model for line numbers.
    gutter: GutterModel,
    /// Visible lines with syntax highlighting.
    visible_lines: Vec<LinePresentation>,
    /// Caret/cursor position.
    caret: CaretPresentation,
    /// Dialog state (None or UnsavedChangesConfirmation).
    dialog: DialogPresentation,
    /// Sidebar presentation.
    sidebar: SidebarPresentation,
    /// Current active tab index.
    active_tab: usize,
    /// Whether sidebar is collapsed.
    sidebar_collapsed: bool,
}

impl Default for MockEditorState {
    fn default() -> Self {
        Self::new()
    }
}

impl MockEditorState {
    fn new() -> Self {
        Self {
            tab_bar: Self::create_tab_bar(),
            status: Self::create_status(),
            gutter: GutterModel::new(true, 156),
            visible_lines: Self::create_visible_lines(),
            caret: CaretPresentation {
                position: VisualPosition::new(0, 5),
                visible: true,
            },
            dialog: DialogPresentation::None,
            sidebar: Self::create_sidebar(),
            active_tab: 0,
            sidebar_collapsed: false,
        }
    }

    fn create_tab_bar() -> TabBarPresentation {
        TabBarPresentation {
            tabs: vec![
                TabPresentation::new(1, "main.rs".to_string(), true, false),
                TabPresentation::new(2, "*untitled.txt".to_string(), false, true),
                TabPresentation::new(3, "lib.rs".to_string(), false, false),
            ],
            visible: true,
        }
    }

    fn create_status() -> StatusPresentation {
        StatusPresentation {
            title: "main.rs".to_string(),
            dirty: false,
            cursor_line: 42,
            cursor_column: 15,
            language: "Rust".to_string(),
            total_lines: 156,
            message: None,
        }
    }

    fn create_sidebar() -> SidebarPresentation {
        SidebarPresentation {
            visible: true,
            focused: false,
            directory_name: "my_project".to_string(),
            entries: vec![],
            width: 220,
        }
    }

    fn create_visible_lines() -> Vec<LinePresentation> {
        // Create 20 sample lines with syntax highlighting
        let sample_code = vec![
            // Line 1: Comment
            (1, false, vec![
                StyledSpan::new("// Editor Chrome Demo - main.rs", TextStyle::Comment),
            ]),
            // Line 2: Empty
            (2, false, vec![StyledSpan::raw("")]),
            // Line 3: Use statement
            (3, false, vec![
                StyledSpan::new("use", TextStyle::Keyword),
                StyledSpan::raw(" "),
                StyledSpan::new("std", TextStyle::Module),
                StyledSpan::new("::", TextStyle::Punctuation),
                StyledSpan::new("io", TextStyle::Module),
                StyledSpan::new(";", TextStyle::Punctuation),
            ]),
            // Line 4: Empty
            (4, false, vec![StyledSpan::raw("")]),
            // Line 5: fn main
            (5, true, vec![  // Current line
                StyledSpan::new("fn", TextStyle::Keyword),
                StyledSpan::raw(" "),
                StyledSpan::new("main", TextStyle::Function),
                StyledSpan::new("()", TextStyle::Punctuation),
                StyledSpan::raw(" "),
                StyledSpan::new("{", TextStyle::Punctuation),
            ]),
            // Line 6: let binding
            (6, false, vec![
                StyledSpan::raw("    "),
                StyledSpan::new("let", TextStyle::Keyword),
                StyledSpan::raw(" "),
                StyledSpan::new("message", TextStyle::Variable),
                StyledSpan::raw(" "),
                StyledSpan::new("=", TextStyle::Operator),
                StyledSpan::raw(" "),
                StyledSpan::new("\"Hello, World!\"", TextStyle::String),
                StyledSpan::new(";", TextStyle::Punctuation),
            ]),
            // Line 7: println!
            (7, false, vec![
                StyledSpan::raw("    "),
                StyledSpan::new("println!", TextStyle::Macro),
                StyledSpan::new("(", TextStyle::Punctuation),
                StyledSpan::new("\"{}\", ", TextStyle::String),
                StyledSpan::new("message", TextStyle::Variable),
                StyledSpan::new(")", TextStyle::Punctuation),
                StyledSpan::new(";", TextStyle::Punctuation),
            ]),
            // Line 8: Empty
            (8, false, vec![StyledSpan::raw("")]),
            // Line 9: for loop
            (9, false, vec![
                StyledSpan::raw("    "),
                StyledSpan::new("for", TextStyle::Keyword),
                StyledSpan::raw(" "),
                StyledSpan::new("i", TextStyle::Variable),
                StyledSpan::raw(" "),
                StyledSpan::new("in", TextStyle::Keyword),
                StyledSpan::raw(" "),
                StyledSpan::new("0", TextStyle::Number),
                StyledSpan::new("..", TextStyle::Operator),
                StyledSpan::new("10", TextStyle::Number),
                StyledSpan::raw(" "),
                StyledSpan::new("{", TextStyle::Punctuation),
            ]),
            // Line 10: inner println
            (10, false, vec![
                StyledSpan::raw("        "),
                StyledSpan::new("println!", TextStyle::Macro),
                StyledSpan::new("(", TextStyle::Punctuation),
                StyledSpan::new("\"Count: {}\"", TextStyle::String),
                StyledSpan::new(", ", TextStyle::Punctuation),
                StyledSpan::new("i", TextStyle::Variable),
                StyledSpan::new(")", TextStyle::Punctuation),
                StyledSpan::new(";", TextStyle::Punctuation),
            ]),
            // Line 11: close for
            (11, false, vec![
                StyledSpan::raw("    "),
                StyledSpan::new("}", TextStyle::Punctuation),
            ]),
            // Line 12: Empty
            (12, false, vec![StyledSpan::raw("")]),
            // Line 13: Comment
            (13, false, vec![
                StyledSpan::raw("    "),
                StyledSpan::new("// Process some data", TextStyle::Comment),
            ]),
            // Line 14: let with type
            (14, false, vec![
                StyledSpan::raw("    "),
                StyledSpan::new("let", TextStyle::Keyword),
                StyledSpan::raw(" "),
                StyledSpan::new("numbers", TextStyle::Variable),
                StyledSpan::new(":", TextStyle::Punctuation),
                StyledSpan::raw(" "),
                StyledSpan::new("Vec", TextStyle::Type),
                StyledSpan::new("<", TextStyle::Punctuation),
                StyledSpan::new("i32", TextStyle::Type),
                StyledSpan::new(">", TextStyle::Punctuation),
                StyledSpan::raw(" "),
                StyledSpan::new("=", TextStyle::Operator),
                StyledSpan::raw(" "),
                StyledSpan::new("vec!", TextStyle::Macro),
                StyledSpan::new("[", TextStyle::Punctuation),
                StyledSpan::new("1", TextStyle::Number),
                StyledSpan::new(", ", TextStyle::Punctuation),
                StyledSpan::new("2", TextStyle::Number),
                StyledSpan::new(", ", TextStyle::Punctuation),
                StyledSpan::new("3", TextStyle::Number),
                StyledSpan::new("]", TextStyle::Punctuation),
                StyledSpan::new(";", TextStyle::Punctuation),
            ]),
            // Line 15: Empty
            (15, false, vec![StyledSpan::raw("")]),
            // Line 16: if statement
            (16, false, vec![
                StyledSpan::raw("    "),
                StyledSpan::new("if", TextStyle::Keyword),
                StyledSpan::raw(" "),
                StyledSpan::new("numbers", TextStyle::Variable),
                StyledSpan::new(".", TextStyle::Punctuation),
                StyledSpan::new("len", TextStyle::Function),
                StyledSpan::new("()", TextStyle::Punctuation),
                StyledSpan::raw(" "),
                StyledSpan::new(">", TextStyle::Operator),
                StyledSpan::raw(" "),
                StyledSpan::new("0", TextStyle::Number),
                StyledSpan::raw(" "),
                StyledSpan::new("{", TextStyle::Punctuation),
            ]),
            // Line 17: inner statement
            (17, false, vec![
                StyledSpan::raw("        "),
                StyledSpan::new("println!", TextStyle::Macro),
                StyledSpan::new("(", TextStyle::Punctuation),
                StyledSpan::new("\"Has elements\"", TextStyle::String),
                StyledSpan::new(")", TextStyle::Punctuation),
                StyledSpan::new(";", TextStyle::Punctuation),
            ]),
            // Line 18: close if
            (18, false, vec![
                StyledSpan::raw("    "),
                StyledSpan::new("}", TextStyle::Punctuation),
            ]),
            // Line 19: close main
            (19, false, vec![
                StyledSpan::new("}", TextStyle::Punctuation),
            ]),
            // Line 20: Empty
            (20, false, vec![StyledSpan::raw("")]),
        ];

        sample_code
            .into_iter()
            .map(|(num, is_current, spans)| LinePresentation::with_spans(num, is_current, spans))
            .collect()
    }

    /// Toggle the active tab to the next one.
    fn cycle_tab(&mut self) {
        let tab_count = self.tab_bar.tabs.len();
        if tab_count == 0 {
            return;
        }

        // Clear current active
        for tab in &mut self.tab_bar.tabs {
            tab.is_active = false;
        }

        // Move to next tab
        self.active_tab = (self.active_tab + 1) % tab_count;
        self.tab_bar.tabs[self.active_tab].is_active = true;

        // Update status bar title
        self.status.title = self.tab_bar.tabs[self.active_tab].title.clone();
    }

    /// Toggle the dialog visibility.
    fn toggle_dialog(&mut self) {
        self.dialog = match self.dialog {
            DialogPresentation::None => DialogPresentation::unsaved_changes(
                "Close",
                vec!["*untitled.txt".to_string()],
            ),
            DialogPresentation::UnsavedChangesConfirmation { .. } => DialogPresentation::None,
        };
    }

    /// Toggle sidebar collapsed state.
    fn toggle_sidebar(&mut self) {
        self.sidebar_collapsed = !self.sidebar_collapsed;
    }
}

/// The main editor chrome demo view.
struct EditorChromeDemo {
    /// Model holding the mock editor state.
    state: Model<MockEditorState>,
    /// DialogView needs ViewContext for focus handles, store whether we should render it.
    show_dialog: bool,
}

impl View for EditorChromeDemo {
    fn render(&self, cx: &mut ViewContext) -> AnyElement {
        // Clone state data first to avoid borrow checker issues
        // (Borrow-safe render pattern: complete mutable cx ops before getting theme)
        let (
            tab_bar_data,
            status_data,
            sidebar_data,
            sidebar_collapsed,
            gutter_data,
            visible_lines,
            caret_data,
            dialog_data,
        ) = {
            let state = cx.read_model(&self.state);
            (
                state.tab_bar.clone(),
                state.status.clone(),
                state.sidebar.clone(),
                state.sidebar_collapsed,
                state.gutter.clone(),
                state.visible_lines.clone(),
                state.caret.clone(),
                state.dialog.clone(),
            )
        };

        // Create views from cloned state data
        let tab_bar_view = TabBarView::new(tab_bar_data);
        let status_bar_view = StatusBarView::new(status_data);

        // Create sidebar with collapsed state
        let mut sidebar_view = SidebarView::new(sidebar_data);
        sidebar_view.set_collapsed(sidebar_collapsed);

        // Create gutter view
        let gutter_view = GutterView::new(gutter_data, visible_lines.clone());

        // Create text area view
        let render_model = RenderModel {
            visible_lines,
            caret: caret_data,
            ..Default::default()
        };
        let text_area_view = TextAreaView::new(&render_model);

        // Render all sub-views first (mutable borrows of cx)
        let tab_bar_element = tab_bar_view.render(cx);
        let sidebar_element = sidebar_view.render(cx);
        let gutter_element = gutter_view.render(cx);
        let text_area_element = text_area_view.render(cx);
        let status_bar_element = status_bar_view.render(cx);

        // Now get theme for container styling
        let theme = cx.theme();

        // Build main layout (professional IDE structure):
        // Row (full window)
        // +-- SidebarView (260px or 48px collapsed, left edge)
        // +-- Column (main area, flex: 1)
        //     +-- TabBarView (36px height, inside main area after sidebar)
        //     +-- Row (flex: 1, editor content)
        //     |   +-- GutterView
        //     |   +-- TextAreaView (overflow_hidden for clipping)
        //     +-- StatusBarView (28px height)

        // Editor content: gutter + text area (with overflow clipping)
        let editor_row = Div::new()
            .flex_row()
            .grow(1.0)
            .overflow_hidden() // Clip content to prevent text overlap on resize
            .child(gutter_element)
            .child(text_area_element);

        // Main area column: tab bar + editor + status bar
        let main_area = Div::new()
            .flex_col()
            .grow(1.0)
            .bg(theme.color(ColorToken::BgPrimary))
            .overflow_hidden() // Ensure content stays within bounds
            .child(tab_bar_element)
            .child(editor_row)
            .child(status_bar_element);

        // Root layout: sidebar + main area side by side
        let main_layout = Div::new()
            .flex_row()
            .w(pct(100.0))
            .h(pct(100.0))
            .bg(theme.color(ColorToken::BgPrimary))
            .child(sidebar_element)
            .child(main_area);

        // If dialog is showing, wrap in Stack for overlay
        match &dialog_data {
            DialogPresentation::None => main_layout.into(),
            DialogPresentation::UnsavedChangesConfirmation { .. } => {
                // Create dialog view
                let dialog_view = DialogView::new(dialog_data.clone(), cx);
                let dialog_element = dialog_view.render(cx);

                // Use Stack for z-layering: main layout on bottom, dialog on top
                stack()
                    .w(pct(100.0))
                    .h(pct(100.0))
                    .child(main_layout)
                    .child(dialog_element)
                    .into()
            }
        }
    }
}

fn main() {
    env_logger::init();

    App::new()
        .title("ora - Editor Chrome Demo")
        .size(1200, 800)
        .on_open(|cx| {
            // Create the mock editor state model
            let state = cx.new_model(MockEditorState::new());

            log::info!("Editor Chrome Demo initialized");
            log::info!("Keyboard controls:");
            log::info!("  'T'           - Toggle theme (dark/light)");
            log::info!("  'D'           - Toggle dialog visibility");
            log::info!("  'S'           - Toggle sidebar collapsed/expanded");
            log::info!("  Ctrl+Tab      - Cycle to next tab");
            log::info!("  Ctrl+Shift+Tab- Cycle to previous tab");
            log::info!("");
            log::info!("Note: S and D keys require keyboard events routed to the view.");
            log::info!("      Currently only T (theme toggle) works from event loop.");

            // Create and set the root view
            let view = EditorChromeDemo {
                state: state.clone(),
                show_dialog: false,
            };
            cx.set_root_view(view);

            // Note: For S/D/Tab keyboard shortcuts to work, they need to be handled
            // via the action system or added to event_loop.rs similar to the 'T' key.
            // This is a known limitation documented in STATE.md:
            // "Action and button handlers lack context access"
        })
        .run();
}
