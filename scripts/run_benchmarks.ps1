# Muda Editor Performance Benchmarks Runner
#
# Usage:
#   .\scripts\run_benchmarks.ps1           # Run all benchmarks
#   .\scripts\run_benchmarks.ps1 quick     # Run quick subset
#   .\scripts\run_benchmarks.ps1 syntax    # Run only syntax benchmarks
#   .\scripts\run_benchmarks.ps1 editor    # Run only editor benchmarks
#
# Results are saved in target/criterion/

param(
    [string]$Mode = "all"
)

$ErrorActionPreference = "Stop"

Write-Host "============================================" -ForegroundColor Cyan
Write-Host "  Muda Editor Performance Benchmarks" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host ""

# Ensure we're in the project root
if (-not (Test-Path "Cargo.toml")) {
    Write-Host "Error: Please run this script from the project root directory." -ForegroundColor Red
    exit 1
}

switch ($Mode) {
    "quick" {
        Write-Host "Running quick benchmarks (text_buffer only)..." -ForegroundColor Yellow
        cargo bench --bench text_buffer_benchmarks -- --sample-size 10
    }
    "syntax" {
        Write-Host "Running syntax highlighting benchmarks..." -ForegroundColor Yellow
        cargo bench --bench syntax_benchmarks
    }
    "editor" {
        Write-Host "Running editor end-to-end benchmarks..." -ForegroundColor Yellow
        cargo bench --bench editor_benchmarks
    }
    "render" {
        Write-Host "Running render benchmarks..." -ForegroundColor Yellow
        cargo bench --bench render_benchmarks
    }
    "text" {
        Write-Host "Running text buffer benchmarks..." -ForegroundColor Yellow
        cargo bench --bench text_buffer_benchmarks
    }
    "all" {
        Write-Host "Running all benchmarks (this may take several minutes)..." -ForegroundColor Yellow
        Write-Host ""

        Write-Host "[1/4] Text Buffer Benchmarks..." -ForegroundColor Green
        cargo bench --bench text_buffer_benchmarks

        Write-Host ""
        Write-Host "[2/4] Syntax Highlighting Benchmarks..." -ForegroundColor Green
        cargo bench --bench syntax_benchmarks

        Write-Host ""
        Write-Host "[3/4] Render Benchmarks..." -ForegroundColor Green
        cargo bench --bench render_benchmarks

        Write-Host ""
        Write-Host "[4/4] Editor End-to-End Benchmarks..." -ForegroundColor Green
        cargo bench --bench editor_benchmarks
    }
    "baseline" {
        Write-Host "Saving baseline measurements..." -ForegroundColor Yellow
        cargo bench -- --save-baseline before_optimization
        Write-Host "Baseline saved as 'before_optimization'" -ForegroundColor Green
    }
    "compare" {
        Write-Host "Comparing against baseline..." -ForegroundColor Yellow
        cargo bench -- --baseline before_optimization
    }
    default {
        Write-Host "Unknown mode: $Mode" -ForegroundColor Red
        Write-Host "Valid modes: all, quick, syntax, editor, render, text, baseline, compare" -ForegroundColor Yellow
        exit 1
    }
}

Write-Host ""
Write-Host "============================================" -ForegroundColor Cyan
Write-Host "  Benchmarks Complete!" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "HTML reports available in: target/criterion/" -ForegroundColor Green
Write-Host "Open target/criterion/report/index.html to view results." -ForegroundColor Green
