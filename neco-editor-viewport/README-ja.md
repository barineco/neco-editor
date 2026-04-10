# neco-editor-viewport

[英語版](README.md)

エディタ描画用のビューポート幾何計算。バイトオフセット、論理行/視覚行、ピクセル座標を相互変換する。DOM や Canvas API に依存しない純粋な幾何レイヤです。

## 動作の概要

すべての関数はステートレスで、`ViewportMetrics` (行高さ、ASCII 文字幅、CJK 文字幅、タブ幅) を入力に取ります。`ViewportLayout` はガターとコンテンツ領域のオフセットを加えた構造体です。

`visible_line_range` はスクロール位置とコンテナ高さから描画対象の視覚行範囲を算出し、`caret_rect` は指定バイトオフセットのカーソル描画位置をピクセル矩形で返します。`selection_rects` は選択範囲がカバーする視覚行ごとに矩形を 1 つずつ生成する関数です。

逆方向の変換も用意しており、`hit_test` はピクセル座標 `(x, y)` のクリックをバイトオフセットに変換します。

論理行と視覚行の変換は `neco-wrap::WrapMap`、行・列の解決は `neco-textview::LineIndex` に委譲します。ビューポート層自体は状態を持ちません。

## 使い方

```rust
use neco_editor_viewport::{
    ViewportMetrics, ViewportLayout, visible_line_range,
    caret_rect, hit_test, gutter_width,
};
use neco_textview::LineIndex;
use neco_wrap::{WrapMap, WrapPolicy};

let text = "hello\nworld";
let li = LineIndex::new(text);
let lines: Vec<&str> = text.split('\n').collect();
let wm = WrapMap::new(lines.iter().copied(), 80, &WrapPolicy::code());
let metrics = ViewportMetrics {
    line_height: 20.0,
    char_width: 8.0,
    cjk_char_width: 14.0,
    tab_width: 4,
};

// 表示範囲の視覚行
let (first, last) = visible_line_range(0.0, 100.0, &wm, &metrics);

// カーソルの描画位置
let gw = gutter_width(li.line_count(), &metrics);
let layout = ViewportLayout {
    gutter_width: gw,
    content_left: gw + 8.0,
};
let rect = caret_rect(text, 0, &li, &wm, &metrics, &layout).unwrap();

// クリック座標からバイトオフセットへ
let offset = hit_test(rect.x, rect.y, 0.0, text, &li, &wm, &metrics, &layout);
assert_eq!(offset, 0);
```

## API

| 項目 | 説明 |
|------|------|
| `ViewportMetrics` | フォントメトリクス: line_height, char_width, cjk_char_width, tab_width |
| `ViewportLayout` | 算出済みレイアウト: gutter_width, content_left |
| `Rect` | ピクセル矩形: x, y, width, height |
| `ViewportError` | LineIndex 操作由来の `TextViewError` を内包 |
| `visible_line_range(scroll_top, container_height, wrap_map, metrics)` | 表示範囲の先頭・末尾の視覚行番号 |
| `caret_rect(text, offset, line_index, wrap_map, metrics, layout)` | `offset` のカーソル描画位置をピクセル矩形で返す |
| `selection_rects(text, selection, line_index, wrap_map, metrics, layout)` | 選択範囲の視覚行ごとの矩形 |
| `hit_test(x, y, scroll_top, text, line_index, wrap_map, metrics, layout)` | ピクセル座標からバイトオフセットに変換 |
| `gutter_width(total_lines, metrics)` | 行番号ガターのピクセル幅 |
| `line_top(visual_line, metrics)` | 視覚行の上端 Y 座標 |
| `scroll_to_reveal(text, offset, scroll_top, container_height, line_index, wrap_map, metrics)` | カーソルが見える scroll_top を返す。既に見えていれば `None` |

## ライセンス

MIT
