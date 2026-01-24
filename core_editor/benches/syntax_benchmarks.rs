//! Syntax Highlighting Performance Benchmarks
//!
//! Measures the tree-sitter based syntax highlighting:
//! - Full document parsing (current bottleneck)
//! - Per-line highlighting
//! - Query execution time

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use core_editor::syntax::{SyntaxHighlighter, SyntaxLanguage};

mod test_data;
use test_data::{generate_json, generate_plain_text, generate_rust_source, TestSizes};

/// Benchmark full document parsing (the current O(n) bottleneck).
fn bench_full_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("syntax/full_parse");
    group.sample_size(50); // Fewer samples for slow operations

    for size in [
        TestSizes::TINY,
        TestSizes::SMALL,
        TestSizes::MEDIUM,
        TestSizes::LARGE,
    ] {
        let size_name = match size {
            TestSizes::TINY => "1KB",
            TestSizes::SMALL => "10KB",
            TestSizes::MEDIUM => "100KB",
            TestSizes::LARGE => "1MB",
            _ => "unknown",
        };

        // Rust syntax (most complex)
        let rust_content = generate_rust_source(size);
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(
            BenchmarkId::new("rust", size_name),
            &rust_content,
            |b, content| {
                let mut highlighter = SyntaxHighlighter::new(SyntaxLanguage::Rust);
                b.iter(|| {
                    highlighter.parse(black_box(content));
                })
            },
        );

        // JSON syntax (simpler grammar)
        let json_content = generate_json(size);
        group.bench_with_input(
            BenchmarkId::new("json", size_name),
            &json_content,
            |b, content| {
                let mut highlighter = SyntaxHighlighter::new(SyntaxLanguage::Json);
                b.iter(|| {
                    highlighter.parse(black_box(content));
                })
            },
        );

        // Plain text (no parsing)
        let plain_content = generate_plain_text(size);
        group.bench_with_input(
            BenchmarkId::new("plain", size_name),
            &plain_content,
            |b, content| {
                let mut highlighter = SyntaxHighlighter::new(SyntaxLanguage::Plain);
                b.iter(|| {
                    highlighter.parse(black_box(content));
                })
            },
        );
    }

    group.finish();
}

/// Benchmark per-line highlighting.
fn bench_highlight_line(c: &mut Criterion) {
    let mut group = c.benchmark_group("syntax/highlight_line");

    for size in [TestSizes::SMALL, TestSizes::MEDIUM, TestSizes::LARGE] {
        let size_name = match size {
            TestSizes::SMALL => "10KB",
            TestSizes::MEDIUM => "100KB",
            TestSizes::LARGE => "1MB",
            _ => "unknown",
        };

        let content = generate_rust_source(size);
        let mut highlighter = SyntaxHighlighter::new(SyntaxLanguage::Rust);
        highlighter.parse(&content);

        // Get a line from the middle of the document
        let lines: Vec<&str> = content.lines().collect();
        let middle_line_idx = lines.len() / 2;
        let middle_line = lines.get(middle_line_idx).unwrap_or(&"");

        // Calculate byte offset for middle line
        let mut line_start_byte = 0;
        for (i, line) in content.lines().enumerate() {
            if i == middle_line_idx {
                break;
            }
            line_start_byte += line.len() + 1; // +1 for newline
        }

        group.bench_with_input(
            BenchmarkId::new("single_line", size_name),
            &(&content, &highlighter, middle_line_idx, line_start_byte, middle_line.to_string()),
            |b, (content, highlighter, line_idx, line_start_byte, line_text)| {
                b.iter(|| {
                    black_box(highlighter.highlight_line(
                        content,
                        *line_idx,
                        *line_start_byte,
                        line_text,
                    ))
                })
            },
        );
    }

    group.finish();
}

