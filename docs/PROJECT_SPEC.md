# manga2epub — Project Specification

## 1. 文書の目的

本書は、漫画の各ページをJPEG画像として受け取り、漫画向けFixed Layout EPUBを生成するコンバーター「manga2epub」の初期仕様・設計方針を定義する。

最初にCLIアプリケーションとして実装し、CLIおよびEPUB生成コアが十分に安定した後、同一のコアライブラリを利用するGUIアプリケーションへ発展させる。

本書は以下の用途を兼ねる。

- プロジェクトの要求仕様
- EPUB生成仕様
- Rust実装時の設計指針
- Codex等の生成AIに実装を依頼する際の前提資料
- 将来的なGUI実装時の基礎仕様

仕様が未確定の箇所は「未決定事項」として明示し、実装者が独断で大きな仕様を追加しないこと。

---

# 2. プロジェクト概要

## 2.1 目的

JPEG形式で用意された漫画の各ページを、EPUB 3.3準拠のFixed Layout EPUBとしてパッケージングする。

本ツールの主な責務は以下とする。

- EPUBパッケージ生成
- 書誌メタデータ生成
- Fixed Layout設定
- 漫画向けページ進行方向設定
- ページの左右・中央配置設定
- 表紙設定
- 目次生成
- EPUBコンテナ生成
- EPUB仕様上の妥当性検証

画像編集ツールとしての機能は持たない。

## 2.2 プロジェクト名

プロジェクト名は `manga2epub` とする。

CLIコマンド名も原則として `manga2epub` とする。

## 2.3 リスペクト元

設計思想および用途について、NODO epub makerをリスペクト元の一つとする。

ただし、本プロジェクトでは既存ツールの実装をコピーするのではなく、EPUB 3.3仕様に基づき独自実装する。

---

# 3. 対象環境

## 3.1 必須OS

macOSで動作すること。

## 3.2 クロスプラットフォーム

実装上大きな追加コストが発生しない場合は、以下も動作対象とする。

- Windows
- Linux

macOS対応を犠牲にしてクロスプラットフォーム化しないこと。

---

# 4. 技術選定

## 4.1 言語

第一候補としてRustを採用する。

Rustについてプロジェクトオーナーは現時点で実務知識を持っていないため、実装ではRust固有の慣習を尊重しつつ、過度に技巧的なコードを書かないこと。

## 4.2 Rustを採用する理由

本プロジェクトでは以下を重視する。

- CLIアプリケーションを単一実行ファイルとして配布しやすい
- macOSで自然に動作する
- Windows/Linuxへの展開が比較的容易
- XML、ZIP、ファイルI/Oを安全に扱える
- CLIと将来のGUIから共通利用できるライブラリを作りやすい
- 型によって不正な状態を排除しやすい
- テスト可能なコアライブラリを構築しやすい

## 4.3 将来のGUI

GUIフレームワークの第一候補としてTauriを想定する。

ただしGUIは初期開発の対象外とする。

GUI化のためにEPUB生成処理を書き直すことがないよう、EPUB生成処理をUI層から分離する。

将来のGUIでは、画像を1ファイル単位で並べ替える操作を提供できることが望ましい。特に、ファイル名順ではなくユーザーが明示的にページ順を指定する用途を想定する。

---

# 5. Rustでの設計方針

## 5.1 コアとUIを分離する

EPUB生成ロジックをCLIの `main()` 内に直接実装しない。

概念上、以下の構造とする。

```text
CLI
  │
  ▼
epub-core
  ▲
  │
GUI
```

CLIと将来のGUIは、ともに同一の `epub-core` を利用する。

## 5.2 型で状態を表現する

例えばページ配置は文字列のままコアへ渡さず、enumとして表現する。

概念例：

```rust
enum PagePlacement {
    Left,
    Right,
    Center,
}
```

読書方向についても同様に、

```rust
enum ReadingDirection {
    Rtl,
    Ltr,
}
```

など、取り得る値を型として限定する。

## 5.3 Rustの基本文化として重視する事項

- `cargo fmt` で整形できるコードを書く
- `cargo clippy` を継続的に通す
- 通常のエラー処理に `panic!` を使わない
- CLI入力エラー、ファイルI/Oエラー、EPUB生成エラーを適切に `Result` で伝播する
- 不必要な `unwrap()` / `expect()` を避ける
- public APIは必要最小限にする
- 過度なジェネリクスや抽象化を避ける
- 最初から巨大な設計にしない
- テスト可能な小さな関数へ分割する

プロジェクトオーナーがRustを学べることも意識し、Rust特有の実装を採用する場合は理由が理解可能な構造にする。

---

# 6. EPUB仕様

## 6.1 対象バージョン

EPUB 3.3をターゲットとする。

OPFのpackage要素はEPUB 3.x仕様に従い、

```xml
<package version="3.0" ...>
```

とする。

`version="3.3"` にはしない。

## 6.2 レイアウト

漫画はFixed Layoutとして生成する。

```xml
<meta property="rendition:layout">pre-paginated</meta>
```

を使用する。

## 6.3 ページ進行方向

デフォルトは日本漫画向けの右綴じとする。

```xml
<spine page-progression-direction="rtl">
```

将来、設定によりLTRも指定可能とする。

## 6.4 Synthetic Spread

標準のデフォルト値は `landscape` とする。

```xml
<meta property="rendition:spread">landscape</meta>
```

これにより、横画面などSynthetic Spreadを利用できる環境では見開きを構成できるようにする。

設定値として将来的に以下を扱える構造とする。

```text
auto
none
landscape
both
```

初期CLIでは `landscape` を標準値とし、他の値を公開するかどうかは実装時に判断する。

## 6.5 Orientation

EPUB 3.3標準の、

