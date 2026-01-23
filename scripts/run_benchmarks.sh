#!/bin/bash
# Muda Editor Performance Benchmarks Runner
#
# Usage:
#   ./scripts/run_benchmarks.sh           # Run all benchmarks
#   ./scripts/run_benchmarks.sh quick     # Run quick subset
#   ./scripts/run_benchmarks.sh syntax    # Run only syntax benchmarks
#   ./scripts/run_benchmarks.sh editor    # Run only editor benchmarks
#
# Results are saved in target/criterion/

set -e

MODE="${1:-all}"

echo "============================================"
echo "  Muda Editor Performance Benchmarks"
echo "============================================"
echo ""

# Ensure we're in the project root
if [ ! -f "Cargo.toml" ]; then
    echo "Error: Please run this script from the project root directory."
    exit 1
fi

case "$MODE" in
    quick)
        echo "Running quick benchmarks (text_buffer only)..."
        cargo bench --bench text_buffer_benchmarks -- --sample-size 10
        ;;
    syntax)
        echo "Running syntax highlighting benchmarks..."
        cargo bench --bench syntax_benchmarks
        ;;
    editor)
        echo "Running editor end-to-end benchmarks..."
        cargo bench --bench editor_benchmarks
        ;;
    render)
        echo "Running render benchmarks..."
        cargo bench --bench render_benchmarks
        ;;
    text)
        echo "Running text buffer benchmarks..."
        cargo bench --bench text_buffer_benchmarks
        ;;
    all)
        echo "Running all benchmarks (this may take several minutes)..."
        echo ""

        echo "[1/4] Text Buffer Benchmarks..."
        cargo bench --bench text_buffer_benchmarks

        echo ""
        echo "[2/4] Syntax Highlighting Benchmarks..."
        cargo bench --bench syntax_benchmarks

        echo ""
        echo "[3/4] Render Benchmarks..."
        cargo bench --bench render_benchmarks

        echo ""
        echo "[4/4] Editor End-to-End Benchmarks..."
        cargo bench --bench editor_benchmarks
        ;;
    baseline)
        echo "Saving baseline measurements..."
        cargo bench -- --save-baseline before_optimization
        echo "Baseline saved as 'before_optimization'"
        ;;
    compare)
        echo "Comparing against baseline..."
        cargo bench -- --baseline before_optimization
        ;;
    *)
        echo "Unknown mode: $MODE"
        echo "Valid modes: all, quick, syntax, editor, render, text, baseline, compare"
        exit 1
        ;;
esac

echo ""
echo "============================================"
echo "  Benchmarks Complete!"
echo "============================================"
echo ""
echo "HTML reports available in: target/criterion/"
echo "Open target/criterion/report/index.html to view results."
