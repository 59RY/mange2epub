use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

use epub_core::{BuildRequest, PublicationMetadata, build_epub};
use sha2::{Digest, Sha256};
use zip::ZipArchive;

static NEXT_TEST_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

// 実際の画像ファイルから EPUB を生成し、画像形式ごとの出力構成とバイト列を検証する
#[test]
fn builds_the_jpeg_only_fixture() {
    assert_fixture_build(
        "jpeg_only",
        &[
            "image-0000.jpg",
            "image-0001.jpg",
            "image-0002.jpg",
            "image-0003.jpg",
            "image-0004.jpg",
            "image-0005.jpg",
            "image-0006.jpg",
        ],
        "jpg",
        "image/jpeg",
    );
}

// 実際の PNG ファイルから EPUB を生成し、PNG 固有の出力構成とバイト列を検証する
#[test]
fn builds_the_png_only_fixture() {
    assert_fixture_build(
        "png_only",
        &[
            "01-Title.png",
            "02-Blank.png",
            "03-Figure-and-TOC.png",
            "04-Main1.png",
            "05-Main2.png",
            "06-Main3.png",
            "07-Main4.png",
            "08-Main5.png",
            "09-Main6.png",
            "10-Omake1.png",
            "11-Omake2.png",
            "12-EOF.png",
        ],
        "png",
        "image/png",
    );
}

// fixture を入力として EPUB を生成し、各ページ画像が正しい順序と形式で格納されることを確認する
fn assert_fixture_build(
    fixture_name: &str,
    source_names: &[&str],
    extension: &str,
    media_type: &str,
) {
    let input_directory = fixture_directory(fixture_name);
    let temporary_directory = TestDirectory::new();
    let output_path = temporary_directory.path().join("book.epub");
    let request = BuildRequest {
        image_directory: input_directory.clone(),
        image_order: None,
        output_path: output_path.clone(),
        metadata: PublicationMetadata::new("Integration Test Book".to_owned()),
        page_overrides: Vec::new(),
    };

    let report = build_epub(&request).unwrap();

    assert_eq!(report.output_path, output_path);
    assert_eq!(report.page_count, source_names.len());

    let mut archive = ZipArchive::new(File::open(output_path).unwrap()).unwrap();
    let package = read_archive_text(&mut archive, "EPUB/package.opf");
    for (index, source_name) in source_names.iter().enumerate() {
        let image_path = format!("images/image-{index:04}.{extension}");
        let properties = if index == 0 {
            " properties=\"cover-image\""
        } else {
            ""
        };
        assert!(
            package.contains(&format!(
                "<item id=\"image-{index:04}\" href=\"{image_path}\" media-type=\"{media_type}\"{properties}/>"
            ))
        );

        let source = fs::read(input_directory.join(source_name)).unwrap();
        let packaged = read_archive_bytes(&mut archive, &format!("EPUB/{image_path}"));
        assert_eq!(sha256(&source), sha256(&packaged));

        let page = read_archive_text(&mut archive, &format!("EPUB/pages/page-{index:04}.xhtml"));
        assert!(page.contains(&format!("../{image_path}")));
    }
}

// リポジトリ内に保持するダミー画像 fixture のパスを返す
fn fixture_directory(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
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

// 画像の複製を保持せず、内容を安定して比較するための SHA-256 ダイジェストを返す
fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

// テストごとの EPUB 出力先を分離する一時ディレクトリ
struct TestDirectory {
    path: PathBuf,
}

impl TestDirectory {
    // 並列実行でも衝突しない一時ディレクトリを作る
    fn new() -> Self {
        let unique_id = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "epub-core-integration-test-{}-{unique_id}",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap();
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
        fs::remove_dir_all(&self.path).unwrap();
    }
}
