//! Rendering Performance Benchmarks
//!
//! Measures the view model building and rendering pipeline:
//! - RenderModel construction
//! - Line presentation building
//! - Styled span generation

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use core_editor::domain::Document;
use core_editor::view::{EditorView, FocusState, Sidebar};
use core_editor::view_model::builder::ViewModelBuilder;
use ropey::Rope;
use std::path::PathBuf;

mod test_data;
use test_data::{generate_rust_source, TestSizes};

/// Benchmark building the complete render model (happens every frame).
fn bench_build_render_model(c: &mut Criterion) {
    let mut group = c.benchmark_group("render/build_model");
    group.sample_size(50);

    for size in [TestSizes::SMALL, TestSizes::MEDIUM, TestSizes::LARGE] {
        let size_name = match size {
            TestSizes::SMALL => "10KB",
            TestSizes::MEDIUM => "100KB",
            TestSizes::LARGE => "1MB",
            _ => "unknown",
        };

        let content = generate_rust_source(size);
        let doc = Document::from_str(&content, Some(PathBuf::from("test.rs")));
        let mut view = EditorView::new(doc.id());
        view.viewport.resize(120, 40); // Typical terminal size
        let sidebar = Sidebar::default();

        group.bench_with_input(
            BenchmarkId::new("full_model", size_name),
            &(&doc, &view, &sidebar),
            |b, (doc, view, sidebar)| {
                b.iter(|| {
                    black_box(ViewModelBuilder::build(
                        doc,
                        view,
                        None,
                        None,
                        sidebar,
                        FocusState::Editor,
                        40,
                        None,
                    ))
                })
            },
        );
    }

    group.finish();
}

/// Benchmark render model with cursor at different positions.
fn bench_cursor_positions(c: &mut Criterion) {
    let mut group = c.benchmark_group("render/cursor_position");

    let content = generate_rust_source(TestSizes::MEDIUM);
    let doc = Document::from_str(&content, Some(PathBuf::from("test.rs")));
    let sidebar = Sidebar::default();

    // Cursor at beginning
    let mut view_start = EditorView::new(doc.id());
    view_start.viewport.resize(120, 40);

    group.bench_function("beginning", |b| {
        b.iter(|| {
            black_box(ViewModelBuilder::build(
                &doc,
                &view_start,
                None,
                None,
                &sidebar,
                FocusState::Editor,
                40,
                None,
            ))
        })
    });

    // Cursor at middle
    let mut view_middle = EditorView::new(doc.id());
    view_middle.viewport.resize(120, 40);
    let middle_offset = doc.len_chars() / 2;
    view_middle.move_caret_to(middle_offset, false);
    view_middle.viewport.scroll_y = (doc.len_lines() / 2).saturating_sub(20);

    group.bench_function("middle", |b| {
        b.iter(|| {
            black_box(ViewModelBuilder::build(
                &doc,
                &view_middle,
                None,
                None,
                &sidebar,
                FocusState::Editor,
                40,
                None,
            ))
        })
    });

    // Cursor at end
    let mut view_end = EditorView::new(doc.id());
    view_end.viewport.resize(120, 40);
    view_end.move_caret_to(doc.len_chars(), false);
    view_end.viewport.scroll_y = doc.len_lines().saturating_sub(40);

    group.bench_function("end", |b| {
        b.iter(|| {
            black_box(ViewModelBuilder::build(
                &doc,
                &view_end,
                None,
                None,
                &sidebar,
                FocusState::Editor,
                40,
                None,
            ))
        })
    });

    group.finish();
}

