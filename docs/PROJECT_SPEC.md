# manga2epub — Project Specification

## 1. 文書の目的

本書は、漫画の各ページを画像として受け取り、漫画向け固定レイアウト EPUB を生成するコンバーター「manga2epub」の仕様・設計方針を定義する。CLI アプリケーションとして実装し、CLI と EPUB 生成コアが十分に安定した段階で、同じコアライブラリを使う GUI アプリケーションへ拡張する。

本書は次の用途を兼ねる。

- プロジェクトの要求仕様
- EPUB 生成仕様
- Rust 実装時の設計指針
- Codex 等の生成 AI に実装を依頼する際の前提資料
- 将来の GUI 実装の基礎仕様

仕様が未確定の箇所は「未決事項」として明示する。実装者が独断で大きな仕様を追加してはならない。

---

# 2. プロジェクト概要

## 2.1 目的

JPEG または PNG 形式で用意した漫画の各ページを、EPUB 3.3 準拠の Fixed-Layout EPUB としてパッケージングする。

本ツールが担う責務は次のとおり。

- EPUB パッケージ生成
- 書誌メタデータ生成
- 固定レイアウト設定
- 漫画向けページ進行方向設定
- ページの左右・中央配置設定
- 表紙設定
- 目次生成
- EPUB コンテナ生成
- EPUB 仕様上の妥当性検証

画像編集の機能は持たない。

## 2.2 プロジェクト名

プロジェクト名は `manga2epub` とする。CLI コマンド名も原則としてこれに合わせる。

## 2.3 リスペクト元

設計思想と用途の面で、NODO epub maker をリスペクト元の一つとする。ただし実装をコピーするのではなく、EPUB 3.3 仕様に基づいて独自に実装する。

---

# 3. 対象環境

## 3.1 必須 OS

macOS で動作すること。

## 3.2 クロスプラットフォーム

実装上大きな追加コストがなければ、Windows・Linux での動作も対象に含める。ただし、macOS 対応を犠牲にしてクロスプラットフォーム化はしない。

---

# 4. 技術選定

## 4.1 言語

第一候補として Rust を採用する。プロジェクトオーナーは Rust の実務知識を持っていないため、Rust の慣習は尊重しつつ、過度に技巧的なコードは避ける。

## 4.2 Rust を採用する理由

本プロジェクトで重視するのは次の点である。

- CLI アプリケーションを単一実行ファイルとして配布しやすい
- macOS で自然に動作する
- Windows/Linux への展開が比較的容易
- XML、ZIP、ファイル I/O を安全に扱える
- CLI と将来の GUI から共通利用できるライブラリを作りやすい
- 型によって不正な状態を排除しやすい
- テスト可能なコアライブラリを構築しやすい

## 4.3 将来の GUI

GUI フレームワークの第一候補は Tauri とする。ただし、GUI は初期開発の対象外とする。

GUI 化のために EPUB 生成処理を書き直すことがないよう、生成処理は UI 層から分離しておく。将来の GUI では、画像を 1 ファイル単位で並べ替える操作を想定する。特に、ファイル名順ではなく利用者が明示的にページ順を指定する用途を想定する。

---

# 5. Rust での設計方針

## 5.1 コアと UI を分離する

EPUB 生成ロジックは CLI の `main()` に直接実装しない。概念上、次の構造とする。

```text
CLI
  │
  ▼
epub-core
  ▲
  │
GUI
```

CLI と将来の GUI は、同一の `epub-core` を利用する。

## 5.2 型で状態を表現する

例えばページ配置は文字列のままコアへ渡さず、enum で表現する。

```rust
enum PagePlacement {
    Left,
    Right,
    Center,
}
```

読書方向も同様に、取り得る値を型で限定する。

```rust
enum ReadingDirection {
    Rtl,
    Ltr,
}
```

## 5.3 Rust の基本文化として重視する事項

- `cargo fmt` で整形できるコードを書く
- `cargo clippy` を継続的に通す
- 通常のエラー処理に `panic!` を使わない
- CLI 入力エラー、ファイル I/O エラー、EPUB 生成エラーを `Result` で適切に伝播する
- 不必要な `unwrap()`/`expect()` を避ける
- public API は必要最小限にする
- 過度なジェネリクスや抽象化を避ける
- 最初から巨大な設計にしない
- テスト可能な小さな関数へ分割する

プロジェクトオーナーが Rust を学ぶ機会でもあるため、Rust 特有の実装を採用する場合は理由が理解できる構造にする。

---

# 6. EPUB 仕様

## 6.1 対象バージョン

EPUB 3.3 をターゲットとする。OPF の package 要素は、EPUB 3.x 仕様に従い `<package version="3.0" ...>` とする。`version="3.3"` とはしない。

## 6.2 レイアウト

漫画は固定レイアウトで生成するため、以下を使用する。

```xml
<meta property="rendition:layout">pre-paginated</meta>
```

## 6.3 ページ進行方向

デフォルトは日本漫画向けの右綴じとする。

```xml
<spine page-progression-direction="rtl">
```

将来、設定により LTR も指定できるようにする。

## 6.4 Synthetic Spread

標準のデフォルト値は `landscape` とする。

