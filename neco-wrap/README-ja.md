# neco-wrap

[英語版](README.md)

論理行を視覚行に分割する行折り返しエンジン。改行判定と文字幅判定をプラグ可能なポリシーで差し替えられます。

## 動作の概要

`neco-wrap` は `WrapPolicy` に従って論理行を視覚行に分割します。ポリシーは 2 つの関数ポインタをまとめたものです。1 つは文字の視覚的な幅を返す関数、もう 1 つは指定したバイトオフセットで改行が許可・禁止・必須のどれかを返す関数です。

`wrap_line` は 1 つの論理行を処理し、視覚行の区切り位置を示す `WrapPoint` のリストを返す低レベル関数です。各 `WrapPoint` が保持するのはバイトオフセットとその時点の累積視覚幅です。`WrapMap` はこれをドキュメント全体に適用して論理行ごとに結果を管理し、論理座標 `(行番号, バイトオフセット)` と視覚行番号の相互変換も担います。

組み込みポリシーは 2 種類あり、`WrapPolicy::code` は ASCII 空白と一般的な演算子の後で改行し、`WrapPolicy::japanese_basic` は日本語テキスト向けに禁則処理を適用する構成です。

## 使い方

### 1 行を折り返す

```rust
use neco_wrap::{WrapPolicy, wrap_line};

let policy = WrapPolicy::code();
let wraps = wrap_line("ab cd ef", 4, &policy);

// "ab " の後と "cd " の後の 2 箇所に折り返し点が入る
assert_eq!(wraps.len(), 2);
assert_eq!(wraps[0].byte_offset(), 3);
assert_eq!(wraps[1].byte_offset(), 6);
```

### WrapMap でドキュメント全体を管理する

```rust
use neco_wrap::{WrapPolicy, WrapMap};

let lines = ["hello world", "foo bar baz"];
let policy = WrapPolicy::code();
let mut map = WrapMap::new(lines.iter().copied(), 6, &policy);

// 全論理行を合計した視覚行数
let total = map.total_visual_lines();

// 論理座標 (行番号, バイトオフセット) から視覚行番号に変換
let vline = map.to_visual_line(0, 6);

// 編集後は変更した行だけを再計算
map.rewrap_line(0, "hi", &policy);
```

## API

| 項目 | 説明 |
|------|-------------|
| `BreakOpportunity` | バイトオフセットでの改行判定: `Allowed`、`Forbidden`、`Mandatory` |
| `WrapPolicy` | `char_width` と `break_opportunity` の関数ポインタをまとめた型 |
| `WrapPolicy::new(char_width, break_opportunity)` | 2 つの関数ポインタからコンストラクタ |
| `WrapPolicy::char_width()` | 保持している文字幅関数を返す |
| `WrapPolicy::break_opportunity()` | 保持している改行判定関数を返す |
| `WrapPolicy::code()` | 組み込みポリシー: ASCII 空白とコード演算子の後で改行 |
| `WrapPolicy::japanese_basic()` | 組み込みポリシー: 禁則処理付きの日本語改行 |
| `WrapPoint` | 視覚行の区切り位置のバイトオフセットと累積視覚幅 |
| `WrapPoint::byte_offset()` | 視覚行が終わる論理行内のバイトオフセット |
| `WrapPoint::visual_width()` | この折り返し点までの累積視覚カラム数 |
| `VisualLine` | 論理行内の 1 視覚行の `[start, end)` バイト範囲 |
| `VisualLine::start()` | 開始バイトオフセット |
| `VisualLine::end()` | 終了バイトオフセット |
| `VisualLine::len()` | 視覚行のバイト長 |
| `VisualLine::is_empty()` | start と end が等しいとき true |
| `WrapMap` | ドキュメント全体の折り返し状態。論理行ごとに折り返し点を保持する |
| `WrapMap::new(lines, max_width, policy)` | 行イテレータから構築する |
| `WrapMap::max_width()` | 折り返しに使うカラム上限 |
| `WrapMap::line_count()` | 論理行数 |
| `WrapMap::visual_line_count(line)` | 指定した論理行の視覚行数 |
| `WrapMap::total_visual_lines()` | ドキュメント全体の視覚行数の合計 |
| `WrapMap::wrap_points(line)` | 論理行の `WrapPoint` スライス |
| `WrapMap::visual_lines(line, line_len)` | 論理行の `VisualLine` セグメント一覧 |
| `WrapMap::to_visual_line(line, byte_offset_in_line)` | 論理座標から絶対視覚行番号に変換 |
| `WrapMap::from_visual_line(visual_line)` | 絶対視覚行番号から `(論理行番号, 開始バイトオフセット)` に変換 |
| `WrapMap::rewrap_line(line, line_text, policy)` | 編集後に 1 論理行の折り返し点を再計算する |
| `WrapMap::set_max_width(max_width, lines, policy)` | カラム上限を変更して全行を再計算する |
| `WrapMap::splice_lines(start_line, removed_count, new_lines, policy)` | 論理行の範囲を置き換える。ドキュメントの splice に対応する |
| `wrap_line(line_text, max_width, policy)` | 低レベル関数: 1 行を折り返して `Vec<WrapPoint>` を返す |

## ライセンス

MIT