```xml
<meta property="rendition:orientation">...</meta>
```

は必要に応じて利用可能とする。

ただしデフォルトではReading System側へ過度な制約を課さない。

原則として、

```text
auto
```

または指定自体を省略する。

---

# 7. 表紙

入力画像の1ページ目を自動的に表紙とする。

例えば入力画像が、

```text
image-0000.jpg
image-0001.jpg
image-0002.jpg
...
```

の場合、

```text
image-0000.jpg
```

を表紙画像とする。

manifestでは、

```xml
<item
    id="cover"
    href="images/image-0000.jpg"
    media-type="image/jpeg"
    properties="cover-image"/>
```

のように設定する。

EPUB内部のファイル名は入力元のファイル名を維持する必要はない。初期実装では、EPUB内部で正規化したファイル名を使用する。

表紙画像は同時に漫画本文の第1ページとして扱う。

すなわち、

```text
表紙画像
=
EPUB metadata上のcover-image
=
spine上の第1ページ
```

とする。

---

# 8. ページ配置

## 8.1 内部表現

ページ配置は少なくとも以下の3種類を持つ。

```text
left
right
center
```

意味は以下の通り。

### left

Synthetic Spread上の左ページ。

### right

Synthetic Spread上の右ページ。

### center

1画像で単独の見開き相当ページを構成する。

通常の左右ページとは異なり、Synthetic Spread上で単独中央表示させる。

## 8.2 EPUBへのマッピング

ページ配置プロパティは、パッケージ文書のspineにある対応する`itemref`へ
マッピングする。manifestの`item`には書かない。

基本出力では、以下のように`rendition:`付き表記を使用する。

```xml
<itemref idref="page-0000" properties="rendition:page-spread-left"/>
```

```xml
<itemref idref="page-0001" properties="rendition:page-spread-right"/>
```

```xml
<itemref idref="page-0002" properties="rendition:page-spread-center"/>
```

EPUB 3.3向けの内部表現および基本出力では、`rendition:` 付き表記へ統一する。

古いReading Systemとの互換性が必要な場合に限り、left/rightについて、

```xml
<itemref idref="page-0000" properties="rendition:page-spread-left page-spread-left"/>
```

のような互換出力を可能にする余地を残す。

初期バージョンで互換出力を実装することは必須ではない。

---

# 9. デフォルトページ配置

## 9.1 基本ルール

デフォルトでは以下とする。

```text
Page 1 = center + cover

Page 2 = right
Page 3 = left
Page 4 = right
Page 5 = left
...
```

すなわち1ページ目を除き、

```text
right
left
right
left
...
```

と交互に配置する。

## 9.2 override

利用者がページ配置を指定した場合、そのページのみ自動判定を上書きする。

例えば、

```text
Page 1 = center
Page 2 = right
Page 3 = left
Page 4 = center  ← override
Page 5 = left    ← 通常の自動判定
Page 6 = right
```

のように、overrideによって後続ページの自動計算をシフトさせない。

後続ページも含めた特殊なページ構成が必要な場合は、利用者が必要なページをすべて指定する。

## 9.3 フルカスタマイズ

全ページについて配置を指定することも可能にする。

「一部のみoverride」と「全ページ指定」で別々の機構を作らず、同一のoverride機構で表現できることが望ましい。

例えば100ページすべてをoverrideとして指定すれば、結果として完全手動指定になる。

---

# 10. 入力画像

## 10.1 初期対応形式

JPEGを必須対応形式とする。

最低限、

```text
.jpg
.jpeg
```

を扱えること。

拡張子の大文字小文字についてはmacOS/Windows間の互換性を考慮する。

## 10.2 画像処理を行わない

本ツールは入力JPEGについて以下を行わない。

- リサイズ
- 再圧縮
- JPEG品質変更
- トリミング
- 余白除去
- 色補正
- カラープロファイル変換
- 自動回転
- アスペクト比補正

EPUB内へ格納するJPEGのバイト列は、原則として入力ファイルと同一とする。

## 10.3 画像情報の参照

EPUB生成に必要な範囲でJPEGの以下の情報を読み取ることは許容する。

- width
- height
- 必要な基本属性

これは画像加工には含めない。

## 10.4 ページサイズ

Fixed Layout XHTMLにはviewport寸法が必要となる。

漫画全体で共通の論理viewportを使用する方針とする。

第一候補として、先頭画像のwidth/heightを基準viewportとして使用する。

例：

```html
<meta
    name="viewport"
    content="width=1200, height=1759"/>
```

画像そのものは加工せず、XHTML/CSS上で論理viewport内に表示する。

入力画像の縦横比が漫画内で統一されていることを前提とする。

縦横比の異なる画像を投入した場合でも、原則としてEPUB生成を失敗させない。ただし、他の画像と明らかに縦横比が異なる場合はWARNINGを表示する。

数ピクセル程度の差を警告対象とするかどうかは、画像サイズ、縦横比、許容誤差の扱いを含めて実装時に決定する。

center配置の画像については、見開き相当の画像である可能性があるため、通常ページと異なるサイズや縦横比であっても、直ちにWARNINGとするとは限らない。この扱いは実装時に明確化する。

いずれの場合も、縦横比不一致は少なくとも初期仕様ではERRORとしない。

---

# 11. XHTMLページ

各画像について1つのXHTML Content Documentを生成する。

概念的には、

```text
image-0000.jpg
→ page-0000.xhtml

image-0001.jpg
→ page-0001.xhtml
```

とする。

XHTMLはFixed Layout用viewportを持ち、対応するJPEG画像を1枚表示する。

画像はページ全面へ表示し、不要な余白、padding、marginを持たせない。

画像の再エンコードは行わない。

