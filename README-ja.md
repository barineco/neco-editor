# neco editor

[English](README.md)

`neco editor` は、エディタランタイム向けのテキスト編集、ファイルツリー管理、ファイル監視イベント正規化を扱う Rust crate 群です。

アプリケーション内で共通化していたエディタ側の基盤処理を、独立して再利用できる crate へ切り出しています。パス関係判定、インメモリのファイルツリー、小規模テキストパッチ、監視イベント統合など、それぞれが1つの責務に絞られており、crates.io で個別に利用可能です。

## crate 一覧

| crate | 概要 | 内部依存 | 主な外部依存 |
|---|---|---|---|
| [`neco-pathrel`](./neco-pathrel) | パス関係判定と名前変更追従の補助 | なし | なし |
| [`neco-filetree`](./neco-filetree) | ファイルツリーの探索、差し替え、平坦化、展開計画の補助 | `neco-pathrel` | なし |
| [`neco-textpatch`](./neco-textpatch) | 小さな構造化テキスト更新向けの決定的パッチ補助 | なし | なし |
| [`neco-watchnorm`](./neco-watchnorm) | ファイル監視イベントの正規化と一括統合 | なし | なし |
| [`neco-textview`](./neco-textview) | 効率的な位置・オフセット変換を持つ行インデックス付きテキストバッファ | なし | なし |
| [`neco-decor`](./neco-decor) | エディタオーバーレイ向けスパンベースの装飾モデル | `neco-textview` | なし |
| [`neco-diffcore`](./neco-diffcore) | 行単位の変更検出向け最小差分エンジン | なし | なし |
| [`neco-wrap`](./neco-wrap) | 等幅エディタ向けのソフトラップ行マップ | `neco-textview` | なし |
| [`neco-history`](./neco-history) | ツリーベースの分岐を持つ汎用 undo/redo 履歴 | `neco-tree` (neco-crates) | なし |

各 crate は crates.io で個別公開できるよう、意図的に独立性を保っています。運用は monorepo 体制ですが、実行時に密結合する単一フレームワークではありません。

このリポジトリはまだ開発途中で、crate ごとに成熟度に差があります。すでに実用できる部分もあれば、まだ詰めている途中の実装も含みます。

更新では、内部実装の変更が比較的起こりやすい状態です。特にアルゴリズム差し替えや高速化を目的とした実装変更は、全 crate 一律の長期安定 API より起こりやすいものとして考えてください。

## 状況

- リポジトリ全体で整形、lint、テストのゲートを維持
- GitHub Actions CI は [`.github/workflows/ci.yml`](./.github/workflows/ci.yml) に設定
- crate ごとに成熟度や更新速度は異なる

## コントリビューション

課題報告と pull request は歓迎します。

広すぎる提案より、対象と目的が絞られた変更の方が検証しやすくなります。

開発フローは [CONTRIBUTING.md](./CONTRIBUTING.md)、脆弱性報告は [SECURITY.md](./SECURITY.md) を参照してください。

## サポート

この crate 群や関連アプリが役に立った場合は、次のページから継続開発を支援できます。

- OFUSE: <https://ofuse.me/barineco>
- Ko-fi: <https://ko-fi.com/barineco>

支援は保守、安定化対応、機能追加の継続に充てます。

## ライセンス

特記がない限り、このリポジトリは [MIT ライセンス](./LICENSE) です。
