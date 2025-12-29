set positional-arguments

alias t := test
alias f := fix
alias b := build
alias c := clean

default:
    @just --list

# ============================================================
# CI
# ============================================================

# Run the full CI suite
ci: fix check lychee check-udeps

# Run all checks
check: check-format check-clippy test

# Check links in documentation
lychee:
    @command -v lychee >/dev/null 2>&1 || cargo install lychee
    lychee --config ./lychee.toml .

# ============================================================
# Formatting
# ============================================================

# Check code formatting
check-format:
    cargo +nightly fmt --all -- --check

# Format code
format-fix:
    cargo fix --allow-dirty --allow-staged
    cargo +nightly fmt --all

# ============================================================
# Linting
# ============================================================

# Run clippy
check-clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# Auto-fix clippy issues
clippy-fix:
    cargo clippy --workspace --all-targets --fix --allow-dirty --allow-staged

# ============================================================
# Testing
# ============================================================

# Run tests
test:
    @command -v cargo-nextest >/dev/null 2>&1 || cargo install cargo-nextest
    RUSTFLAGS="-D warnings" cargo nextest run --workspace --all-features

# Watch tests
watch-test:
    cargo watch -x "nextest run --workspace --all-features"

# ============================================================
# Building
# ============================================================

# Build release
build:
    cargo build --workspace --release

# Clean build artifacts
clean:
    cargo clean

# ============================================================
# Dependencies
# ============================================================

# Check for unused dependencies
check-udeps:
    @command -v cargo-udeps >/dev/null 2>&1 || cargo install cargo-udeps
    cargo +nightly udeps --workspace --all-features --all-targets

# ============================================================
# Documentation
# ============================================================

# Generate and open documentation
doc:
    cargo doc --workspace --no-deps --open

# Watch for changes and run checks
watch-check:
    cargo watch -x "clippy --workspace --all-targets"

# ============================================================
# Install
# ============================================================

# Install paracas from crates.io
install:
    cargo install paracas

# Install paracas locally from source
install-local:
    cargo install --path bin

# ============================================================
# Download Commands
# ============================================================

# Download tick data (all available history by default)
download instrument start="" end="" output="" format="csv":
    cargo run --package paracas --release -- download {{instrument}} \
        {{ if start != "" { "-s " + start } else { "" } }} \
        {{ if end != "" { "-e " + end } else { "" } }} \
        {{ if output != "" { "-o " + output } else { "" } }} \
        -f {{format}}

# Download with OHLCV aggregation
download-ohlcv instrument start="" end="" output="" format="csv" timeframe="h1":
    cargo run --package paracas --release -- download {{instrument}} \
        {{ if start != "" { "-s " + start } else { "" } }} \
        {{ if end != "" { "-e " + end } else { "" } }} \
        {{ if output != "" { "-o " + output } else { "" } }} \
        -f {{format}} -t {{timeframe}}

# List available instruments
list-instruments:
    cargo run --package paracas --release -- list

# Show instrument info
info instrument:
    cargo run --package paracas --release -- info {{instrument}}

# Compound command for development
fix:
    cargo +nightly fmt --all
    cargo clippy --workspace --all-targets --fix --allow-dirty --allow-staged

# ============================================================
# Benchmarks
# ============================================================

# Run criterion benchmarks
bench:
    cargo bench --package paracas-bench

# Run benchmark and output markdown table for README
bench-table:
    cargo build --release
    cargo run --package paracas-bench --bin benchmark_table --release

# Quick benchmark (1-day only, fewer iterations)
bench-quick:
    cargo build --release
    cargo run --package paracas-bench --bin benchmark_table --release -- --quick
