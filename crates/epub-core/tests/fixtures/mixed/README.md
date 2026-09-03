# JPEG / PNG Mixed fixture

PEG と PNG が混在するダミー画像を用いて検証するための fixture です。
実際の利用を想定して日本語のファイル名を維持しています。

| EPUB 内のページ | ファイル名 | 形式 | 画像サイズ | 配置 | 想定する内容 |
| ---: | --- | --- | ---: | --- | --- |
| 1 | `表紙.png` | PNG | 874 × 1240 | center | 表紙 |
| 2 | `空白(2P用).png` | PNG | 874 × 1240 | right | 導入 |
| 3 | `目次(3P用).png` | PNG | 874 × 1240 | left | 目次ページ |
| 4 | `大きなページ(4,5P用).jpg` | JPEG | 1748 × 1240 | center | 本編の見開き |
| 5 | `通常コンテント(6P用).png` | PNG | 874 × 1240 | right | 本編 |
| 6 | `通常コンテント(7P用).jpg` | JPEG | 874 × 1240 | left | 本編 |
| 7 | `大きなページ(8,9P用).png` | PNG | 1748 × 1240 | center | 本編の見開き |
| 8 | `EOF.png` | PNG | 874 × 1240 | center | 裏表紙 |

見開き画像も分割せず、1 枚の画像から 1 つの XHTML ページを作成します。
このため、画像内に記載されたページ番号と EPUB 内のページ番号は一部で異なっています。

## 自動テスト

リポジトリのルートで以下を実行します。

```shell
cargo test --package epub-cli --test cli_build builds_the_mixed_book_fixture_from_a_yaml_configuration_file
```

テストでは、画像順、画像形式、ページ配置、画像サイズ、階層目次、画像バイト列が維持されることを検証します。
`book.yaml` と画像はテスト用の一時ディレクトリへコピーするため、fixture のディレクトリには EPUB を出力しません。
このテストは、既存の GitHub Actions が実行する Workspace 全体のテストにも含まれます。

## 手動確認

リポジトリのルートで以下を実行します。

```shell
mkdir -p target/acceptance
fixture_run_directory="$(mktemp -d target/acceptance/mixed.XXXXXX)"
cp crates/epub-core/tests/fixtures/mixed/* "$fixture_run_directory/"
cargo run --package epub-cli -- build --config "$fixture_run_directory/book.yaml"
```

生成された `book.epub` は、使用した一時ディレクトリ内にあります。
必要に応じて、EPUBCheck、ビューアーの動作確認などを適宜実施してください。
