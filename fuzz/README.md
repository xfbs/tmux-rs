# Fuzzing tmux-rs

Requires nightly Rust and `cargo-fuzz`:

    cargo install cargo-fuzz

## Available targets

| Target | Function | Type |
|--------|----------|------|
| `colour_find_rgb` | `colour_find_rgb(r, g, b)` | Differential (vs old revision) |
| `style_parse` | `style_parse(style, base, input)` | Crash detection |
| `key_string_lookup` | `key_string_lookup_string(input)` | Crash detection |
| `colour_fromstring` | `colour_fromstring(input)` | Crash detection |

## Commands

List available fuzz targets:

    cargo +nightly fuzz list

Run a specific target:

    cargo +nightly fuzz run style_parse

Run with a time limit:

    cargo +nightly fuzz run style_parse -- -max_total_time=300

Run with more cores:

    cargo +nightly fuzz run style_parse -- -jobs=8

Minimize a crash artifact:

    cargo +nightly fuzz tmin style_parse fuzz/artifacts/style_parse/<crash-file>

## Safety

All current fuzz targets exercise **pure parsing functions** that do not execute
commands, write files, or perform I/O. This is intentional — tmux can execute
arbitrary shell commands, so fuzz targets must never reach the command execution
layer.

When adding new targets, restrict to functions that:
- Parse input into data structures (style_parse, key_string_lookup_string, etc.)
- Perform computation without side effects (colour_find_rgb, etc.)
- Do NOT feed into cmdq, cmd_exec, or spawn
