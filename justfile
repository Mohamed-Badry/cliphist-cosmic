# List available recipes
default:
    @just --list

# Format the code using cargo fmt
fmt:
    cargo fmt

# Run clippy lints
lint:
    cargo clippy -- -D warnings

# Run all tests quietly
test:
    cargo test -q --all

# Build the release binary
build-release:
    cargo build --release --locked

# Run in debug mode with vim bindings enabled by default
dev *args:
    cargo run -- --vim {{args}}

# Run in release mode
run *args:
    cargo run --release -- {{args}}

# Run in release mode with vim bindings enabled
run-vim *args:
    cargo run --release -- --vim {{args}}

# Run in release mode with the horizontal preset
run-horizontal *args:
    cargo run --release -- --vim --width 650 --height 465 {{args}}

# Install the binary to ~/.cargo/bin
install:
    cargo install --path . --force --locked

# Uninstall the binary
uninstall:
    cargo uninstall cliphist-cosmic
