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

// YAML 設定ファイルから EPUB を生成し、複数の著者情報と相対出力先を検証する
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
    let mut package = String::new();
    archive
        .by_name("EPUB/package.opf")
        .unwrap()
        .read_to_string(&mut package)
        .unwrap();
    package
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