EPUB内部の画像ファイル名およびXHTMLファイル名は、入力元のファイル名に依存せず、決定論的な正規化名を使用してよい。

---

# 12. メタデータ

## 12.1 タイトル

必須項目とする。

例：

```xml
<dc:title id="title">同人誌のタイトル</dc:title>
```

## 12.2 タイトル読み

タイトルの読みを指定可能とする。

読みはカタカナを想定する。

例：

```xml
<meta
    property="file-as"
    refines="#title">ドウシンシノタイトル</meta>
```

`file-as` は仕様上「ソート等に利用する正規化表現」であるため、内部データモデルでは単純に「ruby」などと命名せず、以下いずれかのように意味を分離する。

```text
title
title_file_as
```

or

```text
display
file_as
```

GUI上では「タイトル読み」と表示してもよい。

## 12.3 著者

著者は `dc:creator` で表現する。

例：

```xml
<dc:creator id="creator1">祐天寺</dc:creator>
```

著者の役割として、以下を設定可能とする。

```xml
<meta
    property="role"
    refines="#creator1"
    scheme="marc:relators">aut</meta>
```

漫画では作者が作画も行う場合があるため、将来的には複数 role を許容できるデータモデルが望ましい。

例えば、`aut`、`ill` を同一 creator へ設定可能にする余地を残す。

初期実装では、著者は任意の 1 名とし、役割を省略した場合は `aut` を使用する。

複数著者・複数 role は、内部データモデルと CLI の利用例を確定してから追加する。

## 12.4 著者読み

カタカナの読みを `file-as` として指定可能とする。

例：

```xml
<meta
    property="file-as"
    refines="#creator1">ユウテンジ</meta>
```

## 12.5 alternate-script

必要に応じて著者名等へ別script表現を設定可能とする。

カタカナ表現には例えば、以下を検討する。

```xml
<meta
    property="alternate-script"
    refines="#creator1"
    xml:lang="ja-Kana">ユウテンジ</meta>
```

ローマ字転写の場合は、以下のような表現を第一候補とする。

```xml
<meta
    property="alternate-script"
    refines="#creator1"
    xml:lang="ja-Latn">Yūtenji</meta>
```

- `alternate-script` は必須ではない。
- `file-as` と `alternate-script` は別用途なので、内部データモデルでも区別する。
- 初期実装では著者名にのみ対応し、値を指定する場合は `xml:lang` に対応する言語タグも指定する。

## 12.6 Description

Description を任意指定可能とする。

例：

```xml
<dc:description>ここで指定した値がPlay Booksで表示されることを期待する。</dc:description>
```

Google Play Books での表示を主要な利用目的の 1 つとする。

実際の表示についてはビューアー側の仕組み および Google側の実装に依存するため、結合テストで確認する。

## 12.7 Publisher

任意指定とする。

例：

```xml
<dc:publisher>Yūtenji Publishers</dc:publisher>
```

## 12.8 Language

必須項目とする。

デフォルト：

```xml
<dc:language>ja</dc:language>
```

CLIオプションから変更可能にする。

設定ファイル導入後も、同じ意味の値を指定可能にする。

## 12.9 Identifier

利用者による指定を可能とする。

指定された場合は、その値を Primary Identifier として利用する。

指定がない場合は UUID を自動生成する。

自動生成例：

```xml
<dc:identifier id="pub-id">
    urn:uuid:12345678-abcd-1234-ef00-123456789abc
</dc:identifier>
```

`package` は、

```xml
<package
    ...
    unique-identifier="pub-id">
```

とする。

利用者が任意 ID を設定した場合も、EPUB として一意な Identifier となる文字列として扱う。

URI/URN の組み立て方については、設定値をそのまま利用する方式と、ツール側で `urn:` を付加する方式を混在させない。

CLI オプションおよび将来の設定ファイルでは、指定値をそのまま利用する。
ツール側で `urn:` などの接頭辞を補わない。

## 12.10 modified

EPUB 3.3 で必要な、

```xml
<meta property="dcterms:modified">
    ...
</meta>
```

を生成する。

日時は EPUB 生成時の UTC 時刻から生成する。
値は秒精度の `YYYY-MM-DDThh:mm:ssZ` 形式とし、1 秒未満の端数を含めない。

再現可能ビルドを将来必要とする場合は、明示指定可能にする余地を残す。

---

# 13. 旧EPUB 3.0時代の互換メタデータ

過去に利用していた以下のようなメタデータは、EPUB 3.3標準出力では原則として生成しない。

```xml
<meta content="true" name="fixed-layout"/>
<meta content="none" name="orientation-lock"/>
<meta content="1200x1759" name="original-resolution"/>
<meta content="comic" name="book-type"/>
<meta content="horizontal-rl" name="primary-writing-mode"/>
<meta content="#ffffff" name="SpineColor"/>
<meta name="cover" content="cover"/>
```

理由は、これらがEPUB 3.3 CoreにおけるFixed Layoutの基本表現ではないためである。

代わりに標準出力では、

```xml
<meta property="rendition:layout">pre-paginated</meta>
<meta property="rendition:spread">landscape</meta>
```

各XHTMLの、

```html
<meta name="viewport" content="width=..., height=..."/>
```

manifestの、

```xml
properties="cover-image"
```

およびspineの、

```xml
page-progression-direction="rtl"
```

を利用する。

将来的に特定Reading System向け互換性が必要と判明した場合のみ、

```text
compatibility profile
```

として追加する。

ベンダー固有メタデータを標準動作へ混在させないこと。

---

# 14. Prefix

OPFのpackage要素では、相互運用性を優先して `rendition` prefixを明示的に宣言する方針とする。

概念例：

