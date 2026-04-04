# Fuzzing tmux-rs

Requires nightly Rust and `cargo-fuzz`:

    cargo install cargo-fuzz

## Available targets

| Target | Function | Type |
|--------|----------|------|
| `colour_find_rgb` | `colour_find_rgb(r, g, b)` | Differential (vs old revision) |
| `style_parse` | `style_parse(style, base, input)` | Crash detection, sandboxed |
| `key_string_lookup` | `key_string_lookup_string(input)` | Crash detection, sandboxed |
| `colour_fromstring` | `colour_fromstring(input)` | Crash detection, sandboxed |
| `cmd_parse` | `cmd_parse_from_buffer(input)` | Crash detection, sandboxed |

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

Most fuzz targets are sandboxed with **Landlock** (Linux kernel security module).
The sandbox denies all filesystem writes except to the fuzz corpus and artifact
directories. Child processes inherit these restrictions, so even if code under
test spawns a shell command, it cannot modify the filesystem.

The sandbox requires Linux 5.13+ with Landlock enabled (check `cat /sys/kernel/security/lsm`).
It is configured with `CompatLevel::HardRequirement` — the fuzzer refuses to run
if Landlock is not available.

When adding new targets:
- Add `mod sandbox;` and call `sandbox::enable("target_name")` at the top of the fuzz closure
- Prefer functions that parse input without executing (style_parse, cmd_parse, etc.)
- The sandbox makes it safe to fuzz functions that *might* reach execution paths,
  but avoid targeting cmd_exec or spawn directly
