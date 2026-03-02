//! Phase 8 Integration Demo
//!
//! Demonstrates all Phase 8 advanced UI widgets composed via AppLayout:
//!
//! - AppLayout: top-level layout orchestrator with Stack-based overlays
//! - SidebarView: collapsible file explorer with FileTree showing sample hierarchy
//! - TabBarView: horizontal tab bar with dirty indicator
//! - GutterView: line number gutter
//! - TextAreaView: syntax-highlighted code editor content
//! - StatusBarView: bottom status bar
//! - PanelManagerView: resizable bottom panel (Output/Problems/Terminal/Debug)
//! - CommandPaletteView: VS Code-style overlay with fuzzy search (open by default)
//! - Toast notifications: success toast in bottom-right corner
//!
//! Keyboard controls:
//! - 'T': Toggle theme (dark/light)
//! - 'S': Toggle sidebar visibility
//! - 'P': Toggle bottom panel visibility

use ora::{
    AnyElement, App, Model, View, ViewContext,
    AppLayout,
    TabBarView, StatusBarView, SidebarView, GutterView, TextAreaView, DialogView,
    CommandPaletteView, CommandItem, PanelManagerView,
};
use ora::elements::toast::ToastSeverity;
use core_editor::view_model::{
    TabBarPresentation, TabPresentation, StatusPresentation,
    SidebarPresentation, GutterModel, LinePresentation, StyledSpan,
    DialogPresentation, CaretPresentation, VisualPosition, TextStyle,
    RenderModel, FileTreePresentation, FileTreeNode,
};

// ---------------------------------------------------------------------------
// Sample data helpers
// ---------------------------------------------------------------------------

/// Build a sample file tree showing a small Rust project hierarchy.
fn make_file_tree() -> FileTreePresentation {
    FileTreePresentation {
        roots: vec![
            FileTreeNode::dir("my_project", true, vec![
                FileTreeNode::dir("src", true, vec![
                    FileTreeNode::file("main.rs", "rs"),
                    FileTreeNode::file("lib.rs", "rs"),
                    FileTreeNode::dir("views", false, vec![
                        FileTreeNode::file("app_layout.rs", "rs"),
                        FileTreeNode::file("sidebar.rs", "rs"),
                    ]),
                    FileTreeNode::dir("elements", false, vec![
                        FileTreeNode::file("button.rs", "rs"),
                        FileTreeNode::file("input.rs", "rs"),
                    ]),
                ]),
                FileTreeNode::dir("examples", false, vec![
                    FileTreeNode::file("phase8_demo.rs", "rs"),
                    FileTreeNode::file("editor_chrome_demo.rs", "rs"),
                ]),
                FileTreeNode::file("Cargo.toml", "toml"),
                FileTreeNode::file("README.md", "md"),
            ]),
        ],
        selected_index: Some(0),
    }
}

/// Build sample command palette items.
fn make_commands() -> Vec<CommandItem> {
    vec![
        CommandItem::new("file.open", "Open File").with_shortcut("Ctrl+O"),
        CommandItem::new("file.save", "Save").with_shortcut("Ctrl+S"),
        CommandItem::new("file.close", "Close Tab").with_shortcut("Ctrl+W"),
        CommandItem::new("view.palette", "Command Palette").with_shortcut("Ctrl+Shift+P"),
        CommandItem::new("view.sidebar", "Toggle Sidebar").with_shortcut("Ctrl+B"),
        CommandItem::new("view.panel", "Toggle Panel").with_shortcut("Ctrl+J"),
        CommandItem::new("search.find", "Find in Files").with_shortcut("Ctrl+Shift+F"),
        CommandItem::new("term.toggle", "Toggle Terminal").with_shortcut("Ctrl+`"),
        CommandItem::new("settings.open", "Open Settings").with_shortcut("Ctrl+,"),
        CommandItem::new("theme.toggle", "Toggle Theme").with_shortcut("T"),
    ]
}