```xml
<package
    xmlns="http://www.idpf.org/2007/opf"
    xmlns:dc="http://purl.org/dc/elements/1.1/"
    version="3.0"
    unique-identifier="pub-id"
    prefix="rendition: http://www.idpf.org/vocab/rendition/#">
```

必要に応じて他のprefixも同様に扱う。

---

# 15. 目次

## 15.1 EPUB Navigation Document

EPUB 3.3では `nav.xhtml` を正式な目次として生成する。

概念例：

```html
<nav epub:type="toc">
    <ol>
        <li>
            <a href="pages/page-0002.xhtml">第1話</a>
        </li>
        <li>
            <a href="pages/page-0027.xhtml">第2話</a>
        </li>
    </ol>
</nav>
```

manifestでは `nav` propertyを付与する。

## 15.2 NCX

`toc.ncx` はEPUB 3.3の標準目次としては使用しない。

したがってデフォルトでは、

```xml
<spine toc="ncx">
```

も生成しない。

将来的に古いReading Systemとの互換性を必要とした場合、

```text
compatibility.ncx = true
```

のような互換オプションで生成可能にしてよい。

NCXを生成する場合は、その `dtb:uid` とEPUBのPrimary Identifierを同期させる。

---

# 16. EPUBファイル構造

第一候補として以下の構造を採用する。

```text
/
├── mimetype
├── META-INF/
│   └── container.xml
└── EPUB/
    ├── package.opf
    ├── nav.xhtml
    ├── styles/
    │   └── page.css
    ├── pages/
    │   ├── page-0000.xhtml
    │   ├── page-0001.xhtml
    │   └── ...
    └── images/
        ├── image-0000.jpg
        ├── image-0001.jpg
        └── ...
```

EPUB内部のファイル名は入力元のファイル名を維持せず、正規化した連番形式を使用する。

元ファイルそのもののバイト列は変更しない。

---

# 17. ZIP / OCF

EPUBは単なる任意のZIPではなく、EPUB OCFとして正しく生成する。

特に `mimetype` は、

```text
application/epub+zip
```

のみを内容とする。

以下を厳守する。

```text
mimetype
```

はZIPの最初のentryとする。

`mimetype` は圧縮しない。

不要なBOM、改行、前後空白を入れない。

他のファイルは必要に応じてDeflate圧縮する。

---

# 18. 設定ファイル

## 18.1 形式

YAMLを採用する。

ファイル名のデフォルトは、

```text
book.yaml
```

とする。

`book.yaml` は推奨する標準ファイル名であり、CLIが現在のディレクトリから自動検出することはしない。
YAMLを使う場合は、利用者が設定ファイルへのパスを明示する。

## 18.2 方針

CLI引数だけで全書誌情報・全ページ指定を行わせない。

CLIは簡単な操作に利用し、複雑な書籍設定はYAMLへ記述する。

GUI化した場合も、GUIの内部データモデルとYAMLを極力共通化する。

---

# 19. book.yaml 初期案

以下のスキーマで進める。

```yaml
version: 1

book:
  title: "同人誌のタイトル"
  title_file_as: "ドウシンシノタイトル"

  language: "ja"

  description: |
    ここに書籍の説明を書く。

  publisher: "Yūtenji Publishers"

  # 未指定なら UUID を生成
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

layout:
  direction: rtl
  spread: landscape
  orientation: auto

images:
  directory: "./images"

pages:
  cover: first

  auto:
    first_after_cover: right
    alternate: true

  overrides:
    - page: 4
      placement: center

    - page: 12
      placement: left

    - page: 13
      placement: right

toc:
  - title: "第1話"
    page: 2

  - title: "第2話"
    page: 24

  - title: "あとがき"
    page: 50
```

`layout.spread` の標準値は `landscape` とする。

---

# 20. ページ指定スキーマの考え方

`overrides` は疎な指定と完全指定の両方に利用する。

例えば通常は、

```yaml
pages:
  overrides:
    - page: 24
      placement: center
```

だけ指定する。

完全に手動指定したい場合は、

```yaml
pages:
  overrides:
    - page: 1
      placement: center
    - page: 2
      placement: right
    - page: 3
      placement: left
    - page: 4
      placement: center
    - page: 5
      placement: right
    ...
```

のように全ページを指定する。

これにより、

```text
partial customization
```

と、

```text
full customization
```

で別々の仕様を作らない。

## 20.1 page番号

YAML上の `page` は **1-origin** とする。

つまり、

```yaml
page: 1
```

は第1画像、すなわち表紙を指す。

内部Rust実装では0-originでも構わない。

## 20.2 ページ順序の指定

画像の取り込み順は、以下の2種類を扱える設計とする。

### デフォルト

ユーザーによる明示指定がない場合、入力ディレクトリ内のJPEGをファイル名の昇順で読み込む。

数値部分は自然順で比較する。

例えば、

```text
page-1.jpg
page-2.jpg
page-10.jpg
```

は、

```text
page-1.jpg
page-2.jpg
page-10.jpg
```

の順とする。

単純な辞書順によって `page-10.jpg` が `page-2.jpg` より前になることを避ける。

### 明示指定

ユーザーが1ファイル単位で順番を指定する場合は、その指定を優先する。

CLI/YAMLでは、将来的に以下のような形式を扱える構造を想定する。

```yaml
images:
  directory: "./images"

  order:
    - "cover.jpg"
    - "page-01.jpg"
    - "page-02.jpg"
```

または、GUIで管理するページ順序をYAMLへ保存できる構造を想定する。

具体的な最終スキーマは、自然順ソートおよびGUIのデータモデルと合わせて確定する。

明示指定がある場合、指定されていないファイルを自動的に末尾へ追加するか、未指定ファイルをエラーとするかは実装時に決定する。

---

