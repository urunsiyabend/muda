//! Text Buffer Performance Benchmarks
//!
//! Measures the core text buffer operations that execute on every edit:
//! - Character insertion
//! - String insertion
//! - Deletion
//! - Position/offset conversions
//! - Line access

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use mudatexteditor::domain::text_buffer::{TextBuffer, TextRange};

mod test_data;
use test_data::{generate_rust_source, TestSizes};

/// Benchmark single character insertion at various positions.
fn bench_insert_char(c: &mut Criterion) {
    let mut group = c.benchmark_group("text_buffer/insert_char");

    for size in [TestSizes::SMALL, TestSizes::MEDIUM, TestSizes::LARGE] {
        let size_name = match size {
            TestSizes::SMALL => "10KB",
            TestSizes::MEDIUM => "100KB",
            TestSizes::LARGE => "1MB",
            _ => "unknown",
        };

        // Insert at beginning
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(
            BenchmarkId::new("beginning", size_name),
            &size,
            |b, &size| {
                let content = generate_rust_source(size);
                b.iter_batched(
                    || TextBuffer::from_str(&content),
                    |mut buf| {
                        buf.insert_char(0, 'X');
                        black_box(buf)
                    },
                    criterion::BatchSize::SmallInput,
                )
            },
        );

        // Insert at middle
        group.bench_with_input(BenchmarkId::new("middle", size_name), &size, |b, &size| {
            let content = generate_rust_source(size);
            let middle = content.chars().count() / 2;
            b.iter_batched(
                || TextBuffer::from_str(&content),
                |mut buf| {
                    buf.insert_char(middle, 'X');
                    black_box(buf)
                },
                criterion::BatchSize::SmallInput,
            )
        });

        // Insert at end
        group.bench_with_input(BenchmarkId::new("end", size_name), &size, |b, &size| {
            let content = generate_rust_source(size);
            b.iter_batched(
                || TextBuffer::from_str(&content),
                |mut buf| {
                    let end = buf.len_chars();
                    buf.insert_char(end, 'X');
                    black_box(buf)
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

/// Benchmark string insertion (multi-character).
fn bench_insert_string(c: &mut Criterion) {
    let mut group = c.benchmark_group("text_buffer/insert_string");

    let insert_sizes = [10, 100, 1000];

    for doc_size in [TestSizes::SMALL, TestSizes::MEDIUM, TestSizes::LARGE] {
        let doc_name = match doc_size {
            TestSizes::SMALL => "10KB_doc",
            TestSizes::MEDIUM => "100KB_doc",
            TestSizes::LARGE => "1MB_doc",
            _ => "unknown",
        };

        for insert_size in insert_sizes {
            let insert_text: String = (0..insert_size).map(|_| 'X').collect();
            let bench_id = format!("{}/{}chars", doc_name, insert_size);

            group.throughput(Throughput::Elements(insert_size as u64));
            group.bench_with_input(BenchmarkId::new("middle", &bench_id), &doc_size, |b, &size| {
                let content = generate_rust_source(size);
                let middle = content.chars().count() / 2;
                b.iter_batched(
                    || TextBuffer::from_str(&content),
                    |mut buf| {
                        buf.insert(middle, &insert_text);
                        black_box(buf)
                    },
                    criterion::BatchSize::SmallInput,
                )
            });
        }
    }

    group.finish();
}

/// Benchmark deletion operations.
fn bench_delete(c: &mut Criterion) {
    let mut group = c.benchmark_group("text_buffer/delete");

    for size in [TestSizes::SMALL, TestSizes::MEDIUM, TestSizes::LARGE] {
        let size_name = match size {
            TestSizes::SMALL => "10KB",
            TestSizes::MEDIUM => "100KB",
            TestSizes::LARGE => "1MB",
            _ => "unknown",
        };

        // Delete single character
        group.bench_with_input(
            BenchmarkId::new("single_char", size_name),
            &size,
            |b, &size| {
                let content = generate_rust_source(size);
                let middle = content.chars().count() / 2;
                b.iter_batched(
                    || TextBuffer::from_str(&content),
                    |mut buf| {
                        buf.delete(TextRange::new(middle, middle + 1));
                        black_box(buf)
                    },
                    criterion::BatchSize::SmallInput,
                )
            },
        );

        // Delete 100 characters
        group.bench_with_input(
            BenchmarkId::new("100_chars", size_name),
            &size,
            |b, &size| {
                let content = generate_rust_source(size);
                let middle = content.chars().count() / 2;
                b.iter_batched(
                    || TextBuffer::from_str(&content),
                    |mut buf| {
                        buf.delete(TextRange::new(middle, middle + 100));
                        black_box(buf)
                    },
                    criterion::BatchSize::SmallInput,
                )
            },
        );

        // Delete a line (approximately)
        group.bench_with_input(
            BenchmarkId::new("line", size_name),
            &size,
            |b, &size| {
                let content = generate_rust_source(size);
                b.iter_batched(
                    || TextBuffer::from_str(&content),
                    |mut buf| {
                        let line_start = buf.line_to_offset(buf.len_lines() / 2);
                        let line_len = buf.line_len(buf.len_lines() / 2);
                        buf.delete(TextRange::new(line_start, line_start + line_len + 1));
                        black_box(buf)
                    },
                    criterion::BatchSize::SmallInput,
                )
            },
        );
    }

    group.finish();
}

/// Benchmark offset/position conversions (used in cursor movement).
fn bench_coordinate_conversion(c: &mut Criterion) {
    let mut group = c.benchmark_group("text_buffer/coordinates");

    for size in [TestSizes::SMALL, TestSizes::MEDIUM, TestSizes::LARGE] {
        let size_name = match size {
            TestSizes::SMALL => "10KB",
            TestSizes::MEDIUM => "100KB",
            TestSizes::LARGE => "1MB",
            _ => "unknown",
        };

        let content = generate_rust_source(size);
        let buffer = TextBuffer::from_str(&content);
        let middle_offset = buffer.len_chars() / 2;
        let middle_line = buffer.len_lines() / 2;

        // Offset to position (line, column)
        group.bench_with_input(
            BenchmarkId::new("offset_to_position", size_name),
            &buffer,
            |b, buf| {
                b.iter(|| {
                    black_box(buf.offset_to_position(middle_offset))
                })
            },
        );

        // Position to offset
        let mid_pos = buffer.offset_to_position(middle_offset);
        group.bench_with_input(
            BenchmarkId::new("position_to_offset", size_name),
            &buffer,
            |b, buf| {
                b.iter(|| {
                    black_box(buf.position_to_offset(mid_pos))
                })
            },
        );

        // Line to offset
        group.bench_with_input(
            BenchmarkId::new("line_to_offset", size_name),
            &buffer,
            |b, buf| {
                b.iter(|| {
                    black_box(buf.line_to_offset(middle_line))
                })
            },
        );

        // Offset to line
        group.bench_with_input(
            BenchmarkId::new("offset_to_line", size_name),
            &buffer,
            |b, buf| {
                b.iter(|| {
                    black_box(buf.offset_to_line(middle_offset))
                })
            },
        );
    }

    group.finish();
}

/// Benchmark line access (used in rendering).
fn bench_line_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("text_buffer/line_access");

    for size in [TestSizes::SMALL, TestSizes::MEDIUM, TestSizes::LARGE] {
        let size_name = match size {
            TestSizes::SMALL => "10KB",
            TestSizes::MEDIUM => "100KB",
            TestSizes::LARGE => "1MB",
            _ => "unknown",
        };

        let content = generate_rust_source(size);
        let buffer = TextBuffer::from_str(&content);
        let middle_line = buffer.len_lines() / 2;

        // Get single line content
        group.bench_with_input(
            BenchmarkId::new("get_line", size_name),
            &buffer,
            |b, buf| {
                b.iter(|| {
                    black_box(buf.line(middle_line))
                })
            },
        );

        // Get line length
        group.bench_with_input(
            BenchmarkId::new("line_len", size_name),
            &buffer,
            |b, buf| {
                b.iter(|| {
                    black_box(buf.line_len(middle_line))
                })
            },
        );

        // Get 40 consecutive lines (typical viewport)
        group.bench_with_input(
            BenchmarkId::new("get_40_lines", size_name),
            &buffer,
            |b, buf| {
                let start_line = (buf.len_lines() / 2).saturating_sub(20);
                b.iter(|| {
                    for line_idx in start_line..start_line + 40 {
                        if line_idx < buf.len_lines() {
                            black_box(buf.line(line_idx));
                        }
                    }
                })
            },
        );
    }

    group.finish();
}

/// Benchmark full content retrieval (currently happens every frame).
fn bench_full_content(c: &mut Criterion) {
    let mut group = c.benchmark_group("text_buffer/full_content");

    for size in [TestSizes::SMALL, TestSizes::MEDIUM, TestSizes::LARGE] {
        let size_name = match size {
            TestSizes::SMALL => "10KB",
            TestSizes::MEDIUM => "100KB",
            TestSizes::LARGE => "1MB",
            _ => "unknown",
        };

        let content = generate_rust_source(size);
        let buffer = TextBuffer::from_str(&content);

        // This is what happens in build_visible_lines currently
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(
            BenchmarkId::new("to_string", size_name),
            &buffer,
            |b, buf| {
                b.iter(|| {
                    black_box(buf.to_string())
                })
            },
        );
    }

    group.finish();
}

/// Benchmark typing throughput (rapid character insertion).
fn bench_typing_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("text_buffer/typing_throughput");
    group.sample_size(50); // Fewer samples for slow tests

    for char_count in [100, 500, 1000] {
        let chars: Vec<char> = (0..char_count).map(|i| if i % 10 == 9 { '\n' } else { 'a' }).collect();

        group.throughput(Throughput::Elements(char_count as u64));
        group.bench_with_input(
            BenchmarkId::new("chars", char_count),
            &chars,
            |b, chars| {
                b.iter_batched(
                    || TextBuffer::from_str(""),
                    |mut buf| {
                        for (i, &ch) in chars.iter().enumerate() {
                            buf.insert_char(i, ch);
                        }
                        black_box(buf)
                    },
                    criterion::BatchSize::SmallInput,
                )
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_insert_char,
    bench_insert_string,
    bench_delete,
    bench_coordinate_conversion,
    bench_line_access,
    bench_full_content,
    bench_typing_throughput,
);

criterion_main!(benches);
