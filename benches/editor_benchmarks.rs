//! End-to-End Editor Performance Benchmarks
//!
//! Measures complete editing operations as they would execute in the real editor:
//! - Keypress latency (edit mode)
//! - Keypress latency (navigation mode)
//! - Typing throughput
//! - Large file behavior

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use mudatexteditor::commands::editor_command::{Direction, MoveScope};
use mudatexteditor::commands::{CommandHistory, EditorCommand};
use mudatexteditor::commands::dispatcher::{CommandContext, CommandDispatcher};
use mudatexteditor::domain::Document;
use mudatexteditor::events::EventBus;
use mudatexteditor::view::{EditorView, FocusState, Sidebar};
use mudatexteditor::view_model::builder::ViewModelBuilder;
use std::path::PathBuf;

mod test_data;
use test_data::{generate_rust_source, generate_typing_input, TestSizes};

/// Complete keypress cycle for character insertion.
/// This measures what happens on every keystroke in edit mode.
fn bench_keypress_edit(c: &mut Criterion) {
    let mut group = c.benchmark_group("editor/keypress_edit");
    group.sample_size(50);

    for size in [TestSizes::SMALL, TestSizes::MEDIUM, TestSizes::LARGE] {
        let size_name = match size {
            TestSizes::SMALL => "10KB",
            TestSizes::MEDIUM => "100KB",
            TestSizes::LARGE => "1MB",
            _ => "unknown",
        };

        let content = generate_rust_source(size);

        group.bench_with_input(
            BenchmarkId::new("insert_char", size_name),
            &content,
            |b, content| {
                b.iter_batched(
                    || {
                        // Setup: create document, view, dispatcher
                        let doc = Document::from_str(content, Some(PathBuf::from("test.rs")));
                        let mut view = EditorView::new(doc.id());
                        view.viewport.resize(120, 40);
                        // Position cursor in middle
                        let middle = doc.len_chars() / 2;
                        view.move_caret_to(middle, false);
                        view.viewport.scroll_y = doc.len_lines() / 2 - 20;

                        let history = CommandHistory::new();
                        let event_bus = EventBus::new();
                        let dispatcher = CommandDispatcher::new();
                        let sidebar = Sidebar::default();

                        (doc, view, history, event_bus, dispatcher, sidebar)
                    },
                    |(mut doc, mut view, mut history, mut event_bus, mut dispatcher, sidebar)| {
                        // 1. Dispatch the insert command
                        let mut ctx = CommandContext {
                            view: &mut view,
                            document: &mut doc,
                            history: &mut history,
                            event_bus: &mut event_bus,
                        };
                        dispatcher.dispatch(EditorCommand::InsertChar('x'), &mut ctx);

                        // 2. Build render model (happens every frame)
                        let model = ViewModelBuilder::build(
                            &doc,
                            &view,
                            None,
                            None,
                            &sidebar,
                            FocusState::Editor,
                            40,
                            None,
                        );

                        black_box(model)
                    },
                    criterion::BatchSize::SmallInput,
                )
            },
        );
    }

    group.finish();
}

