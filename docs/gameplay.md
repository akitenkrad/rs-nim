**English** | [日本語](gameplay.ja.md)

# Playing rs-nim

## Rules

The board holds several heaps of stones. Players alternate turns; on each turn you pick **one** heap and remove **one or more** stones from it.

| Rule | Who wins |
|---|---|
| Normal | The player who takes the last stone wins. |
| Misère | The player who takes the last stone **loses**. |

## Starting a game

Run `rs-nim` (or `cargo run --release`) to open the title screen. Change the settings there, then press `Enter` / `Space` to start.

| Key | Setting | Values |
|---|---|---|
| `d` | Difficulty | Easy / Normal / Perfect |
| `r` | Rule | Normal / Misère |
| `f` | First player | You / CPU |
| `p` | Heap layout | Random (3–5 heaps of 1–9 stones) / 1-3-5-7 (classic) |
| `l` | Language | English / 日本語 |

The CPU plays the winning move (see [strategy](strategy.md)) with a probability that depends on the difficulty: 20 % on Easy, 65 % on Normal and 100 % on Perfect. When no winning move exists, Perfect takes a single stone from the largest heap and waits for you to slip; the other levels play a random legal move.

## Controls during a game

| Key | Action |
|---|---|
| `←` `→` / `Tab` | Select a heap |
| `↑` `↓` / `1`-`9` | Number of stones to take |
| `Enter` / `Space` | Take the selected stones |
| `h` | Hint: highlight the winning move (or tell you the position is lost) |
| `b` | Toggle the nim-sum panel |
| `s` | Save the current game as a record file (see [replay](replay.md)) |
| `n` | New game |
| `t` / `Esc` | Back to the title screen |
| `q` / `Ctrl-C` | Quit |

The selected stones are highlighted before you commit. Removed stones float up, thin out and fade away. Winning showers the screen with confetti; losing makes it rain.

## The nim-sum panel

Press `b` to show each heap in binary together with their XOR (the *nim-sum*). Bit columns that contain an odd number of 1s are highlighted — those are exactly the columns you need to fix to hand your opponent a losing position. See [strategy](strategy.md) for why.

## Language

The interface is in English by default. Switch to Japanese with `l` on the title screen, or start the game with `--lang ja` (this also works for `replay`).

## Terminal requirements

- At least **90 × 26** cells.
- A true-colour (24-bit) terminal is recommended: WezTerm, iTerm2, kitty, Alacritty, …
- Stones are drawn with background-coloured cells, so no special font is required.
- If box-drawing characters or arrows look misaligned in a CJK locale, set your terminal's *East Asian Ambiguous width* to 1 (narrow).