# 21. CLI

実行ファイル名およびCLIコマンド名は `manga2epub` とする。

## 21.1 EPUB生成

```bash
manga2epub build ./images --output ./book.epub --title "同人誌のタイトル"
```

`<image_directory>`、`--output`、`--title` を指定して EPUB を生成する。

画像ディレクトリ内の JPEG 画像を自然順でページとして扱う。

## 21.2 メタデータ指定

タイトル以外のメタデータは、必要に応じて CLI オプションで指定する。

```bash
manga2epub build ./images \\
  --output ./book.epub \\
  --title "同人誌のタイトル" \\
  --title-file-as "ドウジンシノタイトル" \\
  --creator "著者名" \\
  --creator-file-as "チョシャメイ" \\
  --creator-role aut \\
  --creator-alternate-script "チョシャメイ" \\
  --creator-alternate-script-language ja-Kana \\
  --description "紹介文" \\
  --publisher "発行元" \\
  --language ja \\
  --identifier "urn:uuid:12345678-abcd-1234-ef00-123456789abc"
```

- `--language` の既定値は `ja` とする。
- `--identifier` を省略した場合は `urn:uuid:` 形式のUUIDを自動生成する。
- `--creator-role` を省略した場合は `aut` とする。
- `--creator-alternate-script` を指定する場合は、
- `--creator-alternate-script-language` も指定する。

## 21.3 設定ファイル指定

設定ファイルは、CLIオプションと同じメタデータを繰り返し指定する負担を減らすために導入する。

```bash
manga2epub build ./book.yaml
```

設定ファイルを導入するまでは、この形式を受け付けない。

## 21.4 初期設定生成

将来的に、

```bash
manga2epub init
```

で雛形 `book.yaml` を生成できると便利である。

## 21.5 検査

将来的に、以下のような形を提供してよい。

```bash
manga2epub check ./book.epub
```

ただし EPUBCheck そのものを Rust で再実装しない。

## 21.6 inspect

将来的に、

```bash
manga2epub inspect ./book.epub
```

で、以下のような情報を表示できると便利である。

```text
Title: 同人誌のタイトル
Creator: 祐天寺
Pages: 52
Direction: RTL
Layout: Fixed
Spread: Landscape
Cover: image-0000.jpg
TOC entries: 3
```

初期リリース必須機能ではない。

## 21.7 既存 EPUB 編集

将来的に、

```bash
manga2epub edit ./book.epub
```

のような既存 EPUB 編集機能を提供できると便利である。

ただし、これは最低優先度の機能とする。
初期段階では、既存 EPUB の読み込み・編集・再パッケージングを実装しない。

## 21.8 表示ロケール

CLI の利用者向けメッセージは、日本語と英語を切り替えられるようにする。

```bash
manga2epub --locale ja build ./images --output ./book.epub
manga2epub --locale en build ./images --output ./book.epub
```

表示ロケールは、以下の優先順位で決定する。

1. `--locale` で明示された言語
2. OS から取得した実行ロケールのうち、対応している言語
3. 英語

OS から取得した言語に対応していない場合も英語を使用する。

翻訳文は CLI crate のロケールファイルで管理し、単一実行ファイルへ埋め込む。
EPUB 生成コアはロケールを扱わず、構造化したエラーを返す。
CLI はそのエラーを利用者向けメッセージへ翻訳する。

日本語の利用者向けメッセージでは、英数字と日本語の間に半角スペースを入れる。

表示ロケールは、EPUB メタデータの `language` とは独立した設定とする。

---

# 22. Rustプロジェクト構造

初期段階からCargo Workspaceを利用する案を第一候補とする。

```text
/
├── Cargo.toml
├── crates/
│   ├── epub-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs
│   │
│   └── epub-cli/
│       ├── Cargo.toml
│       └── src/
│           └── main.rs
│
├── tests/
│   └── fixtures/
│
├── docs/
│   └── PROJECT_SPEC.md
│
└── README.md
```

GUI実装時に、

```text
crates/
├── epub-core/
├── epub-cli/
└── epub-gui/
```

へ拡張する。

---

# 23. epub-core の責務

`epub-core` はUIを一切知らない。

少なくとも以下を担当する。

```text
BookConfig
Metadata
Creator
Page
PagePlacement
ReadingDirection
SpreadMode
TOC
Image ordering
Natural sort
Explicit page ordering
EPUB package generation
OPF generation
XHTML generation
Navigation Document generation
OCF/ZIP generation
validation
warning generation
```

CLI引数解析は担当しない。

Tauriも依存させない。

---

# 24. epub-cli の責務

CLIは以下のみを担当する。

- コマンドライン引数解析
- CLI オプションからの入力値の組み立て
- YAML 読み込み（設定ファイル入力を提供する場合）
- ユーザー向けエラー表示
- WARNING表示
- `epub-core` 呼び出し
- 終了コード

EPUB XMLをCLI側で直接組み立てない。

---

# 25. XML生成

XMLを大量の文字列連結で生成しない。

適切なXMLライブラリを利用する。

特に以下を正しくescapeする。

```text
&
<
>
"
'
```

タイトル、著者名、Description等へ任意のUnicode文字列が入っても、正常なXMLを生成する。

UTF-8を前提とする。

---

# 26. ファイル順序

画像ファイルの読み込み順は決定論的でなければならない。

ファイルシステムが返した順序をそのまま利用しない。

## 26.1 デフォルト順序

ユーザーによる明示指定がない場合、ファイル名の昇順で読み込む。

数値部分は自然順で比較する。

例えば、

```text
image-1.jpg
image-2.jpg
image-10.jpg
```

は、

```text
image-1.jpg
image-2.jpg
image-10.jpg
```

の順とする。