```xml
<meta property="rendition:spread">landscape</meta>
```

これにより、横画面など Synthetic Spread を利用できる環境で見開きを構成できる。

設定値として、将来的に次を扱える構造にする。

```text
auto
none
landscape
both
```

初期 CLI では `landscape` を標準値とし、他の値を公開するかは実装時に判断する。

## 6.5 Orientation

EPUB 3.3 標準の `<meta property="rendition:orientation">...</meta>` は必要に応じて利用できるようにする。ただしデフォルトではビューアー側へ過度な制約を課さない。原則として `auto` か、指定自体を省略する。

---

# 7. 表紙

入力画像の 1 ページ目を自動的に表紙とする。例えば入力画像が

```text
image-0000.jpg
image-0001.jpg
image-0002.jpg
...
```

の場合、`image-0000.jpg` を表紙画像とする。manifest では次のように設定する。

```xml
<item
    id="cover"
    href="images/image-0000.jpg"
    media-type="image/jpeg"
    properties="cover-image"/>
```

EPUB 内部のファイル名は入力元のファイル名を維持しない。初期実装では EPUB 内部で正規化したファイル名を使う。

表紙画像は本文の 1 ページ目も兼ねる。EPUB メタデータの cover-image であると同時に、spine 上の第 1 ページでもある。

---

# 8. ページ配置

## 8.1 内部表現

ページ配置は少なくとも次の 3 種類を持つ。

```text
left
right
center
```

- left: Synthetic Spread 上の左ページ
- right: Synthetic Spread 上の右ページ
- center: 1 枚の画像で単独の見開き相当ページを構成する。通常の左右ページとは異なり、Synthetic Spread 上で単独中央表示させる

## 8.2 EPUB へのマッピング

ページ配置プロパティは、パッケージ文書の spine にある対応する `itemref` へマッピングする。manifest の `item` には書かない。

基本出力では、次のように `rendition:` 付き表記を使用する。

```xml
<itemref idref="page-0000" properties="rendition:page-spread-left"/>
<itemref idref="page-0001" properties="rendition:page-spread-right"/>
<itemref idref="page-0002" properties="rendition:page-spread-center"/>
```

EPUB 3.3 向けの内部表現・基本出力では、`rendition:` 付き表記に統一する。

古いビューアーとの互換性が必要な場合に限り、left/right について次のような互換出力を可能にする余地を残す。

```xml
<itemref idref="page-0000" properties="rendition:page-spread-left page-spread-left"/>
```

初期バージョンでの実装は必須ではない。

---

# 9. デフォルトページ配置

## 9.1 基本ルール

デフォルトは次のとおり。

```text
Page 1 = center + cover

Page 2 = right
Page 3 = left
Page 4 = right
Page 5 = left
...
```

1 ページ目を除き、right/left と交互に配置する。

## 9.2 override

利用者がページ配置を指定した場合、そのページのみ自動判定を上書きする。

例えば

```text
Page 1 = center
Page 2 = right
Page 3 = left
Page 4 = center  ← override
Page 5 = left    ← 通常の自動判定
Page 6 = right
```

のように、override によって後続ページの自動計算をシフトさせない。後続ページも含めた特殊な構成が必要な場合は、利用者が必要なページをすべて指定する。

## 9.3 フルカスタマイズ

全ページの配置を指定することも可能にする。一部のみの override と全ページ指定を別の機構にせず、同じ override 機構で表現する。100 ページすべてを override として指定すれば、結果として完全な手動指定になる。

---

# 10. 入力画像

## 10.1 初期対応形式

JPEG および PNG を必須対応形式とする。最低限、`.jpg` `.jpeg` `.png` を扱えること。拡張子の大文字小文字は、macOS/Windows 間の互換性を考慮する。

## 10.2 画像処理を行わない

本ツールは入力画像に手を加えない。リサイズ、再圧縮、画像品質変更、トリミング、余白除去、色補正、カラープロファイル変換、自動回転、アスペクト比補正のいずれも行わない。EPUB 内へ格納する画像のバイト列は、原則として入力ファイルと同一とする。

## 10.3 画像情報の参照

EPUB 生成に必要な範囲で、画像の width・height 等の基本属性を読み取ることは許容する。これは画像加工には含めない。

## 10.4 ページサイズ

固定レイアウト XHTML には viewport 寸法が必要となる。漫画全体で共通の論理 viewport を使用する。第一候補として、先頭画像の width/height を基準 viewport として使う。

```html
<meta
    name="viewport"
    content="width=1200, height=1759"/>
```

画像そのものは加工せず、XHTML/CSS 上で論理 viewport 内に表示する。

入力画像の縦横比は漫画内で統一されていることを前提とする。縦横比の異なる画像が投入されても、原則として EPUB 生成は失敗させない。ただし他の画像と明らかに縦横比が異なる場合は WARNING を表示する。

数ピクセル程度の差を警告対象とするかは、画像サイズ・縦横比・許容誤差の扱いを含めて実装時に決定する。center 配置の画像は見開き相当の画像である可能性があるため、通常ページと異なるサイズや縦横比でも直ちに WARNING とするとは限らない。この扱いは実装時に明確化する。