/// Benchmark render model with selection.
fn bench_with_selection(c: &mut Criterion) {
    let mut group = c.benchmark_group("render/selection");

    let content = generate_rust_source(TestSizes::MEDIUM);
    let doc = Document::from_str(&content, Some(PathBuf::from("test.rs")));
    let sidebar = Sidebar::default();

    // No selection
    let mut view_no_sel = EditorView::new(doc.id());
    view_no_sel.viewport.resize(120, 40);

    group.bench_function("no_selection", |b| {
        b.iter(|| {
            black_box(ViewModelBuilder::build(
                &doc,
                &view_no_sel,
                None,
                None,
                &sidebar,
                FocusState::Editor,
                40,
                None,
            ))
        })
    });

    // Small selection (10 chars)
    let mut view_small_sel = EditorView::new(doc.id());
    view_small_sel.viewport.resize(120, 40);
    view_small_sel.begin_selection();
    view_small_sel.move_caret_to(10, true);

    group.bench_function("small_selection_10chars", |b| {
        b.iter(|| {
            black_box(ViewModelBuilder::build(
                &doc,
                &view_small_sel,
                None,
                None,
                &sidebar,
                FocusState::Editor,
                40,
                None,
            ))
        })
    });

    // Large selection (1000 chars spanning multiple lines)
    let mut view_large_sel = EditorView::new(doc.id());
    view_large_sel.viewport.resize(120, 40);
    view_large_sel.begin_selection();
    view_large_sel.move_caret_to(1000.min(doc.len_chars()), true);

    group.bench_function("large_selection_1000chars", |b| {
        b.iter(|| {
            black_box(ViewModelBuilder::build(
                &doc,
                &view_large_sel,
                None,
                None,
                &sidebar,
                FocusState::Editor,
                40,
                None,
            ))
        })
    });

    // Select all
    let mut view_select_all = EditorView::new(doc.id());
    view_select_all.viewport.resize(120, 40);
    view_select_all.select_all(doc.len_chars());

    group.bench_function("select_all", |b| {
        b.iter(|| {
            black_box(ViewModelBuilder::build(
                &doc,
                &view_select_all,
                None,
                None,
                &sidebar,
                FocusState::Editor,
                40,
                None,
            ))
        })
    });

    group.finish();
}

/// Benchmark different viewport sizes.
fn bench_viewport_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("render/viewport_size");

    let content = generate_rust_source(TestSizes::MEDIUM);
    let doc = Document::from_str(&content, Some(PathBuf::from("test.rs")));
    let sidebar = Sidebar::default();

    for (width, height) in [(80, 24), (120, 40), (200, 60), (300, 100)] {
        let mut view = EditorView::new(doc.id());
        view.viewport.resize(width, height);

        group.throughput(Throughput::Elements(height as u64));
        group.bench_function(BenchmarkId::new("wxh", format!("{}x{}", width, height)), |b| {
            b.iter(|| {
                black_box(ViewModelBuilder::build(
                    &doc,
                    &view,
                    None,
                    None,
                    &sidebar,
                    FocusState::Editor,
                    height,
                    None,
                ))
            })
        });
    }

    group.finish();
}

/// Benchmark sidebar visibility impact.
fn bench_sidebar_toggle(c: &mut Criterion) {
    let mut group = c.benchmark_group("render/sidebar");

    let content = generate_rust_source(TestSizes::MEDIUM);
    let doc = Document::from_str(&content, Some(PathBuf::from("test.rs")));
    let mut view = EditorView::new(doc.id());
    view.viewport.resize(120, 40);

    // Sidebar hidden
    let sidebar_hidden = Sidebar::default();

    group.bench_function("hidden", |b| {
        b.iter(|| {
            black_box(ViewModelBuilder::build(
                &doc,
                &view,
                None,
                None,
                &sidebar_hidden,
                FocusState::Editor,
                40,
                None,
            ))
        })
    });

    // Sidebar visible
    let mut sidebar_visible = Sidebar::default();
    sidebar_visible.visible = true;

    group.bench_function("visible", |b| {
        b.iter(|| {
            black_box(ViewModelBuilder::build(
                &doc,
                &view,
                None,
                None,
                &sidebar_visible,
                FocusState::Editor,
                40,
                None,
            ))
        })
    });

    // Sidebar focused
    group.bench_function("focused", |b| {
        b.iter(|| {
            black_box(ViewModelBuilder::build(
                &doc,
                &view,
                None,
                None,
                &sidebar_visible,
                FocusState::Sidebar,
                40,
                None,
            ))
        })
    });

    group.finish();
}

/// Benchmark document content access patterns.
fn bench_content_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("render/content_access");

    for size in [TestSizes::SMALL, TestSizes::MEDIUM, TestSizes::LARGE] {
        let size_name = match size {
            TestSizes::SMALL => "10KB",
            TestSizes::MEDIUM => "100KB",
            TestSizes::LARGE => "1MB",
            _ => "unknown",
        };

        let content = generate_rust_source(size);
        let doc = Document::from_str(&content, Some(PathBuf::from("test.rs")));

        // Full content clone (current implementation)
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(
            BenchmarkId::new("full_clone", size_name),
            &doc,
            |b, doc| {
                b.iter(|| {
                    black_box(doc.content())
                })
            },
        );

        // Getting individual lines (what we should do instead)
        group.bench_with_input(
            BenchmarkId::new("40_lines_only", size_name),
            &doc,
            |b, doc| {
                let start_line = doc.len_lines() / 2 - 20;
                b.iter(|| {
                    for i in 0..40 {
                        let line_idx = start_line + i;
                        if line_idx < doc.len_lines() {
                            black_box(doc.line(line_idx));
                        }
                    }
                })
            },
        );
    }

    group.finish();
}