ゼロ埋めされたファイル名も引き続き推奨する。

```text
image-0000.jpg
image-0001.jpg
image-0002.jpg
```

## 26.2 明示的な順序指定

ユーザーが 1 ファイル単位で順番を指定した場合は、その順序を優先する。

この機能は CLI/YAML で利用可能な設計とし、将来のGUIでは特に重要な機能として扱う。

GUI では、画像のドラッグ＆ドロップ等によってページ順を変更し、その結果を内部データモデルおよび YAML へ保存できることが望ましい。

明示指定時に、指定されていない画像をどう扱うかは以下のいずれかとする。

- 未指定画像をエラーとする
- 未指定画像を自動順序で末尾へ追加する

この選択は、最終的な YAML スキーマ確定時に決定する。

## 26.3 EPUB 内部名

EPUB 内部では、入力元のファイル名を維持する必要はない。

ページ順に基づく正規化名を使用する。下記のような形式を第一候補とする。

```text
images/image-0000.jpg
images/image-0001.jpg
images/image-0002.jpg
```

---

# 27. エラーと警告

## エラーにすべき例

- 入力画像が0枚
- book titleが未指定
- 不正なYAML
- 存在しないページをTOCが参照
- 存在しないページをoverrideが参照
- 同一ページへ矛盾したplacement指定
- 明示された画像ファイルが存在しない
- 明示された画像順序に同一ファイルが重複している
- EPUB生成先へ書き込めない
- JPEGとして読み取れない入力

## 警告候補

- 画像の縦横比が他ページと異なる
- 画像サイズが極端に異なる
- Description未指定
- Publisher未指定
- TOCが空
- alternate-scriptのlanguage tagが不自然
- 明示順序に含まれない画像が自動的に追加された
- 画像の縦横比が基準画像から明らかに異なる

## 27.1 縦横比不一致の扱い

JPEGの縦横比不一致はWARNINGとする。

少なくとも初期仕様では、縦横比不一致をERRORにしない。

例えば、最初の画像群が概ね1:1.4であるにもかかわらず、後続画像に正方形画像が含まれる場合はWARNINGを表示する。

一方で、数ピクセル程度の差を警告対象とするかどうかは慎重に扱う。

以下を実装時の検討事項とする。

- ピクセル寸法の差ではなく、縦横比の差で判定する
- 許容誤差を絶対値または相対値で定義する
- 画像サイズが小さい場合と大きい場合で同じ閾値を使用するか検討する
- center配置の画像を通常ページと同じ基準で判定するか検討する

center配置の画像は見開き相当ページである可能性があるため、通常ページと異なる縦横比であっても、必ずWARNINGとするとは限らない。

警告判定は、ページ配置、基準viewport、画像サイズ、縦横比を考慮して設計する。

警告はEPUB生成を必ずしも失敗させない。

---

# 28. テスト方針

## 28.1 Unit Test

少なくとも以下をテストする。

```text
Page 1 -> Center
Page 2 -> Right
Page 3 -> Left
Page 4 -> Right
```

overrideについて、

```text
Page 4 -> Center
```

が指定された場合、

```text
Page 5
```

の自動配置が影響を受けないことを確認する。

自然順ソートについて、

```text
page-1.jpg
page-2.jpg
page-10.jpg
```

が正しい順序になることを確認する。

明示的なファイル順序指定について、指定順がデフォルトの自然順より優先されることを確認する。

## 28.2 XMLテスト

生成されたOPFについて以下を確認する。

- title
- creator
- creator role
- file-as
- alternate-script
- description
- publisher
- identifier
- modified
- cover-image
- rendition
- rendition
- page-progression-direction
- itemref順序
- page placement

## 28.3 Navigation Test

`nav.xhtml` の目次項目が正しいXHTMLへリンクすること。

## 28.4 JPEG無加工テスト

重要な品質条件として、入力JPEGとEPUB内JPEGのSHA-256を比較する。

```text
SHA256(input JPEG)
==
SHA256(JPEG extracted from EPUB)
```

となること。

これにより、本ツールがJPEGを再圧縮・改変していないことを自動テストする。

## 28.5 Warning Test

以下をテストする。

- 明らかに縦横比が異なる画像でWARNINGが発生する
- 縦横比不一致によってERRORにならない
- center配置の画像について、通常ページと異なる扱いができる
- 許容誤差内の差をWARNINGとするかどうかが、定義した閾値に従う

## 28.6 EPUBCheck

生成されたEPUBをEPUBCheckで検証する。

プロジェクトの品質基準として、

```text
EPUBCheck error = 0
```

を目標とする。

warningについては内容を確認し、プロジェクトとして許容するものを明示する。

## 28.7 Reading System Test

少なくとも以下で実機確認する。

- Apple Books on macOS
- Google Play Books

可能であれば、

- Google Play Books Android
- その他EPUB 3対応Reading System

でも確認する。

Reading Systemごとの差異はEPUB仕様上の問題と個別実装の問題を分けて記録する。

---

# 29. 開発フェーズ

## Phase 1 — 最小EPUB

以下だけを実装する。

```text
JPEG directory
↓
EPUB 3.3 Fixed Layout
```

固定仕様：

```text
JPEG only
natural filename order
RTL
Page 1 = cover + center
Page 2 = right
Page 3 = left
...
```

まずEPUBCheckおよびReading Systemで正しく開けることを確認する。

## Phase 2 — Metadata

以下を追加する。

- title
- title file-as
- creator
- creator file-as
- creator role
- alternate-script
- description
- publisher
- language
- identifier
- UUID auto generation

## Phase 3 — YAML

`book.yaml` を実装する。

## Phase 4 — Page customization

ページoverrideを実装する。

```text
left
right
center
```

を扱う。