いずれの場合も、縦横比不一致は少なくとも初期仕様では ERROR としない。

---

# 11. XHTML ページ

各画像について 1 つの XHTML Content Document を生成する。

```text
image-0000.jpg → page-0000.xhtml
image-0001.jpg → page-0001.xhtml
image-0002.png → page-0002.xhtml
```

XHTML は固定レイアウト用 viewport を持ち、対応する画像を 1 枚表示する。画像はページ全面へ表示し、余白・padding・margin は持たせない。画像の再エンコードは行わない。

EPUB 内部の画像ファイル名・XHTML ファイル名は、入力元のファイル名に依存せず、決定論的な正規化名を使ってよい。

---

# 12. メタデータ

## 12.1 タイトル

必須項目とする。

```xml
<dc:title id="title">書籍のタイトル</dc:title>
```

## 12.2 タイトル読み

タイトルの読みを指定できるようにする。読みはカタカナを想定する。

```xml
<meta
    property="file-as"
    refines="#title">ショセキノタイトル</meta>
```

`file-as` は仕様上「ソート等に利用する正規化表現」であるため、内部データモデルでは単に「ruby」などと命名せず、意味を分離する。例えば `title` / `title_file_as`、あるいは `display` / `file_as` のように分ける。GUI 上では「タイトル読み」と表示してよい。

## 12.3 著者

著者は `dc:creator` で表現する。

```xml
<dc:creator id="creator1">祐天寺</dc:creator>
```

著者の役割は次のように設定できるようにする。

```xml
<meta
    property="role"
    refines="#creator1"
    scheme="marc:relators">aut</meta>
```

漫画では作者が作画も行う場合があるため、将来的には複数 role を扱えるデータモデルにしたい。例えば `aut` と `ill` を同一 creator へ設定できる余地を残す。

初期実装では著者は任意の 1 名とし、役割を省略した場合は `aut` を使う。複数著者・複数 role は、内部データモデルと CLI の利用例を確定してから追加する。

## 12.4 著者読み

カタカナの読みを `file-as` として指定できるようにする。

```xml
<meta
    property="file-as"
    refines="#creator1">ユウテンジ</meta>
```

## 12.5 alternate-script

必要に応じて著者名等へ別 script 表現を設定できるようにする。カタカナ表現は例えば次のようにする。

```xml
<meta
    property="alternate-script"
    refines="#creator1"
    xml:lang="ja-Kana">ユウテンジ</meta>
```

ローマ字転写は次の表現を第一候補とする。

```xml
<meta
    property="alternate-script"
    refines="#creator1"
    xml:lang="ja-Latn">Yūtenji</meta>
```

`alternate-script` は必須ではない。`file-as` と `alternate-script` は用途が異なるため、内部データモデルでも区別する。初期実装では著者名にのみ対応し、値を指定する場合は `xml:lang` に対応する言語タグも指定する。

## 12.6 Description

Description は任意指定とする。

```xml
<dc:description>ここで指定した値が Play Books で表示されることを期待する。</dc:description>
```

Google Play Books での表示を主要な利用目的の一つとする。実際の表示はビューアー側の仕組みと Google 側の実装に依存するため、結合テストで確認する。

## 12.7 Publisher

任意指定とする。

```xml
<dc:publisher>Yūtenji Publishers</dc:publisher>
```

## 12.8 Language

必須項目とする。デフォルトは `<dc:language>ja</dc:language>` 。CLI オプションから変更でき、設定ファイル導入後も同じ意味の値を指定できるようにする。

## 12.9 Identifier

利用者が指定できる。指定があればその値を Primary Identifier として使い、なければ UUID を自動生成する。

```xml
<dc:identifier id="pub-id">urn:uuid:12345678-abcd-1234-ef00-123456789abc</dc:identifier>
```

`package` は次のようにする。

```xml
<package
    ...
    unique-identifier="pub-id">
```

利用者が任意 ID を設定した場合も、EPUB として一意な Identifier となる文字列として扱う。

URI/URN の組み立て方は、設定値をそのまま使う方式とツール側で `urn:` を付加する方式を混在させない。CLI オプションおよび将来の設定ファイルでは指定値をそのまま使い、ツール側で `urn:` などの接頭辞は補わない。

## 12.10 modified

EPUB 3.3 で必要な `<meta property="dcterms:modified">...</meta>` を生成する。日時は EPUB 生成時の UTC 時刻から生成し、秒精度の `YYYY-MM-DDThh:mm:ssZ` 形式とする（1 秒未満の端数は含めない）。

---

# 13. 互換メタデータ

過去に利用していた次のようなメタデータは、EPUB 3.3 標準出力では原則として生成しない。

```xml
<meta content="true" name="fixed-layout"/>
<meta content="none" name="orientation-lock"/>
<meta content="1200x1759" name="original-resolution"/>
<meta content="comic" name="book-type"/>
<meta content="horizontal-rl" name="primary-writing-mode"/>
<meta content="#ffffff" name="SpineColor"/>
```

これらは EPUB 3.3 Core における固定レイアウトの基本表現ではないためである。代わりに標準出力では次を使う。

```xml
<meta property="rendition:layout">pre-paginated</meta>
<meta property="rendition:spread">landscape</meta>
```

