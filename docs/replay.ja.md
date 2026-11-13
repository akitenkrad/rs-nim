[English](replay.md) | **日本語**

# 棋譜とリプレイ

rs-nim はテキスト形式の棋譜ファイル（`.nimrec`）から対局を再生できます．棋譜は手で書いても，別のプログラムで生成しても，自分の対局から保存してもかまいません．リプレイは録画にも使えるように作ってあり，一定の間隔で自動再生し，最後まで再生したら自動で終了できます．

## 棋譜の再生

```sh
rs-nim replay examples/records/classic-1357.nimrec
```

| オプション | 説明 |
|---|---|
| `<FILE>` | 棋譜ファイル．`-` で標準入力から読む |
| `--interval <INTERVAL>` | 自動再生の 1 手あたりの間隔（ミリ秒．既定 `1500`．石が消えるアニメーションの 480 ms より短い値は 480 に切り上げ） |
| `--paused` | 一時停止の状態で開始 |
| `--exit-on-end` | 終局した棋譜の最後まで再生したら，3 秒後に終了 |
| `--print` | TUI を起動せず，再生内容をテキストで出力 |
| `--lang <LANG>` | 表示言語．`en`（既定）か `ja`．`--print` の出力にも効く |

ファイルが読めない場合や棋譜に誤りがある場合は，TUI を起動する前に問題点（行番号付き）を標準エラーに出し，終了コード 1 で終わります．

### リプレイ中の操作

| キー | 動作 |
|---|---|
| `Space` | 再生 / 一時停止 |
| `→` / `l` | 1 手進む（アニメーション付き） |
| `←` / `h` | 1 手戻る |
| `Home` / `g` | 先頭へ |
| `End` / `G` | 末尾へ |
| `+` / `-` | 速く / 遅く（1 段 250 ms） |
| `b` | ニム和パネルの表示切替 |
| `q` / `Esc` / `Ctrl-C` | 終了 |

ステータス欄には再生中かどうか，手数（`k/N`），現在の間隔を表示します．最後の手で終局していれば，結果パネルに勝者の名前を表示します．途中で終わる棋譜では「棋譜の終わり」と表示します．

### テキスト出力

`--print` は棋譜を検査し，各手と，その手の後の山とニム和を出力します．ファイルの確認やスクリプトからの利用に便利です．

```sh
rs-nim --lang ja replay examples/records/classic-1357.nimrec --print
```

```text
手数 1: Alice D 2
  山: A=1 B=3 C=5 D=5
  ニム和: 2
  ...
```

## 自分の対局を保存する

対局中または結果画面で `s` を押すと，現在の対局を `nim-<unix 秒>.nimrec` として保存します．保存先はカレントディレクトリです．`--record-dir <DIR>` を付けて起動すると変えられます．

```sh
rs-nim --record-dir ~/nim-records
```

保存した棋譜には既定の名前を書きません．そのため英語では You / CPU，日本語では あなた / CPU と表示されます．名前を付けるには `human` / `cpu` の行を追加します．

## `.nimrec` 形式

棋譜は UTF-8 のテキストで，`key = value` 形式のヘッダの後に，1 行 1 手で着手を並べます．

```text
# rs-nim record v1
rule  = normal     # normal | misere   (default: normal)
first = human      # human | cpu       (default: human)
heaps = 1 3 5 7    # required; separated by spaces or commas
human = Alice      # optional display name of the "human" side (default: You / あなた)
cpu   = Bob        # optional display name of the "cpu" side   (default: CPU)

D 2                # move: heap, then number of stones to take
B 2
c 3                # heap letters are case-insensitive
4 3                # heaps may also be numbered from 1 (4 = D)
```

規則:

- `#` 以降はコメント，空行は無視します．
- ヘッダのキーは最初の着手より前にだけ書けます．未知のキー・重複したキーはエラーです．
- `heaps` は 1〜6 個の山で，各山は 0〜9 個．少なくとも 1 つは空でない山が必要です．
- `human` と `cpu` は 2 つの陣営を表すだけです．リプレイでは指定した名前で表示するので，任意の 2 人の対局を記録できます．
- 着手は `first` から交互です．各着手は合法でなければなりません．山が存在し，取る個数は 1 以上かつ残りの個数以下です．
- 終局後の着手はエラーです．終局前で終わる棋譜は許されます．

[`examples/records/`](../examples/records/) にサンプル棋譜が 3 つあります．

| ファイル | 内容 |
|---|---|
| `classic-1357.nimrec` | 1-3-5-7，通常ルール．後手がニム和を 0 に保ち続けて勝つ |
| `misere.nimrec` | 短いミゼールの対局 |
| `unfinished.nimrec` | 途中で終わる棋譜 |

## 録画する

リプレイを使えば同じ対局を何度でも再現できるので，録画のやり直しも簡単です．録画する端末は 90 × 26 以上にしてください．

[asciinema](https://asciinema.org) の場合:

```sh
asciinema rec -c "rs-nim replay game.nimrec --exit-on-end" nim.cast
```

[VHS](https://github.com/charmbracelet/vhs) の場合は，次のようなテープファイルを使います．

```text
Output nim.gif
Set Width 1400
Set Height 700
Type "rs-nim replay game.nimrec --interval 1200 --exit-on-end"
Enter
Sleep 20s
```

`--exit-on-end` は最終局面と結果を 3 秒間表示してから終了するので，録画がきれいに終わります．録画しながら手動で 1 手ずつ進めたい場合は，代わりに `--paused` を使ってください．