/// Benchmark pure navigation + render (no document creation overhead).
fn bench_navigation_render_cycle(c: &mut Criterion) {
    let mut group = c.benchmark_group("render/navigation_cycle");
    group.sample_size(100);

    for size in [TestSizes::SMALL, TestSizes::MEDIUM, TestSizes::LARGE] {
        let size_name = match size {
            TestSizes::SMALL => "10KB",
            TestSizes::MEDIUM => "100KB",
            TestSizes::LARGE => "1MB",
            _ => "unknown",
        };

        let content = generate_rust_source(size);
        // Create document ONCE, outside the benchmark
        let doc = Document::from_str(&content, Some(PathBuf::from("test.rs")));
        let mut view = EditorView::new(doc.id());
        view.viewport.resize(120, 40);
        // Position in middle
        let middle = doc.len_chars() / 2;
        view.move_caret_to(middle, false);
        view.viewport.scroll_y = doc.len_lines() / 2 - 20;
        let sidebar = Sidebar::default();

        group.bench_with_input(
            BenchmarkId::new("arrow_right_render", size_name),
            &(&doc, &view, &sidebar),
            |b, (doc, view, sidebar)| {
                let mut view_clone = (*view).clone();
                b.iter(|| {
                    // Simulate arrow right
                    let offset = view_clone.caret_offset();
                    if offset < doc.len_chars() {
                        view_clone.move_caret_to(offset + 1, false);
                    }

                    // Build render model
                    black_box(ViewModelBuilder::build(
                        doc,
                        &view_clone,
                        None,
                        None,
                        sidebar,
                        FocusState::Editor,
                        40,
                        None,
                    ))
                })
            },
        );
    }

    group.finish();
}

/// Benchmark edit + render cycle (with incremental parsing).
fn bench_edit_render_cycle(c: &mut Criterion) {
    let mut group = c.benchmark_group("render/edit_cycle");
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
            BenchmarkId::new("insert_char_render", size_name),
            &content,
            |b, content| {
                b.iter_batched(
                    || {
                        // Create document for each batch (we need fresh state)
                        let mut doc = Document::from_str(content, Some(PathBuf::from("test.rs")));
                        let mut view = EditorView::new(doc.id());
                        view.viewport.resize(120, 40);
                        // Position in middle
                        let middle = doc.len_chars() / 2;
                        view.move_caret_to(middle, false);
                        view.viewport.scroll_y = doc.len_lines() / 2 - 20;

                        // Pre-warm the cache
                        doc.refresh_content_cache();

                        let sidebar = Sidebar::default();
                        (doc, view, sidebar)
                    },
                    |(mut doc, mut view, sidebar)| {
                        // Insert character
                        let offset = view.caret_offset();
                        doc.buffer_mut().insert_char(offset, 'x');
                        view.move_caret_to(offset + 1, false);

                        // Incremental syntax update
                        doc.update_syntax_incremental(offset, 0, 1);

                        // Build render model
                        black_box(ViewModelBuilder::build(
                            &doc,
                            &view,
                            None,
                            None,
                            &sidebar,
                            FocusState::Editor,
                            40,
                            None,
                        ))
                    },
                    criterion::BatchSize::LargeInput,
                )
            },
        );
    }

    group.finish();
}

/// Benchmark just the incremental syntax update (isolate parsing cost).
fn bench_incremental_syntax(c: &mut Criterion) {
    let mut group = c.benchmark_group("render/incremental_syntax");
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
            BenchmarkId::new("update_syntax_incremental", size_name),
            &content,
            |b, content| {
                b.iter_batched(
                    || {
                        let mut doc = Document::from_str(content, Some(PathBuf::from("test.rs")));
                        doc.refresh_content_cache();
                        let middle = doc.len_chars() / 2;
                        (doc, middle)
                    },
                    |(mut doc, offset)| {
                        // Just buffer insert + incremental syntax, no render
                        doc.buffer_mut().insert_char(offset, 'x');
                        doc.update_syntax_incremental(offset, 0, 1);
                        black_box(doc.revision())
                    },
                    // Use PerIteration to measure single incremental parse, not accumulated
                    criterion::BatchSize::PerIteration,
                )
            },
        );
    }

    group.finish();
}