各 XHTML の `<meta name="viewport" content="width=..., height=..."/>`、manifest の `properties="cover-image"`、および spine の `page-progression-direction="rtl"` も使う。

特定のビューアー向け互換性が必要と判明した場合のみ、compatibility profile として追加する。ベンダー固有メタデータを標準動作へ混在させない。

## 13.1 表紙の旧仕様互換

表紙画像は、EPUB 3.3 の標準である manifest の `cover-image` プロパティで指定する。Finder の Quick Look を含む旧仕様の表紙参照にも対応するため、標準出力では次も生成する。

```xml
<meta name="cover" content="image-0000"/>
```

`content` の値は、表紙画像を表す manifest item の `id` と一致させる。このメタデータは EPUB 2 互換のための限定的な例外であり、他の旧仕様・ベンダー固有メタデータを追加する根拠にはしない。

---

# 14. Prefix

OPF の package 要素では、相互運用性を優先して `rendition` prefix を明示的に宣言する。

```xml
<package
    xmlns="http://www.idpf.org/2007/opf"
    xmlns:dc="http://purl.org/dc/elements/1.1/"
    version="3.0"
    unique-identifier="pub-id"
    prefix="rendition: http://www.idpf.org/vocab/rendition/#">
```

必要に応じて他の prefix も同様に扱う。

---

# 15. 目次

## 15.1 EPUB Navigation Document

EPUB 3.3 では `nav.xhtml` を正式な目次として生成する。

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

manifest では `nav` プロパティを付与する。

## 15.2 NCX

`toc.ncx` は EPUB 3.3 の標準目次としては使わない。したがってデフォルトでは `<spine toc="ncx">` も生成しない。

将来、古いビューアーとの互換性が必要になった場合、`compatibility.ncx = true` のような互換オプションで生成できるようにしてよい。NCX を生成する場合は、その `dtb:uid` と EPUB の Primary Identifier を同期させる。

---

# 16. EPUB ファイル構造

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

EPUB 内部のファイル名は入力元のファイル名を維持せず、正規化した連番形式を使う。元ファイルそのもののバイト列は変更しない。

---

# 17. ZIP / OCF

EPUB は単なる任意の ZIP ではなく、EPUB OCF として正しく生成する。

特に `mimetype` は `application/epub+zip` のみを内容とし、次を厳守する。

- `mimetype` は ZIP の最初の entry とする
- `mimetype` は圧縮しない
- 不要な BOM、改行、前後空白を入れない

他のファイルは必要に応じて Deflate 圧縮する。

---

# 18. 設定ファイル

## 18.1 形式

YAML を採用する。ファイル名のデフォルトは `book.yaml` とする。

`book.yaml` は推奨する標準ファイル名であり、CLI が現在のディレクトリから自動検出することはしない。YAML を使う場合は、利用者が設定ファイルへのパスを明示する。

## 18.2 方針

CLI 引数だけで全書誌情報・全ページ指定を行わせない。CLI は簡単な操作に使い、複雑な書籍設定は YAML へ記述する。GUI 化した場合も、GUI の内部データモデルと YAML を極力共通化する。

---

# 19. book.yaml 初期案

以下のスキーマで進める。

```yaml
version: 1

book:
  title: "書籍のタイトル"
  title_file_as: "ショセキノタイトル"

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

`overrides` は疎な指定と完全指定の両方に使う。通常は以下のように一部だけ指定する。

```yaml
pages:
  overrides:
    - page: 24
      placement: center
```

完全に手動指定したい場合は、以下のように全ページを指定する。部分カスタマイズと完全カスタマイズで別々の仕様は作らない。

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

## 20.1 page 番号

YAML 上の `page` は 1-origin とする。`page: 1` は第 1 画像、すなわち表紙を指す。内部 Rust 実装では 0-origin でも構わない。

## 20.2 ページ順序の指定

画像の取り込み順は、次の 2 種類を扱える設計とする。

### デフォルト

利用者による明示指定がない場合、入力ディレクトリ内の対応画像をファイル名の昇順で読み込む。数値部分は自然順で比較するため、`page-1.jpg` `page-2.jpg` `page-10.jpg` はこの順になる。単純な辞書順によって `page-10.jpg` が `page-2.jpg` より前になることを避ける。

### 明示指定

利用者が 1 ファイル単位で順番を指定した場合は、その指定を優先する。CLI/YAML では、将来的に次のような形式を扱える構造を想定する。

```yaml
images:
  directory: "./images"

  order:
    - "cover.jpg"
    - "page-01.jpg"
    - "page-02.jpg"
