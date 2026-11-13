**English** | [日本語](strategy.ja.md)

# Winning strategy

Nim is completely solved. rs-nim implements the solution in `game::winning_move`, which powers both the hint (`h`) and the CPU.

## Normal play: Bouton's theorem

Write every heap size in binary and XOR them together. The result is the **nim-sum**.

- If the nim-sum is **0**, the player to move loses against perfect play.
- If the nim-sum is **not 0**, the player to move wins: there is always a move that makes the nim-sum 0.

To find that move, let `s` be the nim-sum. Pick a heap `h` such that `h XOR s < h` and reduce it to `h XOR s`.

Example with heaps `3 4 5`:

```text
  3 = 011
  4 = 100
  5 = 101
XOR = 010   -> nim-sum 2, the player to move is winning
```

`3 XOR 2 = 1 < 3`, so take 2 stones from heap A (3 → 1). The heaps become `1 4 5`, whose nim-sum is 0.

The nim-sum panel (`b`) highlights the bit columns with an odd number of 1s; a winning move is one that makes every column even again.

The classic `1 3 5 7` layout has nim-sum 0, so whoever moves first is losing.

## Misère play

Play exactly as in normal play **until** the move would leave no heap with two or more stones. At that point, leave an **odd** number of heaps of size 1 (instead of an even number) so that your opponent is forced to take the last stone.

Concretely:

- While two or more heaps have ≥ 2 stones: use the normal-play move.
- When exactly one heap has ≥ 2 stones: reduce it to 0 or 1 so that an odd number of 1-heaps remains.
- When every heap is 0 or 1: you win if the number of 1-heaps is even (take one); otherwise you are losing.

## Verification

The test `game::tests::strategy_matches_brute_force` checks every position with three heaps of 0–6 stones, under both rules, against an exhaustive minimax search. It asserts that `winning_move` returns a move exactly when the position is winning, and that the position after that move is losing for the opponent.