## Phase 5 — Explicit page ordering

ユーザーが1ファイル単位でページ順を指定できる機能を実装する。

CLI/YAMLで利用可能にし、将来のGUIで扱いやすい内部データモデルを確立する。

## Phase 6 — TOC

`nav.xhtml` を生成する。

## Phase 7 — CLI完成度向上

必要に応じて、

```text
init
build
check
inspect
```

等を追加する。

## Phase 8 — GUI

`epub-core` をそのまま利用してGUIを構築する。

GUIでEPUB生成ロジックを再実装しない。

## Phase 9 — 既存EPUB編集

最低優先度の将来機能として、既存EPUBの読み込み、編集、再パッケージングを検討する。

---

# 30. 初期段階で実装しない機能

以下はスコープ外とする。

- JPEG自動縮小
- JPEG圧縮率変更
- 画像フォーマット変換
- 自動トリミング
- 自動余白除去
- 自動色補正
- 自動ページ分割
- OCR
- PDF変換
- CBZ変換
- DRM
- 電子署名
- EPUB 2専用出力
- GUI
- 既存EPUB編集
- クラウドサービス
- 書籍販売サイトへの自動アップロード

要望がない限り勝手に追加しない。

---

# 31. Compatibility Profile

将来的にReading System固有の互換処理が必要になった場合、コアのEPUB 3.3出力へ直接混ぜず、

```text
compatibility profile
```

として分離する。

候補：

```text
standard
legacy
apple
google-play
```

ただし、実際に必要性が確認されるまでprofile自体を実装しない。

特定ベンダー向けタグを「念のため」で大量に出力しない。

---

# 32. Codex等、生成AIへの実装指示

Codex等の各種生成AI（以降32章では「あなた」と呼びます）は本書をプロジェクト仕様の基準として扱うこと。

## 32.1 実装前

大きな機能を実装する前に、

```text
要求
↓
EPUB仕様
↓
Rustデータモデル
↓
実装
```

の順に考える。

不明点を独自仕様で埋めない。

特に以下は、実装前に仕様を確認する。

- 自然順ソートの定義
- 明示的な画像順序指定
- 縦横比WARNINGの閾値
- center配置画像の警告扱い
- EPUB内部ファイル名
- 既存EPUB編集機能の優先度

## 32.2 Rust初心者向け配慮

プロジェクトオーナーはRust初心者である。

そのため、

- Rustとして非idiomaticな実装にはしない
- ただし必要以上に高度なRust機能を利用しない
- lifetimeを不必要に複雑化しない
- unsafeを原則使用しない
- マクロを乱用しない
- 過剰な抽象化を避ける
- なぜその型・crate・設計を使用するのか説明可能にする

## 32.3 依存crate

crate追加時は、

- 何のための依存か
- 標準ライブラリでは不十分な理由
- メンテナンス状況
- ライセンス
- 依存関係の規模

を考慮する。

闇雲にcrateを増やさない。

## 32.4 品質

変更後は原則として以下を通す。

```text
cargo fmt
cargo clippy
cargo test
```

EPUB生成機能に変更が入った場合は可能な範囲でEPUBCheckも実施する。

## 32.5 コミット

一つのコミットへ無関係な変更を混在させない。

フォーマット変更だけで大量差分を発生させない。

## 32.6 プロジェクト進行

あなたは、個別に指示された実装だけを受動的に行うのではなく、本プロジェクトが初期完成条件まで迷走せず進むよう、次のステップを継続的に提案する。

ただし、ユーザーの明示的な指示なしに次の大きな実装フェーズへ進まない。

各作業ターンでは原則として以下の流れとする。

1. 今回の作業範囲を確認する。
2. `docs/PROJECT_SPEC.md` (このドキュメント)と現在の実装状態を照合する。
3. 指定された範囲を実装または調査する。
4. 必要なテスト・lint・検証を実施する。
5. 実施内容と設計上の判断を説明する。
6. 現在のプロジェクト進捗を簡潔に整理する。
7. 次に行うべき作業を1〜3個程度提示する。
8. その中から、次の1ターンとして最も適切な作業を推奨する。

次のステップを提案する際は、単に「次のPhaseへ進む」のではなく、現在の実装状態、依存関係、テスト可能性、仕様上の未決定事項を考慮する。

例えば、次の機能を実装する前にデータモデルや仕様を確定した方がよい場合は、実装ではなく設計確認を次のステップとして提案する。

### 32.6.1 次ステップ提案の形式

各ターンの最後に、概ね以下を示す。

```text
Current status:
- 今回完了したこと
- 現在到達しているPhaseまたは実装状態

Recommended next step:
- 次に推奨する作業
- その作業を次に行う理由
- 想定する作業範囲

Later:
- その後に予定される主要作業
```

厳密にこの書式へ固定する必要はないが、ユーザーが次に何を依頼すべきか自分で再設計しなくてもよい状態にする。

### 32.6.2 作業粒度

一度にプロジェクト全体を実装しない。

各ターンの変更は、ユーザーがコードと設計意図をレビューできる程度の大きさに保つ。

目安として、以下は適切な単位である。

- Cargo Workspaceの初期構築
- 基本データモデルの追加
- JPEG列挙と自然順ソート
- ページ配置ロジックとテスト
- OCF/ZIP生成
- OPF生成
- XHTML生成
- Navigation Document生成
- YAML設定読み込み
- CLI commandの追加

一方、以下のように複数の大きな責務を一度に実装することは原則として避ける。

```text
EPUB生成
+ YAML
+ Metadata
+ TOC
+ CLI完成
+ GUI
```

作業を分割する際は、単にコード量だけでなく、各変更が独立してテスト・レビュー可能かを重視する。

