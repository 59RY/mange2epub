use std::{error::Error, fmt, path::PathBuf};

use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::{
    DocumentError, ImageCollectionError, MinimalMetadata, PackageError, collect_jpeg_images,
    generate_documents, write_epub,
};

/// 1回の EPUB 生成に必要な入力パス
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildRequest {
    pub image_directory: PathBuf,
    pub output_path: PathBuf,
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
    CollectImages(ImageCollectionError),
    GenerateDocuments(DocumentError),
    WritePackage(PackageError),
    TruncateModifiedTime(time::error::ComponentRange),
    FormatModifiedTime(time::error::Format),
}

impl fmt::Display for BuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CollectImages(error) => write!(formatter, "{error}"),
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
            Self::CollectImages(error) => Some(error),
            Self::GenerateDocuments(error) => Some(error),
            Self::WritePackage(error) => Some(error),
            Self::TruncateModifiedTime(error) => Some(error),
            Self::FormatModifiedTime(error) => Some(error),
        }
    }
}

/// `image_directory` 直下にある JPEG 画像から EPUB を生成する。
pub fn build_epub(request: &BuildRequest) -> Result<BuildReport, BuildError> {
    let images =
        collect_jpeg_images(&request.image_directory).map_err(BuildError::CollectImages)?;
    let metadata = default_metadata()?;
    let documents =
        generate_documents(&images, &metadata).map_err(BuildError::GenerateDocuments)?;
    write_epub(&request.output_path, &images, &documents).map_err(BuildError::WritePackage)?;

    Ok(BuildReport {
        output_path: request.output_path.clone(),
        page_count: images.len(),
    })
}

fn default_metadata() -> Result<MinimalMetadata, BuildError> {
    // 最初に利用できるビルドには、まだメタデータの入力機能がない。
    // 後の入力機能が置き換えるまで、これらの値で EPUB 必須メタデータを満たす
    let modified = format_modified_time(OffsetDateTime::now_utc())?;

    Ok(MinimalMetadata {
        title: "Untitled".to_owned(),
        title_file_as: None,
        creator: None,
        description: None,
        publisher: None,
        identifier: format!("urn:uuid:{}", Uuid::new_v4()),
        language: "ja".to_owned(),
        modified,
    })
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

    use super::{BuildRequest, build_epub};

    static NEXT_TEST_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn builds_an_epub_with_default_metadata() {
        let directory = TestDirectory::new();
        write_jpeg(directory.path().join("page-1.jpg"));
        let output = directory.path().join("book.epub");
        let request = BuildRequest {
            image_directory: directory.path().to_path_buf(),
            output_path: output.clone(),
        };

        let report = build_epub(&request).unwrap();

        assert_eq!(report.output_path, output);
        assert_eq!(report.page_count, 1);
        let package = package_document(&output);
        assert!(package.contains("<dc:title id=\"title\">Untitled</dc:title>"));
        assert!(package.contains("<dc:language>ja</dc:language>"));
        assert_modified_timestamp(&package);
        assert_uuid_identifier(&package);
    }

    fn package_document(path: &Path) -> String {
        // Reading Systemと同じように、アーカイブから最終的な .opf を読み取る
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

    fn assert_uuid_identifier(package: &str) {
        // identifier の値は、最初の identifier 要素の開始・終了タグの間に出力される
        let identifier_start = package.find(">urn:uuid:").unwrap() + 1;
        let identifier_end = package[identifier_start..].find('<').unwrap() + identifier_start;
        let identifier = &package[identifier_start..identifier_end];
        let uuid = identifier.strip_prefix("urn:uuid:").unwrap();

        assert!(Uuid::parse_str(uuid).is_ok());
    }

    fn assert_modified_timestamp(package: &str) {
        // 必須形式は、秒単位・固定長のUTC日時
        let marker = "<meta property=\"dcterms:modified\">";
        let start = package.find(marker).unwrap() + marker.len();
        let end = package[start..].find('<').unwrap() + start;
        let modified = &package[start..end];

        assert_eq!(modified.len(), 20);
        assert!(modified.ends_with('Z'));
        assert!(!modified.contains('.'));
    }

    fn write_jpeg(path: PathBuf) {
        // SOF0セグメントだけで、コアの入力処理が画像サイズを取得できる
        let bytes = [
            0xff, 0xd8, 0xff, 0xc0, 0x00, 0x11, 0x08, 0x06, 0xdf, 0x04, 0xb0, 0x03, 0x01, 0x11,
            0x00, 0x02, 0x11, 0x00, 0x03, 0x11, 0x00, 0xff, 0xd9,
        ];
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
