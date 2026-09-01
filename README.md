# manga2epub

## これは何？

漫画などの画像を電子書籍（EPUB）にするユーティリティです。

[ビビビッ](https://vivibit.net/)氏の「[NODO epub maker](https://vivibit.net/epub_maker/)」をリスペクトしつつ、macOSでも同様のことができることを目指しています。

## CLI コマンドの使い方

CLI 引数と YAML 設定ファイルの書き方は [CLI_USAGE.md](docs/CLI_USAGE.md) を参照してください。

## マイルストーン

- 完成度
  - [x] CLI として利用可能
  - [ ] GUI として利用可能
  - [ ] README整備
    - `/* この通り、現時点では断片的です */`
  - [ ] 使い方整備
    - CLI は、[CLI_USAGE.md](docs/CLI_USAGE.md) にて記載
- リリースステージ
  - [x] Alpha
  - [ ] Beta
  - [ ] Stable
- リリースプラットフォーム
  - [ ] macOS (Apple Silicon)
    - コード署名等は非対応のため、OS 上の警告が表示される可能性あり
  - 以降、余裕があればほかのプラットフォームにも展開

## ライセンス

Apache License, Version 2.0 または MIT License のいずれかを選択して利用できます。詳細は [LICENSE](LICENSE) を参照してください。