```

または、GUI で管理するページ順序を YAML へ保存できる構造を想定する。具体的な最終スキーマは、自然順ソートおよび GUI のデータモデルと合わせて確定する。

明示指定時に、指定されていないファイルを自動的に末尾へ追加するか、未指定ファイルをエラーとするかは実装時に決定する。

---

# 21. CLI

実行ファイル名および CLI コマンド名は `manga2epub` とする。

## 21.1 EPUB 生成

```bash
manga2epub build ./images --output ./book.epub --title "書籍のタイトル"
```

`<image_directory>`、`--output`、`--title` を指定して EPUB を生成する。画像ディレクトリ内の対応画像を自然順でページとして扱う。

## 21.2 メタデータ指定

タイトル以外のメタデータは、必要に応じて CLI オプションで指定する。

```bash
manga2epub build ./images \
  --output ./book.epub \
  --title "書籍のタイトル" \
  --title-file-as "ショセキノタイトル" \
  --creator "著者名" \
  --creator-file-as "チョシャメイ" \
  --creator-role aut \
  --creator-alternate-script "チョシャメイ" \
  --creator-alternate-script-language ja-Kana \
  --description "紹介文" \
  --publisher "発行元" \
  --language ja \
  --identifier "urn:uuid:12345678-abcd-1234-ef00-123456789abc"
```

`--language` の既定値は `ja` とする。`--identifier` を省略した場合は UUID を自動生成する。`--creator-role` を省略した場合は `aut` とする。`--creator-alternate-script` を指定する場合は `--creator-alternate-script-language` も指定する。

## 21.3 設定ファイル指定

設定ファイルは、CLI オプションで同じメタデータを繰り返し指定する負担を減らすために導入する。

```bash
manga2epub build ./book.yaml
```

設定ファイルを導入するまでは、この形式を受け付けない。

## 21.4 初期設定生成

将来的に `manga2epub init` で雛形 `book.yaml` を生成できるようにする。

## 21.5 検査

将来的に `manga2epub check ./book.epub` のような形を提供してよい。ただし EPUBCheck そのものを Rust で再実装しない。

## 21.6 inspect

将来的に `manga2epub inspect ./book.epub` で、次のような情報を表示できるようにする。

```text
Title: 書籍のタイトル
Creator: 祐天寺
Pages: 52
Direction: RTL
Layout: Fixed
Spread: Landscape
Cover: image-0000.jpg
TOC entries: 3
```

初期リリースの必須機能ではない。

## 21.7 既存 EPUB 編集

将来的に `manga2epub edit ./book.epub` のような既存 EPUB 編集機能を提供できるとよい。ただし最低優先度の機能とし、初期段階では読み込み・編集・再パッケージングを実装しない。

## 21.8 表示ロケール

CLI の利用者向けメッセージは、日本語と英語を切り替えられるようにする。

```bash
manga2epub --locale ja build ./images --output ./book.epub
manga2epub --locale en build ./images --output ./book.epub
```

表示ロケールは次の優先順位で決定する。

1. `--locale` で明示された言語
2. OS から取得した実行ロケールのうち対応している言語
3. 英語

OS から取得した言語に対応していない場合も英語を使う。

翻訳文は CLI crate のロケールファイルで管理し、単一実行ファイルへ埋め込む。EPUB 生成コアはロケールを扱わず、構造化したエラーを返す。CLI はそのエラーを利用者向けメッセージへ翻訳する。

日本語の利用者向けメッセージでは、英数字と日本語の間に半角スペースを入れる。表示ロケールは、EPUB メタデータの `language` とは独立した設定とする。

---

# 22. Rust プロジェクト構造

初期段階から Cargo Workspace を利用する案を第一候補とする。

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

GUI 実装時には、`crates/` に `epub-gui/` を追加する形で拡張する。

---

# 23. epub-core の責務

`epub-core` は UI を一切知らない。少なくとも次を担当する。

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

CLI 引数解析は担当しない。Tauri にも依存させない。

---

# 24. epub-cli の責務

CLI が担当するのは次のみ。

- コマンドライン引数解析
- CLI オプションからの入力値の組み立て
- YAML 読み込み（設定ファイル入力を提供する場合）
- ユーザー向けエラー表示
- WARNING 表示
- `epub-core` 呼び出し
- 終了コード

EPUB XML を CLI 側で直接組み立てない。

---

# 25. XML 生成

XML を大量の文字列連結で生成せず、適切な XML ライブラリを利用する。特に `&` `<` `>` `"` `'` を正しく escape する。タイトル、著者名、Description 等へ任意の Unicode 文字列が入っても、正常な XML を生成する。UTF-8 を前提とする。

---

# 26. ファイル順序

画像ファイルの読み込み順は決定論的でなければならない。ファイルシステムが返した順序をそのまま利用しない。

## 26.1 デフォルト順序

利用者による明示指定がない場合、ファイル名の昇順で読み込む。数値部分は自然順で比較するため、`image-1.jpg` `image-2.jpg` `image-10.jpg` はこの順になる。ゼロ埋めされたファイル名も引き続き推奨する（`image-0000.jpg`、`image-0001.jpg` など）。

## 26.2 明示的な順序指定

利用者が 1 ファイル単位で順番を指定した場合は、その順序を優先する。この機能は CLI/YAML で利用でき、将来の GUI では特に重要な機能として扱う。

GUI では、画像のドラッグ&ドロップ等でページ順を変更し、その結果を内部データモデルおよび YAML へ保存できるようにしたい。

明示指定時に、指定されていない画像を未指定エラーとするか、自動順序で末尾へ追加するかは、最終的な YAML スキーマ確定時に決定する。

## 26.3 EPUB 内部名

