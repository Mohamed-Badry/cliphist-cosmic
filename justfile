set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

default:
    @just --list

fmt:
    cargo fmt

test:
    cargo test -q --all

build-release:
    cargo build --release --locked

run *args:
    cargo run --release -- {{args}}

run-vim *args:
    cargo run --release -- --vim {{args}}

install:
    cargo install --path . --force --locked

uninstall:
    cargo uninstall cliphist-cosmic