/// Benchmark highlighting multiple lines (simulating viewport rendering).
fn bench_highlight_viewport(c: &mut Criterion) {
    let mut group = c.benchmark_group("syntax/highlight_viewport");
    group.sample_size(50);

    for size in [TestSizes::SMALL, TestSizes::MEDIUM, TestSizes::LARGE] {
        let size_name = match size {
            TestSizes::SMALL => "10KB",
            TestSizes::MEDIUM => "100KB",
            TestSizes::LARGE => "1MB",
            _ => "unknown",
        };

        let content = generate_rust_source(size);
        let mut highlighter = SyntaxHighlighter::new(SyntaxLanguage::Rust);
        highlighter.parse(&content);

        let lines: Vec<&str> = content.lines().collect();
        let viewport_size = 40;
        let start_line = lines.len().saturating_sub(viewport_size) / 2;

        // Pre-calculate byte offsets for lines
        let mut line_offsets = Vec::new();
        let mut offset = 0;
        for line in &lines {
            line_offsets.push(offset);
            offset += line.len() + 1;
        }

        group.throughput(Throughput::Elements(viewport_size as u64));
        group.bench_with_input(
            BenchmarkId::new("40_lines", size_name),
            &(&content, &highlighter, &lines, &line_offsets, start_line),
            |b, &(content, highlighter, lines, line_offsets, start_line)| {
                b.iter(|| {
                    for i in 0..viewport_size {
                        let line_idx = start_line + i;
                        if line_idx < lines.len() {
                            black_box(highlighter.highlight_line(
                                content,
                                line_idx,
                                line_offsets[line_idx],
                                lines[line_idx],
                            ));
                        }
                    }
                })
            },
        );
    }

    group.finish();
}

/// Benchmark the edit+reparse cycle (what happens on every keystroke).
fn bench_edit_reparse_cycle(c: &mut Criterion) {
    let mut group = c.benchmark_group("syntax/edit_reparse");
    group.sample_size(30); // Fewer samples for very slow tests

    for size in [TestSizes::SMALL, TestSizes::MEDIUM] {
        let size_name = match size {
            TestSizes::SMALL => "10KB",
            TestSizes::MEDIUM => "100KB",
            _ => "unknown",
        };

        let base_content = generate_rust_source(size);

        // Simulate typing a character and reparsing
        group.bench_with_input(
            BenchmarkId::new("char_insert", size_name),
            &base_content,
            |b, content| {
                b.iter_batched(
                    || {
                        let content = content.clone();
                        let mut highlighter = SyntaxHighlighter::new(SyntaxLanguage::Rust);
                        highlighter.parse(&content);
                        (content, highlighter)
                    },
                    |(mut content, mut highlighter)| {
                        // Simulate inserting a character at middle
                        let middle = content.len() / 2;
                        content.insert(middle, 'x');
                        // This is the expensive part - full reparse
                        highlighter.parse(&content);
                        black_box((content, highlighter))
                    },
                    criterion::BatchSize::SmallInput,
                )
            },
        );
    }

    // Also test the 1MB case but with fewer iterations
    {
        let base_content = generate_rust_source(TestSizes::LARGE);

        group.sample_size(10);
        group.bench_with_input(
            BenchmarkId::new("char_insert", "1MB"),
            &base_content,
            |b, content| {
                b.iter_batched(
                    || {
                        let content = content.clone();
                        let mut highlighter = SyntaxHighlighter::new(SyntaxLanguage::Rust);
                        highlighter.parse(&content);
                        (content, highlighter)
                    },
                    |(mut content, mut highlighter)| {
                        let middle = content.len() / 2;
                        content.insert(middle, 'x');
                        highlighter.parse(&content);
                        black_box((content, highlighter))
                    },
                    criterion::BatchSize::SmallInput,
                )
            },
        );
    }

    group.finish();
}

/// Benchmark highlighter creation (happens when changing file type).
fn bench_highlighter_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("syntax/highlighter_creation");

    for lang in [
        SyntaxLanguage::Rust,
        SyntaxLanguage::Python,
        SyntaxLanguage::JavaScript,
        SyntaxLanguage::Json,
        SyntaxLanguage::Plain,
    ] {
        let lang_name = format!("{:?}", lang);
        group.bench_function(BenchmarkId::new("new", &lang_name), |b| {
            b.iter(|| {
                black_box(SyntaxHighlighter::new(lang))
            })
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_full_parse,
    bench_highlight_line,
    bench_highlight_viewport,
    bench_edit_reparse_cycle,
    bench_highlighter_creation,
);

criterion_main!(benches);
