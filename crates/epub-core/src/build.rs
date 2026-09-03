use std::{error::Error, fmt, path::PathBuf};

use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::{
    DocumentError, ImageCollectionError, MetadataError, MinimalMetadata, PackageError,
    PageOverride, PageOverrideError, PublicationMetadata, TocEntry, collect_images,
    collect_images_in_order, generate_documents, resolve_page_placements, write_epub,
};

/// 1回の EPUB 生成に必要な入力値
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildRequest {
    pub image_directory: PathBuf,
    /// 未指定時は自然順、指定時は列挙された画像だけを指定順で使用する
    pub image_order: Option<Vec<PathBuf>>,
    pub output_path: PathBuf,
    pub metadata: PublicationMetadata,
    /// 1 始まりのページ番号で指定する配置の上書き
    pub page_overrides: Vec<PageOverride>,
    /// 指定順で出力する目次項目。空の場合は書籍タイトルで第 1 ページへリンクする
    pub toc_entries: Vec<TocEntry>,
}

/// EPUB 生成が成功したときに返す結果
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildReport {
    pub output_path: PathBuf,
    pub page_count: usize,
}

/// EPUB 生成処理全体で発生しうるエラー
#[derive(Debug)]
pub enum BuildError {
    InvalidMetadata(MetadataError),
    CollectImages(ImageCollectionError),
    ResolvePagePlacements(PageOverrideError),
    GenerateDocuments(DocumentError),
    WritePackage(PackageError),
    TruncateModifiedTime(time::error::ComponentRange),
    FormatModifiedTime(time::error::Format),
}

impl fmt::Display for BuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMetadata(error) => write!(formatter, "{error}"),
            Self::CollectImages(error) => write!(formatter, "{error}"),
            Self::ResolvePagePlacements(error) => write!(formatter, "{error}"),
            Self::GenerateDocuments(error) => write!(formatter, "{error}"),
            Self::WritePackage(error) => write!(formatter, "{error}"),
            Self::TruncateModifiedTime(_) => {
                write!(formatter, "could not truncate EPUB modified timestamp")
            }
            Self::FormatModifiedTime(_) => {
                write!(formatter, "could not format EPUB modified timestamp")
            }
        }
    }
}

impl Error for BuildError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidMetadata(error) => Some(error),
            Self::CollectImages(error) => Some(error),
            Self::ResolvePagePlacements(error) => Some(error),
            Self::GenerateDocuments(error) => Some(error),
            Self::WritePackage(error) => Some(error),
            Self::TruncateModifiedTime(error) => Some(error),
            Self::FormatModifiedTime(error) => Some(error),
        }
    }
}

/// 入力画像を自然順または明示順で収集し、EPUB を生成する
pub fn build_epub(request: &BuildRequest) -> Result<BuildReport, BuildError> {
    let metadata = resolve_metadata(&request.metadata)?;
    let images = match request.image_order.as_deref() {
        Some(image_order) => collect_images_in_order(&request.image_directory, image_order),
        None => collect_images(&request.image_directory),
    }
    .map_err(BuildError::CollectImages)?;
    let placements = resolve_page_placements(images.len(), &request.page_overrides)
        .map_err(BuildError::ResolvePagePlacements)?;
    let documents = generate_documents(&images, &metadata, &placements, &request.toc_entries)
        .map_err(BuildError::GenerateDocuments)?;
    write_epub(&request.output_path, &images, &documents).map_err(BuildError::WritePackage)?;

    Ok(BuildReport {
        output_path: request.output_path.clone(),
        page_count: images.len(),
    })
}

/// 利用者の書誌情報を検証し、ビルドごとに決まる値を補う
fn resolve_metadata(metadata: &PublicationMetadata) -> Result<MinimalMetadata, BuildError> {
    metadata.validate().map_err(BuildError::InvalidMetadata)?;

    let modified = format_modified_time(OffsetDateTime::now_utc())?;
    let identifier = metadata
        .identifier
        .clone()
        .unwrap_or_else(|| format!("urn:uuid:{}", Uuid::new_v4()));

    Ok(MinimalMetadata::from_publication(
        metadata, identifier, modified,
    ))
}