/// Complete keypress cycle for cursor navigation.
fn bench_keypress_navigation(c: &mut Criterion) {
    let mut group = c.benchmark_group("editor/keypress_navigation");
    group.sample_size(50);

    for size in [TestSizes::SMALL, TestSizes::MEDIUM, TestSizes::LARGE] {
        let size_name = match size {
            TestSizes::SMALL => "10KB",
            TestSizes::MEDIUM => "100KB",
            TestSizes::LARGE => "1MB",
            _ => "unknown",
        };

        let content = generate_rust_source(size);

        // Arrow right
        group.bench_with_input(
            BenchmarkId::new("arrow_right", size_name),
            &content,
            |b, content| {
                b.iter_batched(
                    || {
                        let doc = Document::from_str(content, Some(PathBuf::from("test.rs")));
                        let mut view = EditorView::new(doc.id());
                        view.viewport.resize(120, 40);
                        let middle = doc.len_chars() / 2;
                        view.move_caret_to(middle, false);
                        view.viewport.scroll_y = doc.len_lines() / 2 - 20;

                        let history = CommandHistory::new();
                        let event_bus = EventBus::new();
                        let dispatcher = CommandDispatcher::new();
                        let sidebar = Sidebar::default();

                        (doc, view, history, event_bus, dispatcher, sidebar)
                    },
                    |(mut doc, mut view, mut history, mut event_bus, mut dispatcher, sidebar)| {
                        let mut ctx = CommandContext {
                            view: &mut view,
                            document: &mut doc,
                            history: &mut history,
                            event_bus: &mut event_bus,
                        };

                        let cmd = EditorCommand::MoveCursor {
                            direction: Direction::Right,
                            scope: MoveScope::Char,
                            extend_selection: false,
                        };
                        dispatcher.dispatch(cmd, &mut ctx);

                        let model = ViewModelBuilder::build(
                            &doc,
                            &view,
                            None,
                            None,
                            &sidebar,
                            FocusState::Editor,
                            40,
                            None,
                        );

                        black_box(model)
                    },
                    criterion::BatchSize::SmallInput,
                )
            },
        );

        // Arrow down (triggers line change)
        group.bench_with_input(
            BenchmarkId::new("arrow_down", size_name),
            &content,
            |b, content| {
                b.iter_batched(
                    || {
                        let doc = Document::from_str(content, Some(PathBuf::from("test.rs")));
                        let mut view = EditorView::new(doc.id());
                        view.viewport.resize(120, 40);
                        let middle = doc.len_chars() / 2;
                        view.move_caret_to(middle, false);
                        view.viewport.scroll_y = doc.len_lines() / 2 - 20;

                        let history = CommandHistory::new();
                        let event_bus = EventBus::new();
                        let dispatcher = CommandDispatcher::new();
                        let sidebar = Sidebar::default();

                        (doc, view, history, event_bus, dispatcher, sidebar)
                    },
                    |(mut doc, mut view, mut history, mut event_bus, mut dispatcher, sidebar)| {
                        let mut ctx = CommandContext {
                            view: &mut view,
                            document: &mut doc,
                            history: &mut history,
                            event_bus: &mut event_bus,
                        };

                        let cmd = EditorCommand::MoveCursor {
                            direction: Direction::Down,
                            scope: MoveScope::Char,
                            extend_selection: false,
                        };
                        dispatcher.dispatch(cmd, &mut ctx);

                        let model = ViewModelBuilder::build(
                            &doc,
                            &view,
                            None,
                            None,
                            &sidebar,
                            FocusState::Editor,
                            40,
                            None,
                        );

                        black_box(model)
                    },
                    criterion::BatchSize::SmallInput,
                )
            },
        );

        // Page down (triggers viewport change)
        group.bench_with_input(
            BenchmarkId::new("page_down", size_name),
            &content,
            |b, content| {
                b.iter_batched(
                    || {
                        let doc = Document::from_str(content, Some(PathBuf::from("test.rs")));
                        let mut view = EditorView::new(doc.id());
                        view.viewport.resize(120, 40);

                        let history = CommandHistory::new();
                        let event_bus = EventBus::new();
                        let dispatcher = CommandDispatcher::new();
                        let sidebar = Sidebar::default();

                        (doc, view, history, event_bus, dispatcher, sidebar)
                    },
                    |(mut doc, mut view, mut history, mut event_bus, mut dispatcher, sidebar)| {
                        let mut ctx = CommandContext {
                            view: &mut view,
                            document: &mut doc,
                            history: &mut history,
                            event_bus: &mut event_bus,
                        };

                        let cmd = EditorCommand::MoveCursor {
                            direction: Direction::Down,
                            scope: MoveScope::Page,
                            extend_selection: false,
                        };
                        dispatcher.dispatch(cmd, &mut ctx);

                        let model = ViewModelBuilder::build(
                            &doc,
                            &view,
                            None,
                            None,
                            &sidebar,
                            FocusState::Editor,
                            40,
                            None,
                        );

                        black_box(model)
                    },
                    criterion::BatchSize::SmallInput,
                )
            },
        );
    }

    group.finish();
}

/// Typing throughput: measure inserting many characters in sequence.
fn bench_typing_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("editor/typing_throughput");
    group.sample_size(20);

    for char_count in [100, 500, 1000] {
        let input_chars = generate_typing_input(char_count);

        group.throughput(Throughput::Elements(char_count as u64));
        group.bench_with_input(
            BenchmarkId::new("chars", char_count),
            &input_chars,
            |b, chars| {
                b.iter_batched(
                    || {
                        let doc = Document::from_str("", Some(PathBuf::from("test.rs")));
                        let mut view = EditorView::new(doc.id());
                        view.viewport.resize(120, 40);

                        let history = CommandHistory::new();
                        let event_bus = EventBus::new();
                        let dispatcher = CommandDispatcher::new();
                        let sidebar = Sidebar::default();

                        (doc, view, history, event_bus, dispatcher, sidebar, chars.clone())
                    },
                    |(mut doc, mut view, mut history, mut event_bus, mut dispatcher, sidebar, chars)| {
                        for ch in chars {
                            let mut ctx = CommandContext {
                                view: &mut view,
                                document: &mut doc,
                                history: &mut history,
                                event_bus: &mut event_bus,
                            };
                            dispatcher.dispatch(EditorCommand::InsertChar(ch), &mut ctx);
                        }

                        // Final render
                        let model = ViewModelBuilder::build(
                            &doc,
                            &view,
                            None,
                            None,
                            &sidebar,
                            FocusState::Editor,
                            40,
                            None,
                        );

                        black_box(model)
                    },
                    criterion::BatchSize::SmallInput,
                )
            },
        );
    }

    group.finish();
}

