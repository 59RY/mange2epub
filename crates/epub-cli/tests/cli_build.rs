use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

use uuid::Uuid;
use zip::ZipArchive;

static NEXT_TEST_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

// JPEG fixture を CLI から生成し、指定したメタデータと自動生成 identifier を検証する
#[test]
fn builds_the_jpeg_fixture_with_a_generated_identifier() {
    let directory = TestDirectory::new();
    let package = build_fixture(
        directory.path(),
        "jpeg_only",
        &[
            "--title",
            "JPEG integration fixture",
            "--title-file-as",
            "ジェイペグインテグレーションフィクスチャ",
            "--description",
            "JPEG 書籍が正しく作成されているかどうかのテストです。",
            "--creator",
            "test",
            "--creator-file-as",
            "テスト",
            "--creator-role",
            "aut",
            "--creator-alternate-script",
            "テスト",
            "--creator-alternate-script-language",
            "ja-Kana",
            "--publisher",
            "Test Publishers",
            "--date",
            "2026-08-31",
            "--language",
            "ja",
        ],
    );

    assert_common_metadata(
        &package,
        "JPEG integration fixture",
        "ジェイペグインテグレーションフィクスチャ",
        "JPEG 書籍が正しく作成されているかどうかのテストです。",
    );
    assert!(package.contains("<dc:date>2026-08-31</dc:date>"));
    assert!(!package.contains("<dc:type>"));
    assert!(!package.contains("<dc:subject>"));
    assert_generated_uuid_identifier(&package);
}

// PNG fixture を CLI から生成し、指定した identifier を変更せずに出力することを検証する
#[test]
fn builds_the_png_fixture_with_the_specified_identifier() {
    let directory = TestDirectory::new();
    let package = build_fixture(
        directory.path(),
        "png_only",
        &[
            "--title",
            "PNG integration fixture",
            "--title-file-as",
            "ピングインテグレーションフィクスチャ",
            "--description",
            "PNG 書籍が正しく作成されているかどうかのテストです。",
            "--creator",
            "test",
            "--creator-file-as",
            "テスト",
            "--creator-role",
            "aut",
            "--creator-alternate-script",
            "テスト",
            "--creator-alternate-script-language",
            "ja-Kana",
            "--publisher",
            "Test Publishers",
            "--date",
            "2026-08-31T15:00:00Z",
            "--type",
            "comic",
            "--type",
            "image",
            "--subject",
            "Illustration",
            "--subject",
            "Fiction",
            "--language",
            "ja",
            "--identifier",
            "urn:test:59RY.manga2epub.integrationTest.pngFixture",
        ],
    );

    assert_common_metadata(
        &package,
        "PNG integration fixture",
        "ピングインテグレーションフィクスチャ",
        "PNG 書籍が正しく作成されているかどうかのテストです。",
    );
    assert!(package.contains("<dc:date>2026-08-31T15:00:00Z</dc:date>"));
    assert_repeated_metadata(&package);
    assert!(
        package.contains(
            "<dc:identifier id=\"pub-id\">urn:test:59RY.manga2epub.integrationTest.pngFixture</dc:identifier>"
        )
    );
}