fn format_modified_time(timestamp: OffsetDateTime) -> Result<String, BuildError> {
    // 0ナノ秒固定にする。EPUB での時間精度は秒単位のため
    timestamp
        .replace_nanosecond(0)
        .map_err(BuildError::TruncateModifiedTime)?
        .format(&Rfc3339)
        .map_err(BuildError::FormatModifiedTime)
}

// 単体テストでは、コマンドライン引数解析を介さずにコアの処理全体を実行する
#[cfg(test)]
mod tests {
    use std::{
        fs,
        io::Read,
        path::{Path, PathBuf},
        sync::atomic::{AtomicUsize, Ordering},
    };

    use uuid::Uuid;
    use zip::ZipArchive;

    use super::{BuildError, BuildRequest, build_epub};
    use crate::{MetadataError, PageOverride, PagePlacement, PublicationMetadata, TocEntry};

    static NEXT_TEST_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

    #[test]
    // identifier を省略すると、ビルド処理が UUID を生成して EPUB へ出力する
    fn builds_an_epub_with_generated_identifier() {
        let directory = TestDirectory::new();
        write_jpeg(directory.path().join("page-1.jpg"));
        let output = directory.path().join("book.epub");
        let request = BuildRequest {
            image_directory: directory.path().to_path_buf(),
            image_order: None,
            output_path: output.clone(),
            metadata: PublicationMetadata::new("書籍のタイトル".to_owned()),
            page_overrides: Vec::new(),
            toc_entries: Vec::new(),
        };

        let report = build_epub(&request).unwrap();

        assert_eq!(report.output_path, output);
        assert_eq!(report.page_count, 1);
        let package = package_document(&output);
        assert!(package.contains("<dc:title id=\"title\">書籍のタイトル</dc:title>"));
        assert!(package.contains("<dc:language>ja</dc:language>"));
        assert!(!package.contains("<dc:date>"));
        assert!(!package.contains("<dc:type>"));
        assert!(!package.contains("<dc:subject>"));
        assert_modified_timestamp(&package);
        assert_uuid_identifier(&package);
        let navigation = navigation_document(&output);
        assert!(navigation.contains("<a href=\"pages/page-0000.xhtml\">書籍のタイトル</a>"));
    }

    #[test]
    // 指定された identifier は変更せず、Primary Identifier として出力する
    fn builds_an_epub_with_the_specified_identifier() {
        let directory = TestDirectory::new();
        write_jpeg(directory.path().join("page-1.jpg"));
        let output = directory.path().join("book.epub");
        let mut metadata = PublicationMetadata::new("書籍のタイトル".to_owned());
        metadata.identifier = Some("https://example.com/books/123".to_owned());
        let request = BuildRequest {
            image_directory: directory.path().to_path_buf(),
            image_order: None,
            output_path: output.clone(),
            metadata,
            page_overrides: Vec::new(),
            toc_entries: Vec::new(),
        };

        build_epub(&request).unwrap();

        let package = package_document(&output);
        assert!(package.contains(
            "<dc:identifier id=\"pub-id\">https://example.com/books/123</dc:identifier>"
        ));
    }

    #[test]
    // PNG でも、画像形式に合う manifest 項目を持つ EPUB を生成する
    fn builds_an_epub_with_a_png_input() {
        let directory = TestDirectory::new();
        write_png(directory.path().join("page-1.png"));
        let output = directory.path().join("book.epub");
        let request = BuildRequest {
            image_directory: directory.path().to_path_buf(),
            image_order: None,
            output_path: output.clone(),
            metadata: PublicationMetadata::new("書籍のタイトル".to_owned()),
            page_overrides: Vec::new(),
            toc_entries: Vec::new(),
        };

        build_epub(&request).unwrap();

        let package = package_document(&output);
        assert!(package.contains(
            "<item id=\"image-0000\" href=\"images/image-0000.png\" media-type=\"image/png\" properties=\"cover-image\"/>"
        ));
    }