### 32.6.3 ユーザーに求める判断

あなた自身で合理的に決められる実装詳細について、毎回ユーザーへ判断を求めない。

例えば以下は、既存仕様とRustの一般的な慣習から明確に判断できるのであれば、あなた自身で提案または実装してよい。

- private関数名
- モジュール分割
- テスト関数名
- ローカル変数名
- 自明なエラー型の構成
- formatterによる整形

一方、以下のようなプロジェクトの挙動・互換性・公開インターフェースへ影響する事項は、仕様書に答えがない場合、独断で確定しない。

- YAMLスキーマ
- CLI UX
- EPUB出力仕様
- Reading System互換処理
- warning/errorの境界
- public API
- 新しい大規模依存crate
- 初期スコープ外機能の追加

その場合は、推奨案と理由を提示し、ユーザーが判断できるようにする。

### 32.6.4 プロジェクトオーナーの負荷軽減

プロジェクトオーナーは、すべての次工程を自ら分解・指示することではなく、以下に注力したい。

- プロジェクトの思想・要件が守られているかの確認
- あなたが行った設計判断のレビュー
- RustコードおよびRust文化の理解
- EPUB仕様上の判断
- UXや製品仕様に関する最終判断

そのためあなたは、

「次に何を実装するか」
「その前に何を決める必要があるか」
「現在どこまで完成しているか」

を継続的に整理し、プロジェクト完走までの道筋を提示する。

ユーザーにプロジェクトマネジメント上の細かなタスク分解を過度に要求しない。

---

# 33. 暫定依存候補

現時点では以下を候補とするが、確定ではない。

```text
clap
  CLI argument parsing

serde
  data model serialization/deserialization

YAML parser
  book.yaml

XML writer/parser
  OPF / XHTML / nav.xhtml

ZIP crate
  EPUB OCF generation

UUID crate
  identifier generation

JPEG metadata reader
  width / height acquisition
```

実装開始時に現行crateを調査し、メンテナンス状況を確認して決定する。

自然順ソート用crateを追加する場合は、依存の必要性と実装の単純さを比較して判断する。

---

# 34. 設計上の基本思想

本ツールは、

```text
Image Converter
```

ではなく、

```text
Manga EPUB Packager
```

として扱う。

入力画像は作品そのものとして尊重し、本ツールはその画像をEPUB仕様に従ってパッケージングする。

したがって責務は、

```text
画像を作る
```

ことではなく、

```text
画像を正しくEPUBに収める
```

ことである。

入力画像の順序については、ファイル名順を便利なデフォルトとして提供しつつ、ユーザーが必要に応じて1ファイル単位で明示的に順序を指定できる柔軟性を持たせる。

---

# 35. 未決定事項

以下は今後決定する。

- YAMLにおける明示的な画像順序指定の最終形式
- 明示順序に含まれない画像をエラーとするか、自動順序で末尾へ追加するか
- natural sortの詳細仕様
- viewportの正確な決定方法
- JPEG縦横比不一致のWARNING閾値
- 数ピクセル程度の差を許容するか
- 縦横比の差を絶対値・相対値のどちらで判定するか
- center配置画像の縦横比WARNING扱い
- 画像サイズが極端に異なる場合のWARNING基準
- `rendition:orientation` を明示出力するか
- `rendition:page-spread-left/right` と非prefix版を併記する互換モード
- NCX互換出力
- EPUB 2 `meta name="cover"` 互換出力
- GUIフレームワークの最終決定
- 使用するRust crate
- CI/CD構成
- リリース方法
- macOSコード署名・notarization
- Windows/Linuxバイナリ配布
- 既存EPUB編集機能の具体的な仕様

以下は決定済みであり、未決定事項として扱わない。

- プロジェクト名は `manga2epub`
- CLIコマンド名は `manga2epub`
- `book.yaml` は本書の初期案を基礎として進める
- EPUB内部で元JPEGファイル名を維持しない
- 明示指定がない場合はファイル名の自然順で読み込む
- 明示的な1ファイル単位のページ順指定を将来サポートする
- `rendition:spread` の標準値は `landscape`
- JPEG縦横比不一致はWARNINGとし、少なくとも初期仕様ではERRORにしない
- 既存EPUB編集機能は最低優先度とする

未決定事項を実装者の独断で固定仕様にしない。

---

# 36. 初期完成条件

最初の実用可能版は、以下を満たした時点とする。

1. macOS上でCLIとして動作する。
2. CLIコマンド名が `manga2epub` である。
3. JPEG群からEPUB 3.3 Fixed Layoutを生成できる。
4. 1ページ目が表紙になる。
5. RTL漫画としてページが進行する。
6. 2ページ目以降がデフォルトでright/left交互になる。
7. 入力画像をファイル名の自然順で読み込める。
8. 任意ページをleft/right/centerへoverrideできる。
9. タイトルを指定できる。
10. タイトル読みをカタカナで指定できる。
11. 著者を指定できる。
12. 著者読みをカタカナで指定できる。
13. Descriptionを指定できる。
14. Publisherを指定できる。
15. Identifierを指定できる。
16. Identifier未指定時はUUIDを生成する。
17. 目次を指定できる。
18. 入力JPEGを再圧縮・加工しない。
19. 縦横比不一致をWARNINGとして通知できる。
20. 縦横比不一致によって通常のEPUB生成を失敗させない。
21. EPUBCheckで重大なエラーがない。
22. Apple Booksで正常に開ける。
23. Google Play BooksでFixed Layout漫画として実用可能な表示になる。
24. EPUB生成処理がCLIから分離されたRustライブラリになっている。
25. 将来のGUIで1ファイル単位のページ順指定を扱えるデータモデルになっている。

この状態を達成してからGUI化を検討する。
