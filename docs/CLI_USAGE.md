# manga2epub CLI コマンドの使い方

## EPUB を生成する

画像ディレクトリを直接指定する場合は、次のように実行します。

```bash
manga2epub build ./images --output ./book.epub --title "書籍のタイトル"
```

`./images` 内の JPEG または PNG 画像を自然順で読み込みます。

### CLI 引数一覧

| 引数 | 必須 | 説明 |
| --- | --- | --- |
| `--config <FILE>` | YAML 設定ファイルを使う場合は必須 | `book.yaml` のパス。直接指定の引数とは併用不可 |
| `<image_directory>` | 直接指定時は必須 | ページ画像が入ったディレクトリ |
| `-o`, `--output <PATH>` | 直接指定時は必須 | 生成する EPUB ファイルのパス |
| `--title <TEXT>` | 直接指定時は必須 | 書籍のタイトル |
| `--title-file-as <TEXT>` | 任意 | タイトルの読み |
| `--creator <TEXT>` | 任意 | 著者名 |
| `--creator-file-as <TEXT>` | `--creator` 指定時のみ任意 | 著者名の読み |
| `--creator-role <CODE>` | `--creator` 指定時のみ任意 | 著者の役割。省略時は `aut` |
| `--creator-alternate-script <TEXT>` | 関連する 2 引数と `--creator` 指定時に必須 | 著者名の別表記 |
| `--creator-alternate-script-language <TAG>` | 関連する 2 引数と `--creator` 指定時に必須 | 別表記の言語タグ |
| `--description <TEXT>` | 任意 | 説明文 |
| `--publisher <TEXT>` | 任意 | 発行元 |
| `--language <TAG>` | 任意 | 書籍の言語。省略時は `ja` |
| `--identifier <TEXT>` | 任意 | Primary Identifier。省略時は UUID を生成 |
| `--locale <ja または en>` | 任意 | CLI メッセージの表示ロケール |

現在の直接指定では、著者、役割、別表記はそれぞれ 1 件だけ指定できます。複数件を指定する場合は YAML 設定ファイルを使います。

`--creator-alternate-script` と `--creator-alternate-script-language` は、`--creator` とともに一組で指定します。

## YAML 設定ファイルを使う

```bash
manga2epub build --config ./book.yaml
```

`--config` を指定する場合、画像ディレクトリ、`--output`、`--title`、その他の書誌情報 CLI 引数は同時に指定できません。

### book.yaml の項目一覧

| キー | 必須 | 値 | 説明 |
| --- | --- | --- | --- |
| `version` | 必須 | `1` | 設定ファイルのバージョン |
| `output` | 必須 | パス | 生成する EPUB ファイルのパス |
| `book` | 必須 | マップ | 書誌情報 |
| `book.title` | 必須 | 文字列 | 書籍のタイトル |
| `book.title_file_as` | 任意 | 文字列 | タイトルの読み |
| `book.description` | 任意 | 文字列 | 説明文 |
| `book.publisher` | 任意 | 文字列 | 発行元 |
| `book.language` | 任意 | 言語タグ | 省略時は `ja` |
| `book.identifier` | 任意 | 文字列または `null` | 省略または `null` なら UUID を生成 |
| `book.creators` | 任意 | 配列 | 著者情報。複数指定可能 |
| `book.creators[].name` | 著者ごとに必須 | 文字列 | 著者名 |
| `book.creators[].file_as` | 任意 | 文字列 | 著者名の読み |
| `book.creators[].roles` | 任意 | 文字列の配列 | 著者の役割。省略時は `aut` |
| `book.creators[].alternate_scripts` | 任意 | 配列 | 別表記。複数指定可能 |
| `book.creators[].alternate_scripts[].lang` | 別表記ごとに必須 | 言語タグ | 別表記の言語タグ |
| `book.creators[].alternate_scripts[].value` | 別表記ごとに必須 | 文字列 | 別表記 |
| `images` | 必須 | マップ | 入力画像の設定 |
| `images.directory` | 必須 | パス | ページ画像が入ったディレクトリ |

`output` と `images.directory` の相対パスは、設定ファイル自身の親ディレクトリを基準に解決します。例えば `config/book.yaml` 内の `./images` は `config/images` を指します。

未知のキーはエラーです。現在は `layout`、`pages`、`toc`、画像の明示順序を受け付けません。

### 記述例

```yaml
version: 1
output: "./book.epub"

book:
  title: "書籍のタイトル"
  title_file_as: "ショセキノタイトル"
  language: "ja"
  description: |
    ここに書籍の説明を書く。
  publisher: "Yūtenji Publishers"
  identifier: null

  creators:
    - name: "祐天寺"
      file_as: "ユウテンジ"
      roles:
        - aut
      alternate_scripts:
        - lang: "ja-Kana"
          value: "ユウテンジ"
        - lang: "ja-Latn"
          value: "Yūtenji"

    - name: "編集者"
      roles:
        - edt

images:
  directory: "./images"
```