    #[test]
    // 画像を読む前に書誌情報を検証するため、入力不備を早く利用者へ返せる
    fn rejects_invalid_metadata_before_collecting_images() {
        let request = BuildRequest {
            image_directory: PathBuf::from("does-not-need-to-exist"),
            image_order: None,
            output_path: PathBuf::from("does-not-need-to-be-created.epub"),
            metadata: PublicationMetadata::new(" ".to_owned()),
            page_overrides: Vec::new(),
            toc_entries: Vec::new(),
        };

        let error = build_epub(&request).unwrap_err();

        assert!(matches!(
            error,
            BuildError::InvalidMetadata(MetadataError::EmptyTitle)
        ));
    }

    #[test]
    // 解決済み配置を spine へ渡し、center の直後を right から再開する
    fn builds_an_epub_with_page_placement_overrides() {
        let directory = TestDirectory::new();
        for page_number in 1..=6 {
            write_jpeg(directory.path().join(format!("page-{page_number}.jpg")));
        }
        let output = directory.path().join("book.epub");
        let request = BuildRequest {
            image_directory: directory.path().to_path_buf(),
            image_order: None,
            output_path: output.clone(),
            metadata: PublicationMetadata::new("書籍のタイトル".to_owned()),
            page_overrides: vec![PageOverride {
                page_number: 4,
                placement: PagePlacement::Center,
            }],
            toc_entries: Vec::new(),
        };

        build_epub(&request).unwrap();

        let package = package_document(&output);
        assert!(package.contains(
            "<itemref idref=\"page-0003\" properties=\"rendition:page-spread-center\"/>"
        ));
        assert!(
            package.contains(
                "<itemref idref=\"page-0004\" properties=\"rendition:page-spread-right\"/>"
            )
        );
        assert!(
            package.contains(
                "<itemref idref=\"page-0005\" properties=\"rendition:page-spread-left\"/>"
            )
        );
    }

    #[test]
    // 明示順序を manifest と spine へ反映し、ページ配置を並べ替え後に適用する
    fn builds_an_epub_with_an_explicit_image_order() {
        let directory = TestDirectory::new();
        write_jpeg(directory.path().join("page-1.jpg"));
        write_png(directory.path().join("page-2.png"));
        let output = directory.path().join("book.epub");
        let request = BuildRequest {
            image_directory: directory.path().to_path_buf(),
            image_order: Some(vec![
                PathBuf::from("page-2.png"),
                PathBuf::from("page-1.jpg"),
            ]),
            output_path: output.clone(),
            metadata: PublicationMetadata::new("書籍のタイトル".to_owned()),
            page_overrides: vec![PageOverride {
                page_number: 2,
                placement: PagePlacement::Center,
            }],
            toc_entries: Vec::new(),
        };

        build_epub(&request).unwrap();

        let package = package_document(&output);
        assert!(package.contains(
            "<item id=\"image-0000\" href=\"images/image-0000.png\" media-type=\"image/png\" properties=\"cover-image\"/>"
        ));
        assert!(package.contains(
            "<item id=\"image-0001\" href=\"images/image-0001.jpg\" media-type=\"image/jpeg\"/>"
        ));
        assert!(package.contains(
            "<itemref idref=\"page-0001\" properties=\"rendition:page-spread-center\"/>"
        ));
    }