// YAML 設定ファイルから EPUB を生成し、書誌情報とページ配置を検証する
#[test]
fn builds_the_png_fixture_from_a_yaml_configuration_file() {
    let directory = TestDirectory::new();
    let configuration_path = directory.path().join("book.yaml");
    let input_directory = fixture_directory("png_only");
    let output_path = directory.path().join("book.epub");
    std::fs::write(
        &configuration_path,
        format!(
            r#"version: 1
output: book.epub
book:
  title: YAML integration fixture
  creators:
    - name: test
      roles:
        - aut
        - edt
      alternate_scripts:
        - lang: ja-Kana
          value: テスト
    - name: editor
  date: "2026-09-01T00:00:00+09:00"
  types:
    - comic
    - image
  subjects:
    - Illustration
    - Fiction
  language: ja
images:
  directory: {}
pages:
  overrides:
    - page: 4
      placement: center
toc:
  entries:
    - label: 表紙
      page: 1
    - label: 導入
      page: 2
    - label: 目次ページ
      page: 3
    - label: 本編
      page: 4
      children:
        - label: おまけ
          page: 10
"#,
            yaml_string(&input_directory)
        ),
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_manga2epub"))
        .args(["build", "--config"])
        .arg(&configuration_path)
        .status()
        .unwrap();

    assert!(status.success());

    let package = read_package_document(&output_path);
    assert!(package.contains("<dc:title id=\"title\">YAML integration fixture</dc:title>"));
    assert!(package.contains("<dc:creator id=\"creator-0000\">test</dc:creator>"));
    assert!(package.contains(
        "<meta property=\"role\" refines=\"#creator-0000\" scheme=\"marc:relators\">aut</meta>"
    ));
    assert!(package.contains(
        "<meta property=\"role\" refines=\"#creator-0000\" scheme=\"marc:relators\">edt</meta>"
    ));
    assert!(package.contains(
        "<meta property=\"alternate-script\" refines=\"#creator-0000\" xml:lang=\"ja-Kana\">テスト</meta>"
    ));
    assert!(package.contains("<dc:creator id=\"creator-0001\">editor</dc:creator>"));
    assert!(package.contains("<dc:date>2026-09-01T00:00:00+09:00</dc:date>"));
    assert_repeated_metadata(&package);
    assert_generated_uuid_identifier(&package);
    assert_page_placement(&package, 0, "center");
    assert_page_placement(&package, 1, "right");
    assert_page_placement(&package, 2, "left");
    assert_page_placement(&package, 3, "center");
    assert_page_placement(&package, 4, "right");
    assert_page_placement(&package, 5, "left");

    let navigation = read_navigation_document(&output_path);
    let cover = navigation.find(">表紙</a>").unwrap();
    let introduction = navigation.find(">導入</a>").unwrap();
    let contents_page = navigation.find(">目次ページ</a>").unwrap();
    let main_content = navigation.find(">本編</a>").unwrap();
    let bonus = navigation.find(">おまけ</a>").unwrap();
    assert!(cover < introduction);
    assert!(introduction < contents_page);
    assert!(contents_page < main_content);
    assert!(main_content < bonus);
    assert!(navigation[main_content..bonus].contains("<ol>"));
    assert_eq!(navigation.matches("<ol>").count(), 2);
}

// 直接指定した JPEG と PNG を明示順序で収録し、画像のバイト列を維持する
#[test]
fn builds_a_mixed_fixture_in_the_explicit_cli_order() {
    let directory = TestDirectory::new();
    let input_directory = prepare_mixed_fixture(directory.path());
    let output_path = directory.path().join("cli-order.epub");
    let status = Command::new(env!("CARGO_BIN_EXE_manga2epub"))
        .arg("build")
        .arg(&input_directory)
        .args(["--output"])
        .arg(&output_path)
        .args([
            "--title",
            "Explicit CLI order fixture",
            "--image-order",
            "02-本文.png",
            "--image-order",
            "01-表紙.jpg",
        ])
        .status()
        .unwrap();

    assert!(status.success());
    assert_mixed_image_order(&output_path, &input_directory);
}

// YAML の明示順序で混在画像を収録し、並べ替え後のページへ配置を適用する
#[test]
fn builds_a_mixed_fixture_in_the_explicit_yaml_order() {
    let directory = TestDirectory::new();
    let input_directory = prepare_mixed_fixture(directory.path());
    let configuration_path = directory.path().join("ordered-book.yaml");
    let output_path = directory.path().join("yaml-order.epub");
    std::fs::write(
        &configuration_path,
        format!(
            r#"version: 1
output: yaml-order.epub
book:
  title: Explicit YAML order fixture
images:
  directory: {}
  order:
    - "02-本文.png"
    - "01-表紙.jpg"
pages:
  overrides:
    - page: 2
      placement: center
"#,
            yaml_string(&input_directory)
        ),
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_manga2epub"))
        .args(["build", "--config"])
        .arg(&configuration_path)
        .status()
        .unwrap();

    assert!(status.success());
    assert_mixed_image_order(&output_path, &input_directory);
    assert_page_placement(&read_package_document(&output_path), 1, "center");
}

// 日本語名の混在 fixture を YAML から生成し、見開き、画像の保持、階層目次を検証する
#[test]
fn builds_the_mixed_book_fixture_from_a_yaml_configuration_file() {
    // YAML の入力値とは別に期待値を持ち、設定と出力が同時に誤っても見落とさない
    let expected_pages = [
        ("表紙.png", "png", "image/png", 874, 1240, "center"),
        ("空白(2P用).png", "png", "image/png", 874, 1240, "right"),
        ("目次(3P用).png", "png", "image/png", 874, 1240, "left"),
        (
            "大きなページ(4,5P用).jpg",
            "jpg",
            "image/jpeg",
            1748,
            1240,
            "center",
        ),
        (
            "通常コンテント(6P用).png",
            "png",
            "image/png",
            874,
            1240,
            "right",
        ),
        (
            "通常コンテント(7P用).jpg",
            "jpg",
            "image/jpeg",
            874,
            1240,
            "left",
        ),
        (
            "大きなページ(8,9P用).png",
            "png",
            "image/png",
            1748,
            1240,
            "center",
        ),
        ("EOF.png", "png", "image/png", 874, 1240, "center"),
    ];
    let source_directory = fixture_directory("mixed");
    let directory = TestDirectory::new();
    let configuration_path = directory.path().join("book.yaml");
    let output_path = directory.path().join("book.epub");

    // 手動確認と同じ YAML を使用し、出力はテスト固有のディレクトリへ隔離する
    std::fs::copy(source_directory.join("book.yaml"), &configuration_path).unwrap();
    for (source_name, ..) in &expected_pages {
        std::fs::copy(
            source_directory.join(source_name),
            directory.path().join(source_name),
        )
        .unwrap();
    }
    let status = Command::new(env!("CARGO_BIN_EXE_manga2epub"))
        .args(["build", "--config"])
        .arg(&configuration_path)
        .status()
        .unwrap();

    assert!(status.success());
    let mut archive = ZipArchive::new(File::open(&output_path).unwrap()).unwrap();
    let package = read_archive_text(&mut archive, "EPUB/package.opf");
    assert_eq!(package.matches("<item id=\"image-").count(), 8);
    assert_eq!(package.matches("href=\"pages/page-").count(), 8);
    assert_eq!(package.matches("<itemref ").count(), 8);
    assert_eq!(package.matches("media-type=\"image/jpeg\"").count(), 2);
    assert_eq!(package.matches("media-type=\"image/png\"").count(), 6);
    assert_eq!(package.matches("properties=\"cover-image\"").count(), 1);

    let mut previous_spine_position = 0;
    for (index, (source_name, extension, media_type, width, height, placement)) in
        expected_pages.iter().enumerate()
    {
        let image_path = format!("images/image-{index:04}.{extension}");
        let properties = if index == 0 {
            " properties=\"cover-image\""
        } else {
            ""
        };
        assert!(package.contains(&format!(
            "<item id=\"image-{index:04}\" href=\"{image_path}\" media-type=\"{media_type}\"{properties}/>"
        )));
        assert!(package.contains(&format!(
            "<item id=\"page-{index:04}\" href=\"pages/page-{index:04}.xhtml\" media-type=\"application/xhtml+xml\" properties=\"svg\"/>"
        )));

        // 見開き画像も 1 ページとして扱い、spine の順序と配置を確認する
        let itemref = format!(
            "<itemref idref=\"page-{index:04}\" properties=\"rendition:page-spread-{placement}\"/>"
        );
        let spine_position = package.find(&itemref).unwrap();
        assert!(previous_spine_position < spine_position);
        previous_spine_position = spine_position;

        let source = std::fs::read(source_directory.join(source_name)).unwrap();
        let packaged = read_archive_bytes(&mut archive, &format!("EPUB/{image_path}"));
        assert_eq!(source.len(), packaged.len(), "{source_name}");
        assert!(source == packaged, "{source_name}");

        let page = read_archive_text(&mut archive, &format!("EPUB/pages/page-{index:04}.xhtml"));
        assert!(page.contains(&format!("content=\"width={width}, height={height}\"")));
        assert!(page.contains(&format!("viewBox=\"0 0 {width} {height}\"")));
        assert!(page.contains(&format!(
            "<image width=\"{width}\" height=\"{height}\" preserveAspectRatio=\"xMidYMid meet\" xlink:href=\"../{image_path}\"/>"
        )));
        assert_eq!(
            page.matches("preserveAspectRatio=\"xMidYMid meet\"")
                .count(),
            2
        );
    }

    // 入れ子とリンク先も比較し、子項目の平坦化や印刷ページ番号との取り違えを検出する
    let navigation = read_archive_text(&mut archive, "EPUB/nav.xhtml");
    assert!(navigation.contains(
        r#"<nav epub:type="toc">
      <ol>
        <li>
          <a href="pages/page-0000.xhtml">表紙</a>
        </li>
        <li>
          <a href="pages/page-0001.xhtml">導入</a>
        </li>
        <li>
          <a href="pages/page-0002.xhtml">目次ページ</a>
        </li>
        <li>
          <a href="pages/page-0003.xhtml">本編</a>
          <ol>
            <li>
              <a href="pages/page-0003.xhtml">本編（前半）</a>
            </li>
            <li>
              <a href="pages/page-0006.xhtml">本編（後半）</a>
            </li>
          </ol>
        </li>
        <li>
          <a href="pages/page-0007.xhtml">裏表紙</a>
        </li>
      </ol>
    </nav>"#
    ));
}

// 指定した fixture と CLI オプションから EPUB を生成し、OPF パッケージ文書を読み取る
fn build_fixture(
    output_directory: &Path,
    fixture_name: &str,
    metadata_arguments: &[&str],
) -> String {
    let input_directory = fixture_directory(fixture_name);
    let output_path = output_directory.join("book.epub");
    let status = Command::new(env!("CARGO_BIN_EXE_manga2epub"))
        .arg("build")
        .arg(input_directory)
        .arg("--output")
        .arg(&output_path)
        .args(metadata_arguments)
        .status()
        .unwrap();

    assert!(status.success());
    read_package_document(&output_path)
}

// JPEG と PNG で共通の CLI メタデータが OPF へ正しく出力されることを確認する
fn assert_common_metadata(package: &str, title: &str, title_file_as: &str, description: &str) {
    assert!(package.contains(&format!("<dc:title id=\"title\">{title}</dc:title>")));
    assert!(package.contains(&format!(
        "<meta property=\"file-as\" refines=\"#title\">{title_file_as}</meta>"
    )));
    assert!(package.contains("<dc:creator id=\"creator-0000\">test</dc:creator>"));
    assert!(package.contains("<meta property=\"file-as\" refines=\"#creator-0000\">テスト</meta>"));
    assert!(package.contains(
        "<meta property=\"role\" refines=\"#creator-0000\" scheme=\"marc:relators\">aut</meta>"
    ));
    assert!(
        package.contains(
            "<meta property=\"alternate-script\" refines=\"#creator-0000\" xml:lang=\"ja-Kana\">テスト</meta>"
        )
    );
    assert!(package.contains(&format!("<dc:description>{description}</dc:description>")));
    assert!(package.contains("<dc:publisher>Test Publishers</dc:publisher>"));
    assert!(package.contains("<dc:language>ja</dc:language>"));
}

// 繰り返し可能な type と subject が、指定順で個別の要素として出力されることを確認する
fn assert_repeated_metadata(package: &str) {
    assert_eq!(package.matches("<dc:type>").count(), 2);
    let comic_position = package.find("<dc:type>comic</dc:type>").unwrap();
    let image_position = package.find("<dc:type>image</dc:type>").unwrap();
    assert!(comic_position < image_position);

    assert_eq!(package.matches("<dc:subject>").count(), 2);
    let illustration_position = package
        .find("<dc:subject>Illustration</dc:subject>")
        .unwrap();
    let fiction_position = package.find("<dc:subject>Fiction</dc:subject>").unwrap();
    assert!(illustration_position < fiction_position);
}

// YAML で指定した配置と、その後の既定配置が OPF の spine に出力されることを確認する
fn assert_page_placement(package: &str, page_index: usize, placement: &str) {
    assert!(package.contains(&format!(
        "<itemref idref=\"page-{page_index:04}\" properties=\"rendition:page-spread-{placement}\"/>"
    )));
}

// 既存 fixture から、自然順と明示順序が異なる Unicode 名の混在入力を作る
fn prepare_mixed_fixture(directory: &Path) -> PathBuf {
    let input_directory = directory.join("mixed-images");
    std::fs::create_dir(&input_directory).unwrap();
    std::fs::copy(
        fixture_directory("jpeg_only").join("image-0000.jpg"),
        input_directory.join("01-表紙.jpg"),
    )
    .unwrap();
    std::fs::copy(
        fixture_directory("png_only").join("02-Blank.png"),
        input_directory.join("02-本文.png"),
    )
    .unwrap();

    input_directory
}

// 指定順での形式・参照先と、元画像のバイト列が EPUB 内でも一致することを確認する
fn assert_mixed_image_order(output_path: &Path, input_directory: &Path) {
    let mut archive = ZipArchive::new(File::open(output_path).unwrap()).unwrap();
    let package = read_archive_text(&mut archive, "EPUB/package.opf");
    assert_eq!(package.matches("<item id=\"image-").count(), 2);
    assert!(package.contains(
        "<item id=\"image-0000\" href=\"images/image-0000.png\" media-type=\"image/png\" properties=\"cover-image\"/>"
    ));
    assert!(package.contains(
        "<item id=\"image-0001\" href=\"images/image-0001.jpg\" media-type=\"image/jpeg\"/>"
    ));
    let first_page = read_archive_text(&mut archive, "EPUB/pages/page-0000.xhtml");
    let second_page = read_archive_text(&mut archive, "EPUB/pages/page-0001.xhtml");
    assert!(first_page.contains("../images/image-0000.png"));
    assert!(second_page.contains("../images/image-0001.jpg"));

    for (source_name, archive_path) in [
        ("02-本文.png", "EPUB/images/image-0000.png"),
        ("01-表紙.jpg", "EPUB/images/image-0001.jpg"),
    ] {
        let source = std::fs::read(input_directory.join(source_name)).unwrap();
        let packaged = read_archive_bytes(&mut archive, archive_path);
        assert_eq!(source.len(), packaged.len());
        assert!(source == packaged);
    }
}

// identifier 未指定時は UUID を含む urn:uuid の値が生成されることを確認する
fn assert_generated_uuid_identifier(package: &str) {
    let marker = "<dc:identifier id=\"pub-id\">urn:uuid:";
    let start = package.find(marker).unwrap() + marker.len();
    let end = package[start..].find('<').unwrap() + start;

    assert!(Uuid::parse_str(&package[start..end]).is_ok());
}

// リポジトリ内に保持する epub-core のダミー画像 fixture を参照する
fn fixture_directory(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("epub-core")
        .join("tests")
        .join("fixtures")
        .join(name)
}

// YAML のダブルクォート文字列として、安全にパスを埋め込む
fn yaml_string(path: &Path) -> String {
    format!(
        "\"{}\"",
        path.to_string_lossy()
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
    )
}

// 生成した EPUB から OPF パッケージ文書を UTF-8 テキストとして読み取る
fn read_package_document(output_path: &Path) -> String {
    let mut archive = ZipArchive::new(File::open(output_path).unwrap()).unwrap();
    read_archive_text(&mut archive, "EPUB/package.opf")
}

// 生成した EPUB から Navigation Document を UTF-8 テキストとして読み取る
fn read_navigation_document(output_path: &Path) -> String {
    let mut archive = ZipArchive::new(File::open(output_path).unwrap()).unwrap();
    read_archive_text(&mut archive, "EPUB/nav.xhtml")
}

// ZIP エントリを UTF-8 テキストとして読み取る
fn read_archive_text(archive: &mut ZipArchive<File>, path: &str) -> String {
    let mut text = String::new();
    archive
        .by_name(path)
        .unwrap()
        .read_to_string(&mut text)
        .unwrap();
    text
}

// ZIP エントリを元画像との比較に使うバイト列として読み取る
fn read_archive_bytes(archive: &mut ZipArchive<File>, path: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    archive
        .by_name(path)
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    bytes
}

// テストごとの出力ファイルを分離する一時ディレクトリ
struct TestDirectory {
    path: PathBuf,
}

impl TestDirectory {
    // 並列実行でも衝突しない一時ディレクトリを作る
    fn new() -> Self {
        let unique_id = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "epub-cli-integration-test-{}-{unique_id}",
            std::process::id()
        ));
        std::fs::create_dir(&path).unwrap();
        Self { path }
    }

    // EPUB の出力先として使う一時ディレクトリを返す
    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDirectory {
    // テスト完了後に出力用の一時ディレクトリを削除する
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.path).unwrap();
    }
}