EPUB 内部では入力元のファイル名を維持する必要はない。ページ順に基づく正規化名を使う。第一候補は次の形式とする。

```text
images/image-0000.jpg
images/image-0001.jpg
images/image-0002.jpg
```

---

# 27. エラーと警告

## エラーにすべき例

- 入力画像が 0 枚
- book title が未指定
- 不正な YAML
- 存在しないページを TOC が参照
- 存在しないページを override が参照
- 同一ページへ矛盾した placement 指定
- 明示された画像ファイルが存在しない
- 明示された画像順序に同一ファイルが重複している
- EPUB 生成先へ書き込めない
- 対応画像として読み取れない入力

## 警告候補

- 画像の縦横比が他ページ（基準画像）と明らかに異なる
- 画像サイズが極端に異なる
- Description 未指定
- Publisher 未指定
- TOC が空
- alternate-script の language tag が不自然
- 明示順序に含まれない画像が自動的に追加された

## 27.1 縦横比不一致の扱い

画像の縦横比不一致は WARNING とし、少なくとも初期仕様では ERROR にしない。例えば、最初の画像群がおおむね 1:1.4 であるにもかかわらず後続画像に正方形画像が含まれる場合は、WARNING を表示する。

一方、数ピクセル程度の差を警告対象とするかは慎重に扱う。実装時の検討事項は次のとおり。

- ピクセル寸法の差ではなく、縦横比の差で判定する
- 許容誤差を絶対値または相対値で定義する
- 画像サイズが小さい場合と大きい場合で同じ閾値を使うか検討する
- center 配置の画像を通常ページと同じ基準で判定するか検討する

center 配置の画像は見開き相当ページである可能性があるため、通常ページと異なる縦横比でも必ず WARNING とするとは限らない。警告判定は、ページ配置、基準 viewport、画像サイズ、縦横比を考慮して設計する。警告は EPUB 生成を必ずしも失敗させない。

---

# 28. テスト方針

## 28.1 Unit Test

少なくとも次をテストする。

```text
Page 1 -> Center
Page 2 -> Right
Page 3 -> Left
Page 4 -> Right
```

override（`Page 4 -> Center`）が指定された場合、`Page 5` の自動配置が影響を受けないことを確認する。

自然順ソートについて、`page-1.jpg` `page-2.jpg` `page-10.jpg` が正しい順序になることを確認する。明示的なファイル順序指定については、指定順がデフォルトの自然順より優先されることを確認する。

## 28.2 XML テスト

生成された OPF について、title、creator、creator role、file-as、alternate-script、description、publisher、identifier、modified、cover-image、rendition（layout/spread）、page-progression-direction、itemref 順序、page placement を確認する。

## 28.3 Navigation Test

`nav.xhtml` の目次項目が正しい XHTML へリンクすることを確認する。

## 28.4 画像無加工テスト

重要な品質条件として、入力画像と EPUB 内画像の SHA-256 を比較し、`SHA256(input image) == SHA256(image extracted from EPUB)` が成立することを確認する。これにより、本ツールが入力画像を再圧縮・改変していないことを自動テストする。

## 28.5 Warning Test

次をテストする。

- 明らかに縦横比が異なる画像で WARNING が発生する
- 縦横比不一致によって ERROR にならない
- center 配置の画像について、通常ページと異なる扱いができる
- 許容誤差内の差を WARNING とするかが、定義した閾値に従う

## 28.6 EPUBCheck

生成された EPUB を EPUBCheck で検証する。プロジェクトの品質基準として、EPUBCheck の error をゼロにすることを目標とする。warning については内容を確認し、プロジェクトとして許容するものを明示する。

## 28.7 実アプリによるテスト

少なくとも次で実機確認する。

- iBooks（macOS）
- Google Play Books（WebUI、Phase 6 以降は iOS アプリも）

可能であれば他のビューアーでも確認したい。ビューアーごとの差異は、EPUB 仕様上の問題と個別実装の問題を分けて記録する。

---

# 29. 開発フェーズ

## Phase 1 — 最小 EPUB

以下だけを実装する。

```text
JPEG directory
↓
EPUB 3.3 固定レイアウト
```

固定仕様は、JPEG のみ、自然順のファイル名順、RTL、Page 1 = cover + center、Page 2 = right、Page 3 = left…という並びである。まず EPUBCheck およびビューアーで正しく開けることを確認する。

## Phase 2 — Metadata

次を追加する。

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
- PNG input support

## Phase 3 — YAML

`book.yaml` を実装する。

## Phase 4 — Page customization

ページ override（left/right/center）を実装する。

## Phase 5 — Explicit page ordering

利用者が 1 ファイル単位でページ順を指定できる機能を実装する。CLI/YAML で利用可能にし、将来の GUI で扱いやすい内部データモデルを確立する。

## Phase 6 — TOC

`nav.xhtml` を生成する。

## Phase 7 — CLI 完成度向上

必要に応じて `init`、`build`、`check`、`inspect` 等を追加する。

## Phase 8 — GUI

`epub-core` をそのまま利用して GUI を構築する。GUI で EPUB 生成ロジックを再実装しない。

## Phase 9 — 既存 EPUB 編集