    #[test]
    // 利用者が指定した目次項目を、指定順のまま最終的な nav.xhtml へ出力する
    fn builds_an_epub_with_toc_entries() {
        let directory = TestDirectory::new();
        for page_number in 1..=3 {
            write_jpeg(directory.path().join(format!("page-{page_number}.jpg")));
        }
        let output = directory.path().join("book.epub");
        let request = BuildRequest {
            image_directory: directory.path().to_path_buf(),
            image_order: None,
            output_path: output.clone(),
            metadata: PublicationMetadata::new("書籍のタイトル".to_owned()),
            page_overrides: Vec::new(),
            toc_entries: vec![
                TocEntry {
                    label: "本編".to_owned(),
                    page_number: 2,
                    children: vec![TocEntry {
                        label: "おまけ".to_owned(),
                        page_number: 3,
                        children: Vec::new(),
                    }],
                },
                TocEntry {
                    label: "あとがき".to_owned(),
                    page_number: 3,
                    children: Vec::new(),
                },
            ],
        };

        build_epub(&request).unwrap();

        let navigation = navigation_document(&output);
        let main_entry = navigation
            .find("<a href=\"pages/page-0001.xhtml\">本編</a>")
            .unwrap();
        let bonus_entry = navigation
            .find("<a href=\"pages/page-0002.xhtml\">おまけ</a>")
            .unwrap();
        let afterword_entry = navigation
            .find("<a href=\"pages/page-0002.xhtml\">あとがき</a>")
            .unwrap();
        assert!(main_entry < bonus_entry);
        assert!(bonus_entry < afterword_entry);
        assert!(!navigation.contains(">書籍のタイトル</a>"));
    }

    fn package_document(path: &Path) -> String {
        // ビューアーと同じように、アーカイブから最終的な .opf を読み取る
        let file = fs::File::open(path).unwrap();
        let mut archive = ZipArchive::new(file).unwrap();
        let mut package = String::new();
        archive
            .by_name("EPUB/package.opf")
            .unwrap()
            .read_to_string(&mut package)
            .unwrap();
        package
    }

    fn navigation_document(path: &Path) -> String {
        // パッケージ済みの EPUB から、ビューアーが参照する目次を読み取る
        let file = fs::File::open(path).unwrap();
        let mut archive = ZipArchive::new(file).unwrap();
        let mut navigation = String::new();
        archive
            .by_name("EPUB/nav.xhtml")
            .unwrap()
            .read_to_string(&mut navigation)
            .unwrap();
        navigation
    }

    fn assert_uuid_identifier(package: &str) {
        // identifier の値は、最初の identifier 要素の開始・終了タグの間に出力される
        let identifier_start = package.find(">urn:uuid:").unwrap() + 1;
        let identifier_end = package[identifier_start..].find('<').unwrap() + identifier_start;
        let identifier = &package[identifier_start..identifier_end];
        let uuid = identifier.strip_prefix("urn:uuid:").unwrap();

        assert!(Uuid::parse_str(uuid).is_ok());
    }

    fn assert_modified_timestamp(package: &str) {
        // 必須形式は、秒単位・固定長の UTC 日時
        let marker = "<meta property=\"dcterms:modified\">";
        let start = package.find(marker).unwrap() + marker.len();
        let end = package[start..].find('<').unwrap() + start;
        let modified = &package[start..end];

        assert_eq!(modified.len(), 20);
        assert!(modified.ends_with('Z'));
        assert!(!modified.contains('.'));
    }

    fn write_jpeg(path: PathBuf) {
        // SOF0 セグメントだけで、コアの入力処理が画像サイズを取得できる
        let bytes = [
            0xff, 0xd8, 0xff, 0xc0, 0x00, 0x11, 0x08, 0x06, 0xdf, 0x04, 0xb0, 0x03, 0x01, 0x11,
            0x00, 0x02, 0x11, 0x00, 0x03, 0x11, 0x00, 0xff, 0xd9,
        ];
        fs::write(path, bytes).unwrap();
    }

    fn write_png(path: PathBuf) {
        // IHDR までを持つ最小の PNG ヘッダーで、画像形式ごとの処理を確認する
        let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        bytes.extend_from_slice(&13_u32.to_be_bytes());
        bytes.extend_from_slice(b"IHDR");
        bytes.extend_from_slice(&1200_u32.to_be_bytes());
        bytes.extend_from_slice(&1759_u32.to_be_bytes());
        fs::write(path, bytes).unwrap();
    }

    struct TestDirectory {
        path: PathBuf,
    }

    impl TestDirectory {
        fn new() -> Self {
            let unique_id = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "epub-core-build-test-{}-{unique_id}",
                std::process::id()
            ));
            fs::create_dir(&path).unwrap();
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.path).unwrap();
        }
    }
}
