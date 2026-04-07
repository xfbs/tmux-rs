# List available recipes
default:
  @just --list

# run unit and integration tests
test:
  # run all (unit and integration) tests
  cargo nextest run
  # run integration tests against system tmux
  TMUX_SERVER_BIN=/usr/bin/tmux TMUX_CLIENT_BIN=/usr/bin/tmux cargo nextest run --tests
  TMUX_CLIENT_BIN=/usr/bin/tmux cargo nextest run --tests
  TMUX_SERVER_BIN=/usr/bin/tmux cargo nextest run --tests

coverage:
  cargo llvm-cov --html

# run code formatter
format:
  cargo +nightly fmt --all

# run static linters
check:
  cargo +nightly fmt --all --check
  RUSTDOCFLAGS="-Dwarnings" cargo doc --no-deps
  cargo clippy -- -Dwarnings

# run tmux-rs under valgrind, in release mode (for manual testing)
valgrind:
  cargo build --release
  valgrind --log-file=target/valgrind-$RANDOM.txt ./target/release/tmux-rs

# run fuzz targets that are known-clean (no crashes) for a short soak test
fuzz duration="10":
  #!/usr/bin/env bash
  set -euo pipefail
  targets=(colour_find_rgb colour_fromstring style_parse key_string_lookup attributes utf8_decode regsub input_parse)
  for target in "${targets[@]}"; do
    echo "fuzzing $target"
    if ! cargo +nightly fuzz run "$target" -- -max_total_time={{duration}}; then
      echo "FAILED: $target"
      exit 1
    fi
    echo
  done
  echo "All ${#targets[@]} fuzz targets passed ({{duration}}s each)"

careful:
  cargo +nightly careful test -- --test-threads=1

sanitizer:
  ASAN_OPTIONS=detect_leaks=0 RUSTFLAGS="-Zsanitizer=address" cargo +nightly test --target x86_64-unknown-linux-gnu -- --test-threads=1

progress:
  @echo "$(rg unsafe | wc -l) unsafe, $(rg '\*mut' | wc -l) mut pointers"

alias fmt := format
alias lint := check