/// Build sample visible lines with syntax highlighting.
fn make_visible_lines() -> Vec<LinePresentation> {
    let sample_code = vec![
        (1, false, vec![
            StyledSpan::new("// Phase 8 Integration Demo — AppLayout capstone", TextStyle::Comment),
        ]),
        (2, false, vec![StyledSpan::raw("")]),
        (3, false, vec![
            StyledSpan::new("use", TextStyle::Keyword),
            StyledSpan::raw(" "),
            StyledSpan::new("ora", TextStyle::Module),
            StyledSpan::new("::", TextStyle::Punctuation),
            StyledSpan::new("{", TextStyle::Punctuation),
            StyledSpan::new("AppLayout", TextStyle::Type),
            StyledSpan::new(", ", TextStyle::Punctuation),
            StyledSpan::new("CommandPaletteView", TextStyle::Type),
            StyledSpan::new("}", TextStyle::Punctuation),
            StyledSpan::new(";", TextStyle::Punctuation),
        ]),
        (4, false, vec![StyledSpan::raw("")]),
        (5, true, vec![  // current line
            StyledSpan::new("fn", TextStyle::Keyword),
            StyledSpan::raw(" "),
            StyledSpan::new("main", TextStyle::Function),
            StyledSpan::new("()", TextStyle::Punctuation),
            StyledSpan::raw(" "),
            StyledSpan::new("{", TextStyle::Punctuation),
        ]),
        (6, false, vec![
            StyledSpan::raw("    "),
            StyledSpan::new("let", TextStyle::Keyword),
            StyledSpan::raw(" "),
            StyledSpan::new("layout", TextStyle::Variable),
            StyledSpan::raw(" "),
            StyledSpan::new("=", TextStyle::Operator),
            StyledSpan::raw(" "),
            StyledSpan::new("AppLayout", TextStyle::Type),
            StyledSpan::new("::", TextStyle::Punctuation),
            StyledSpan::new("new", TextStyle::Function),
            StyledSpan::new("(", TextStyle::Punctuation),
            StyledSpan::raw("..."),
            StyledSpan::new(")", TextStyle::Punctuation),
            StyledSpan::new(";", TextStyle::Punctuation),
        ]),
        (7, false, vec![
            StyledSpan::raw("    "),
            StyledSpan::new("layout", TextStyle::Variable),
            StyledSpan::new(".", TextStyle::Punctuation),
            StyledSpan::new("show_toast", TextStyle::Function),
            StyledSpan::new("(", TextStyle::Punctuation),
            StyledSpan::new("\"Saved!\"", TextStyle::String),
            StyledSpan::new(", ", TextStyle::Punctuation),
            StyledSpan::new("ToastSeverity", TextStyle::Type),
            StyledSpan::new("::", TextStyle::Punctuation),
            StyledSpan::new("Success", TextStyle::Constant),
            StyledSpan::new(")", TextStyle::Punctuation),
            StyledSpan::new(";", TextStyle::Punctuation),
        ]),
        (8, false, vec![
            StyledSpan::new("}", TextStyle::Punctuation),
        ]),
    ];

    sample_code
        .into_iter()
        .map(|(num, is_current, spans)| LinePresentation::with_spans(num, is_current, spans))
        .collect()
}

// ---------------------------------------------------------------------------
// Mock state
// ---------------------------------------------------------------------------

/// Mock editor state for the Phase 8 demo.
#[derive(Clone)]
struct MockEditorState {
    tab_bar: TabBarPresentation,
    status: StatusPresentation,
    sidebar: SidebarPresentation,
    gutter: GutterModel,
    visible_lines: Vec<LinePresentation>,
    caret: CaretPresentation,
    dialog: DialogPresentation,
    sidebar_visible: bool,
    panel_visible: bool,
}