最低優先度の将来機能として、既存 EPUB の読み込み・編集・再パッケージングを検討する。

---

# 30. 初期段階で実装しない機能

以下はスコープ外とする。

- 画像の自動縮小
- 画像圧縮率変更
- 画像フォーマット変換
- 自動トリミング
- 自動余白除去
- 自動色補正
- 自動ページ分割
- OCR
- PDF 変換
- CBZ 変換
- DRM
- 電子署名
- EPUB 2 専用出力
- GUI
- 既存 EPUB 編集
- クラウドサービス
- 書籍販売サイトへの自動アップロード

要望がない限り勝手に追加しない。

---

# 31. Compatibility Profile

将来的にビューアー固有の互換処理が必要になった場合、コアの EPUB 3.3 出力へ直接混ぜず、compatibility profile として分離する。候補は `standard`、`legacy`、`apple`、`google-play` 。ただし、実際に必要性が確認されるまで profile 自体を実装しない。特定ベンダー向けタグを「念のため」で大量に出力しない。

---

# 32. Codex 等、生成 AI への実装指示

Codex 等の各種生成 AI（以降 32 章では「あなた」と呼ぶ）は、本書をプロジェクト仕様の基準として扱うこと。

## 32.1 実装前

大きな機能を実装する前に、要求 → EPUB 仕様 → Rust データモデル → 実装、の順に考える。不明点を独自仕様で埋めない。特に次は、実装前に仕様を確認する。

- 自然順ソートの定義
- 明示的な画像順序指定
- 縦横比 WARNING の閾値
- center 配置画像の警告扱い
- EPUB 内部ファイル名
- 既存 EPUB 編集機能の優先度

## 32.2 Rust 初心者向け配慮

プロジェクトオーナーは Rust 初心者である。そのため、Rust として非 idiomatic な実装にはしない一方、必要以上に高度な Rust 機能も使わない。lifetime を不必要に複雑化せず、unsafe は原則使用しない。マクロを乱用せず、過剰な抽象化を避ける。なぜその型・crate・設計を使うのか説明できるようにする。

## 32.3 依存 crate

crate 追加時は、何のための依存か、標準ライブラリでは不十分な理由、メンテナンス状況、ライセンス、依存関係の規模を考慮する。闇雲に crate を増やさない。

## 32.4 品質

変更後は原則として `cargo fmt`、`cargo clippy`、`cargo test` を通す。EPUB 生成機能に変更が入った場合は、可能な範囲で EPUBCheck も実施する。

## 32.5 コミット

一つのコミットへ無関係な変更を混在させない。フォーマット変更だけで大量差分を発生させない。

## 32.6 プロジェクト進行

あなたは、個別に指示された実装だけを受動的に行うのではなく、本プロジェクトが初期完成条件まで迷走せず進むよう、次のステップを継続的に提案する。ただし、利用者の明示的な指示なしに次の大きな実装フェーズへは進まない。

各作業ターンでは、原則として次の流れとする。

1. 今回の作業範囲を確認する
2. `docs/PROJECT_SPEC.md`（本書）と現在の実装状態を照合する
3. 指定された範囲を実装または調査する
4. 必要なテスト・lint・検証を実施する
5. 実施内容と設計上の判断を説明する
6. 現在のプロジェクト進捗を簡潔に整理する
7. 次に行うべき作業を 1 〜 3 個程度提示する
8. その中から、次の 1 ターンとして最も適切な作業を推奨する

次のステップを提案する際は、単に「次の Phase へ進む」のではなく、現在の実装状態、依存関係、テスト可能性、仕様上の未決事項を考慮する。例えば、次の機能を実装する前にデータモデルや仕様を確定した方がよい場合は、実装ではなく設計確認を次のステップとして提案する。

### 32.6.1 次ステップ提案の形式

各ターンの最後に、おおむね次を示す。

```text
Current status:
- 今回完了したこと
- 現在到達している Phase または実装状態

Recommended next step:
- 次に推奨する作業
- その作業を次に行う理由
- 想定する作業範囲

Later:
- その後に予定される主要作業
```

厳密にこの書式へ固定する必要はないが、利用者が次に何を依頼すべきか自分で再設計しなくてもよい状態にする。

### 32.6.2 作業粒度

一度にプロジェクト全体を実装しない。各ターンの変更は、利用者がコードと設計意図をレビューできる程度の大きさに保つ。

目安として、次は適切な単位である。

- Cargo Workspace の初期構築
- 基本データモデルの追加
- 画像列挙と自然順ソート
- ページ配置ロジックとテスト
- OCF/ZIP 生成
- OPF 生成
- XHTML 生成
- Navigation Document 生成
- YAML 設定読み込み
- CLI command の追加

一方、EPUB 生成・YAML・Metadata・TOC・CLI 完成・GUI のように、複数の大きな責務を一度に実装することは原則として避ける。作業を分割する際は、単にコード量だけでなく、各変更が独立してテスト・レビュー可能かを重視する。

### 32.6.3 利用者に求める判断

あなた自身で合理的に決められる実装詳細について、毎回利用者へ判断を求めない。例えば、private 関数名、モジュール分割、テスト関数名、ローカル変数名、自明なエラー型の構成、formatter による整形は、既存仕様と Rust の一般的な慣習から明確に判断できるのであれば、あなた自身で提案または実装してよい。

