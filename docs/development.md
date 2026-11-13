**English** | [日本語](development.ja.md)

# Development

## Building

```sh
cargo build --release
cargo run --release                      # play
cargo run --release -- replay FILE       # replay a record
```

rs-nim uses Rust edition 2024, [ratatui](https://ratatui.rs) for drawing and [clap](https://docs.rs/clap) for the command line.

## Source layout

| File | Role |
|---|---|
| `src/main.rs` | Command-line parsing, the event loop (33 ms tick) and `replay --print` |
| `src/game.rs` | Rules, nim-sum and the winning move for normal and misère play. Independent of the UI |
| `src/ai.rs` | CPU move selection (per-difficulty accuracy) and a small xorshift RNG |
| `src/record.rs` | Reading and writing `.nimrec` records, and rebuilding the position after any number of moves |
| `src/app.rs` | Screens, input, normal and replay modes, CPU "thinking", animations, confetti and rain |
| `src/ui.rs` | Drawing |

`App` runs in one of two modes: `Mode::Normal` (play against the CPU) and `Mode::Replay` (play back a `Record`). Replay mode reuses the same board drawing; stepping backwards rebuilds the position from the record instead of undoing moves.

## Tests

```sh
cargo test                              # regular tests
cargo test -- --ignored --nocapture     # text dump of the screens and a full game against the CPU
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

Highlights:

- `game::tests::strategy_matches_brute_force` checks `winning_move` against exhaustive minimax for every position with three heaps of 0–6 stones, under both rules.
- `record` tests cover parsing, every error kind with its line number, the `to_text` → `parse` round trip and every file in `examples/records/`.
- `app` and `ui` tests drive replay forward, backward, to the start and to the end, and render replay screens at the minimum 90 × 26 size.