/// Typing throughput with per-keystroke rendering (realistic scenario).
fn bench_typing_with_render(c: &mut Criterion) {
    let mut group = c.benchmark_group("editor/typing_with_render");
    group.sample_size(10);

    for char_count in [50, 100] {
        let input_chars = generate_typing_input(char_count);

        group.throughput(Throughput::Elements(char_count as u64));
        group.bench_with_input(
            BenchmarkId::new("chars_per_frame", char_count),
            &input_chars,
            |b, chars| {
                b.iter_batched(
                    || {
                        let doc = Document::from_str("", Some(PathBuf::from("test.rs")));
                        let mut view = EditorView::new(doc.id());
                        view.viewport.resize(120, 40);

                        let history = CommandHistory::new();
                        let event_bus = EventBus::new();
                        let dispatcher = CommandDispatcher::new();
                        let sidebar = Sidebar::default();

                        (doc, view, history, event_bus, dispatcher, sidebar, chars.clone())
                    },
                    |(mut doc, mut view, mut history, mut event_bus, mut dispatcher, sidebar, chars)| {
                        for ch in chars {
                            // Insert
                            {
                                let mut ctx = CommandContext {
                                    view: &mut view,
                                    document: &mut doc,
                                    history: &mut history,
                                    event_bus: &mut event_bus,
                                };
                                dispatcher.dispatch(EditorCommand::InsertChar(ch), &mut ctx);
                            }

                            // Render after each keystroke (realistic)
                            black_box(ViewModelBuilder::build(
                                &doc,
                                &view,
                                None,
                                None,
                                &sidebar,
                                FocusState::Editor,
                                40,
                                None,
                            ));
                        }
                    },
                    criterion::BatchSize::SmallInput,
                )
            },
        );
    }

    group.finish();
}

/// Undo/Redo performance.
fn bench_undo_redo(c: &mut Criterion) {
    let mut group = c.benchmark_group("editor/undo_redo");

    for size in [TestSizes::SMALL, TestSizes::MEDIUM] {
        let size_name = match size {
            TestSizes::SMALL => "10KB",
            TestSizes::MEDIUM => "100KB",
            _ => "unknown",
        };

        let content = generate_rust_source(size);

        // Make 10 edits, then undo all
        group.bench_with_input(
            BenchmarkId::new("undo_10", size_name),
            &content,
            |b, content| {
                b.iter_batched(
                    || {
                        let mut doc = Document::from_str(content, Some(PathBuf::from("test.rs")));
                        let mut view = EditorView::new(doc.id());
                        view.viewport.resize(120, 40);

                        let mut history = CommandHistory::new();
                        let mut event_bus = EventBus::new();
                        let mut dispatcher = CommandDispatcher::new();

                        // Make 10 edits
                        for i in 0..10 {
                            let mut ctx = CommandContext {
                                view: &mut view,
                                document: &mut doc,
                                history: &mut history,
                                event_bus: &mut event_bus,
                            };
                            dispatcher.dispatch(EditorCommand::InsertChar((b'a' + (i % 26)) as char), &mut ctx);
                        }

                        let sidebar = Sidebar::default();
                        (doc, view, history, event_bus, dispatcher, sidebar)
                    },
                    |(mut doc, mut view, mut history, mut event_bus, mut dispatcher, sidebar)| {
                        // Undo all 10 edits
                        for _ in 0..10 {
                            let mut ctx = CommandContext {
                                view: &mut view,
                                document: &mut doc,
                                history: &mut history,
                                event_bus: &mut event_bus,
                            };
                            dispatcher.dispatch(EditorCommand::Undo, &mut ctx);
                        }

                        let model = ViewModelBuilder::build(
                            &doc,
                            &view,
                            None,
                            None,
                            &sidebar,
                            FocusState::Editor,
                            40,
                            None,
                        );

                        black_box(model)
                    },
                    criterion::BatchSize::SmallInput,
                )
            },
        );
    }

    group.finish();
}