一方、YAML スキーマ、CLI UX、EPUB 出力仕様、ビューアー互換処理、warning/error の境界、public API、新しい大規模依存 crate、初期スコープ外機能の追加のように、プロジェクトの挙動・互換性・公開インターフェースへ影響する事項は、仕様書に答えがない場合、独断で確定しない。その場合は、推奨案と理由を提示し、利用者が判断できるようにする。

### 32.6.4 プロジェクトオーナーの負荷軽減

プロジェクトオーナーは、すべての次工程を自ら分解・指示するのではなく、プロジェクトの思想・要件が守られているかの確認、あなたが行った設計判断のレビュー、Rust コードおよび文化の理解、EPUB 仕様上の判断、UX や製品仕様に関する最終判断に注力したい。

そのためあなたは、「次に何を実装するか」「その前に何を決める必要があるか」「現在どこまで完成しているか」を継続的に整理し、プロジェクト完走までの道筋を提示する。利用者にプロジェクトマネジメント上の細かなタスク分解を過度に要求しない。

---

# 33. 暫定依存候補

現時点では次を候補とするが、確定ではない。

- `clap` — CLI argument parsing
- `serde` — data model serialization/deserialization
- YAML parser — book.yaml
- XML writer/parser — OPF / XHTML / nav.xhtml
- ZIP crate — EPUB OCF generation
- UUID crate — identifier generation
- JPEG / PNG metadata reader — width / height acquisition

実装開始時に現行 crate を調査し、メンテナンス状況を確認して決定する。自然順ソート用 crate を追加する場合は、依存の必要性と実装の単純さを比較して判断する。

---

# 34. 設計上の基本思想

本ツールは、画像を加工するための Image Converter ではなく、Manga EPUB Packager として扱う。入力画像は作品そのものとして尊重し、EPUB 仕様に従ってパッケージングすることだけを役割とする。

画像の順序については、ファイル名順を便利なデフォルトとして提供しつつ、利用者が必要に応じて 1 ファイル単位で明示的に順序を指定できる柔軟性を持たせる。

---

# 35. 未決事項

以下は今後決定する。

- YAML における明示的な画像順序指定の最終形式
- 明示順序に含まれない画像をエラーとするか、自動順序で末尾へ追加するか
- natural sort の詳細仕様
- viewport の正確な決定方法
- 画像縦横比不一致の WARNING 閾値
- 数ピクセル程度の差を許容するか
- 縦横比の差を絶対値・相対値のどちらで判定するか
- center 配置画像の縦横比 WARNING 扱い
- 画像サイズが極端に異なる場合の WARNING 基準
- `rendition:orientation` を明示出力するか
- `rendition:page-spread-left/right` と非 prefix 版を併記する互換モード
- NCX 互換出力
- GUI フレームワークの最終決定
- 使用する Rust crate
- CI/CD 構成
- リリース方法
- macOS コード署名・notarization
- Windows/Linux バイナリ配布
- 既存 EPUB 編集機能の具体的な仕様

以下は決定済みであり、未決事項として扱わない。

- プロジェクト名は `manga2epub`
- CLI コマンド名は `manga2epub`
- `book.yaml` は本書の初期案を基礎として進める
- EPUB 内部で元の入力ファイル名を維持しない
- 明示指定がない場合はファイル名の自然順で読み込む
- 明示的な 1 ファイル単位のページ順指定を将来サポートする
- `rendition:spread` の標準値は `landscape`
- 画像縦横比不一致は WARNING とし、少なくとも初期仕様では ERROR にしない
- 既存 EPUB 編集機能は最低優先度とする

未決事項を実装者の独断で固定仕様にしない。

---

# 36. 初期完成条件

最初の実用可能版は、以下を満たした時点とする。

1. macOS 上で CLI として動作する
2. CLI コマンド名が `manga2epub` である
3. JPEG または PNG の画像群から EPUB 3.3 固定レイアウトを生成できる
4. 1 ページ目が表紙になる
5. RTL 漫画としてページが進行する
6. 2 ページ目以降がデフォルトで right/left 交互になる
7. 入力画像をファイル名の自然順で読み込める
8. 任意ページを left/right/center へ override できる
9. タイトルを指定できる
10. タイトル読みをカタカナで指定できる
11. 著者を指定できる
12. 著者読みをカタカナで指定できる
13. Description を指定できる
14. Publisher を指定できる
15. Identifier を指定できる
16. Identifier 未指定時は UUID を生成する
17. 目次を指定できる
18. 入力画像を再圧縮・加工しない
19. 縦横比不一致を WARNING として通知できる
20. 縦横比不一致によって通常の EPUB 生成を失敗させない
21. EPUBCheck で重大なエラーがない
22. Apple Books で正常に開ける
23. Google Play Books で固定レイアウト漫画として実用可能な表示になる
24. EPUB 生成処理が CLI から分離された Rust ライブラリになっている
25. 将来の GUI で 1 ファイル単位のページ順指定を扱えるデータモデルになっている

この状態を達成してから GUI 化を検討する。