impl MockEditorState {
    fn new() -> Self {
        Self {
            tab_bar: TabBarPresentation {
                tabs: vec![
                    TabPresentation::new(1, "main.rs".to_string(), true, false),
                    TabPresentation::new(2, "*app_layout.rs".to_string(), false, true),
                    TabPresentation::new(3, "lib.rs".to_string(), false, false),
                ],
                visible: true,
            },
            status: StatusPresentation {
                title: "main.rs".to_string(),
                dirty: false,
                cursor_line: 5,
                cursor_column: 1,
                language: "Rust".to_string(),
                total_lines: 8,
                message: Some("Phase 8 complete".to_string()),
            },
            sidebar: SidebarPresentation {
                visible: true,
                focused: false,
                directory_name: "my_project".to_string(),
                entries: vec![],
                width: 480,
            },
            gutter: GutterModel::new(true, 8),
            visible_lines: make_visible_lines(),
            caret: CaretPresentation {
                position: VisualPosition::new(4, 0),
                visible: true,
            },
            dialog: DialogPresentation::None,
            sidebar_visible: true,
            panel_visible: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Phase 8 demo view
// ---------------------------------------------------------------------------

/// Top-level demo view composing all Phase 8 components via AppLayout.
struct Phase8Demo {
    state: Model<MockEditorState>,
}

impl View for Phase8Demo {
    fn render(&self, cx: &mut ViewContext) -> AnyElement {
        // --- Clone state data (borrow-safe pattern) ---
        let (
            tab_bar_data,
            status_data,
            sidebar_data,
            gutter_data,
            visible_lines,
            caret_data,
            dialog_data,
            sidebar_visible,
            panel_visible,
        ) = {
            let s = cx.read_model(&self.state);
            (
                s.tab_bar.clone(),
                s.status.clone(),
                s.sidebar.clone(),
                s.gutter.clone(),
                s.visible_lines.clone(),
                s.caret.clone(),
                s.dialog.clone(),
                s.sidebar_visible,
                s.panel_visible,
            )
        };

        // --- Build Phase 7 views ---
        let tab_bar = TabBarView::new(tab_bar_data);
        let status_bar = StatusBarView::new(status_data);
        let sidebar = SidebarView::new(sidebar_data, make_file_tree());
        let gutter = GutterView::new(gutter_data, visible_lines.clone());
        let render_model = RenderModel {
            visible_lines,
            caret: caret_data,
            ..Default::default()
        };
        let text_area = TextAreaView::new(&render_model);

        // DialogView::new() requires &mut ViewContext for focus handles
        let dialog = DialogView::new(dialog_data, cx);

        // --- Build Phase 8 views ---
        // CommandPaletteView needs a FocusHandle from context
        let palette_focus = cx.focus_handle();
        let mut palette = CommandPaletteView::new(make_commands(), palette_focus);
        // Open the palette with a sample query to demonstrate fuzzy filtering
        palette.open();
        palette.set_query("open".to_string());

        let panel = PanelManagerView::new();

        // --- Compose into AppLayout ---
        let mut layout = AppLayout::new(
            sidebar,
            tab_bar,
            gutter,
            text_area,
            status_bar,
            panel,
            palette,
            dialog,
        );

        // Apply visibility state from mock editor
        layout.sidebar_visible = sidebar_visible;
        layout.panel_visible = panel_visible;

        // Add a sample toast notification (Success auto-dismisses after 4s)
        layout.show_toast("Phase 8 complete — all widgets integrated", ToastSeverity::Success);

        layout.render(cx)
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    env_logger::init();

    App::new()
        .title("ora - Phase 8 Integration Demo")
        .size(1400, 900)
        .on_open(|cx| {
            let state = cx.new_model(MockEditorState::new());

            log::info!("Phase 8 Integration Demo initialized");
            log::info!("Demonstrates: AppLayout + all Phase 8 components");
            log::info!("");
            log::info!("Layout regions visible:");
            log::info!("  - Sidebar (left) with FileTree showing project hierarchy");
            log::info!("  - Tab bar (3 tabs, one dirty)");
            log::info!("  - Editor area (gutter + syntax-highlighted code)");
            log::info!("  - Panel manager (bottom, Output tab active)");
            log::info!("  - Status bar (line 5, col 1, Rust language)");
            log::info!("");
            log::info!("Overlays:");
            log::info!("  - CommandPalette (open, query='open', fuzzy-filtered results)");
            log::info!("  - Toast notification (bottom-right, Success severity)");
            log::info!("");
            log::info!("Keyboard: T = toggle theme");

            cx.set_root_view(Phase8Demo { state });
        })
        .run();
}