/// Selection operations.
fn bench_selection_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("editor/selection");

    let content = generate_rust_source(TestSizes::MEDIUM);

    // Select all
    group.bench_function("select_all", |b| {
        b.iter_batched(
            || {
                let doc = Document::from_str(&content, Some(PathBuf::from("test.rs")));
                let mut view = EditorView::new(doc.id());
                view.viewport.resize(120, 40);

                let history = CommandHistory::new();
                let event_bus = EventBus::new();
                let dispatcher = CommandDispatcher::new();
                let sidebar = Sidebar::default();

                (doc, view, history, event_bus, dispatcher, sidebar)
            },
            |(mut doc, mut view, mut history, mut event_bus, mut dispatcher, sidebar)| {
                let mut ctx = CommandContext {
                    view: &mut view,
                    document: &mut doc,
                    history: &mut history,
                    event_bus: &mut event_bus,
                };
                dispatcher.dispatch(EditorCommand::SelectAll, &mut ctx);

                let model = ViewModelBuilder::build(
                    &doc,
                    &view,
                    None,
                    None,
                    &sidebar,
                    FocusState::Editor,
                    40,
                    None,
                );

                black_box(model)
            },
            criterion::BatchSize::SmallInput,
        )
    });

    // Delete selection
    group.bench_function("delete_selection", |b| {
        b.iter_batched(
            || {
                let doc = Document::from_str(&content, Some(PathBuf::from("test.rs")));
                let mut view = EditorView::new(doc.id());
                view.viewport.resize(120, 40);
                // Select 1000 characters
                view.begin_selection();
                view.move_caret_to(1000.min(doc.len_chars()), true);

                let history = CommandHistory::new();
                let event_bus = EventBus::new();
                let dispatcher = CommandDispatcher::new();
                let sidebar = Sidebar::default();

                (doc, view, history, event_bus, dispatcher, sidebar)
            },
            |(mut doc, mut view, mut history, mut event_bus, mut dispatcher, sidebar)| {
                let mut ctx = CommandContext {
                    view: &mut view,
                    document: &mut doc,
                    history: &mut history,
                    event_bus: &mut event_bus,
                };
                dispatcher.dispatch(EditorCommand::DeleteSelection, &mut ctx);

                let model = ViewModelBuilder::build(
                    &doc,
                    &view,
                    None,
                    None,
                    &sidebar,
                    FocusState::Editor,
                    40,
                    None,
                );

                black_box(model)
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.finish();
}

/// Large file stress test.
fn bench_large_file_stress(c: &mut Criterion) {
    let mut group = c.benchmark_group("editor/large_file");
    group.sample_size(10);

    // 1MB file operations
    let content = generate_rust_source(TestSizes::LARGE);

    group.bench_function("open_1MB", |b| {
        b.iter(|| {
            let doc = Document::from_str(&content, Some(PathBuf::from("test.rs")));
            black_box(doc)
        })
    });

    // First keypress on 1MB file
    group.bench_with_input(
        BenchmarkId::new("first_keypress", "1MB"),
        &content,
        |b, content| {
            b.iter_batched(
                || {
                    let doc = Document::from_str(content, Some(PathBuf::from("test.rs")));
                    let mut view = EditorView::new(doc.id());
                    view.viewport.resize(120, 40);

                    let history = CommandHistory::new();
                    let event_bus = EventBus::new();
                    let dispatcher = CommandDispatcher::new();
                    let sidebar = Sidebar::default();
                    (doc, view, history, event_bus, dispatcher, sidebar)
                },
                |(mut doc, mut view, mut history, mut event_bus, mut dispatcher, sidebar)| {
                    let mut ctx = CommandContext {
                        view: &mut view,
                        document: &mut doc,
                        history: &mut history,
                        event_bus: &mut event_bus,
                    };
                    dispatcher.dispatch(EditorCommand::InsertChar('x'), &mut ctx);

                    let model = ViewModelBuilder::build(
                        &doc,
                        &view,
                        None,
                        None,
                        &sidebar,
                        FocusState::Editor,
                        40,
                        None,
                    );

                    black_box(model)
                },
                criterion::BatchSize::LargeInput, // Use LargeInput for expensive setup
            )
        },
    );

    group.finish();
}

criterion_group!(
    benches,
    bench_keypress_edit,
    bench_keypress_navigation,
    bench_typing_throughput,
    bench_typing_with_render,
    bench_undo_redo,
    bench_selection_operations,
    bench_large_file_stress,
);

criterion_main!(benches);