/// Benchmark just buffer operations (isolate rope cost).
fn bench_buffer_only(c: &mut Criterion) {
    let mut group = c.benchmark_group("render/buffer_only");
    group.sample_size(100);

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
                        let doc = Document::from_str(content, Some(PathBuf::from("test.rs")));
                        let middle = doc.len_chars() / 2;
                        (doc, middle)
                    },
                    |(mut doc, offset)| {
                        // Just buffer insert, no syntax or render
                        doc.buffer_mut().insert_char(offset, 'x');
                        black_box(doc.revision())
                    },
                    // Use PerIteration to measure single inserts, not accumulated operations
                    criterion::BatchSize::PerIteration,
                )
            },
        );
    }

    group.finish();
}

/// Benchmark raw ropey operations without Document overhead.
fn bench_raw_rope(c: &mut Criterion) {
    let mut group = c.benchmark_group("render/raw_rope");
    group.sample_size(100);

    for size in [TestSizes::SMALL, TestSizes::MEDIUM, TestSizes::LARGE] {
        let size_name = match size {
            TestSizes::SMALL => "10KB",
            TestSizes::MEDIUM => "100KB",
            TestSizes::LARGE => "1MB",
            _ => "unknown",
        };

        let content = generate_rust_source(size);
        // Create rope ONCE outside the benchmark
        let rope = Rope::from_str(&content);
        let middle = rope.len_chars() / 2;

        group.bench_function(BenchmarkId::new("insert_char", size_name), |b| {
            b.iter(|| {
                // Clone the rope for each iteration (this tests rope clone + insert)
                let mut r = rope.clone();
                r.insert_char(middle, 'x');
                black_box(r.len_chars())
            })
        });
    }

    group.finish();
}

/// Benchmark raw ropey insert WITHOUT clone (true O(log n) test).
fn bench_rope_insert_only(c: &mut Criterion) {
    let mut group = c.benchmark_group("render/rope_insert_only");
    group.sample_size(100);

    for size in [TestSizes::SMALL, TestSizes::MEDIUM, TestSizes::LARGE] {
        let size_name = match size {
            TestSizes::SMALL => "10KB",
            TestSizes::MEDIUM => "100KB",
            TestSizes::LARGE => "1MB",
            _ => "unknown",
        };

        let content = generate_rust_source(size);

        group.bench_with_input(
            BenchmarkId::new("insert_char_no_clone", size_name),
            &content,
            |b, content| {
                b.iter_batched(
                    || {
                        let rope = Rope::from_str(content);
                        let middle = rope.len_chars() / 2;
                        (rope, middle)
                    },
                    |(mut rope, middle)| {
                        rope.insert_char(middle, 'x');
                        black_box(rope.len_chars())
                    },
                    criterion::BatchSize::PerIteration,
                )
            },
        );
    }

    group.finish();
}

/// Benchmark Document creation (with tree-sitter parsing).
fn bench_document_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("render/document_creation");
    group.sample_size(20);

    for size in [TestSizes::SMALL, TestSizes::MEDIUM, TestSizes::LARGE] {
        let size_name = match size {
            TestSizes::SMALL => "10KB",
            TestSizes::MEDIUM => "100KB",
            TestSizes::LARGE => "1MB",
            _ => "unknown",
        };

        let content = generate_rust_source(size);

        group.bench_function(BenchmarkId::new("from_str", size_name), |b| {
            b.iter(|| {
                let doc = Document::from_str(&content, Some(PathBuf::from("test.rs")));
                black_box(doc.id())
            })
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_build_render_model,
    bench_cursor_positions,
    bench_with_selection,
    bench_viewport_sizes,
    bench_sidebar_toggle,
    bench_content_access,
    bench_navigation_render_cycle,
    bench_edit_render_cycle,
    bench_incremental_syntax,
    bench_buffer_only,
    bench_raw_rope,
    bench_rope_insert_only,
    bench_document_creation,
);

criterion_main!(benches);
