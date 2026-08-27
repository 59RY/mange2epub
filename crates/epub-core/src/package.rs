use std::{
    error::Error,
    fmt,
    fs::{File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

use zip::{CompressionMethod, ZipWriter, result::ZipError, write::SimpleFileOptions};

use crate::{GeneratedDocuments, SourceImage};

const EPUB_DIRECTORY: &str = "EPUB";
const MIMETYPE: &str = "application/epub+zip";

/// EPUB Open Container Format アーカイブの書き込み時に発生しうるエラー
#[derive(Debug)]
pub enum PackageError {
    CreateOutput {
        path: PathBuf,
        source: io::Error,
    },
    ReadImage {
        path: PathBuf,
        source: io::Error,
    },
    WriteArchive(io::Error),
    Zip(ZipError),
    PageCountMismatch {
        image_count: usize,
        page_count: usize,
    },
}

impl fmt::Display for PackageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CreateOutput { path, .. } => {
                write!(
                    formatter,
                    "could not create EPUB output: {}",
                    path.display()
                )
            }
            Self::ReadImage { path, .. } => {
                write!(
                    formatter,
                    "could not read image for EPUB output: {}",
                    path.display()
                )
            }
            Self::WriteArchive(_) => write!(formatter, "could not write EPUB archive data"),
            Self::Zip(_) => write!(formatter, "could not create EPUB ZIP archive"),
            Self::PageCountMismatch {
                image_count,
                page_count,
            } => write!(
                formatter,
                "cannot package {image_count} images with {page_count} generated pages"
            ),
        }
    }
}

impl Error for PackageError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CreateOutput { source, .. } | Self::ReadImage { source, .. } => Some(source),
            Self::WriteArchive(source) => Some(source),
            Self::Zip(source) => Some(source),
            Self::PageCountMismatch { .. } => None,
        }
    }
}

/// 生成した EPUB リソース および 入力画像を OCF ZIP アーカイブへ書き込む
pub fn write_epub(
    output_path: &Path,
    images: &[SourceImage],
    documents: &GeneratedDocuments,
) -> Result<(), PackageError> {
    if images.len() != documents.pages.len() {
        return Err(PackageError::PageCountMismatch {
            image_count: images.len(),
            page_count: documents.pages.len(),
        });
    }

    let output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output_path)
        .map_err(|source| PackageError::CreateOutput {
            path: output_path.to_path_buf(),
            source,
        })?;
    let mut archive = ZipWriter::new(output);
    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    // EPUB では、この完全一致の先頭エントリを無圧縮で格納する必要あり
    write_bytes(&mut archive, "mimetype", MIMETYPE.as_bytes(), stored)?;

    write_bytes(
        &mut archive,
        "META-INF/container.xml",
        documents.container_xml.as_bytes(),
        deflated,
    )?;
    write_bytes(
        &mut archive,
        "EPUB/package.opf",
        documents.package_opf.as_bytes(),
        deflated,
    )?;
    write_bytes(
        &mut archive,
        "EPUB/nav.xhtml",
        documents.navigation_xhtml.as_bytes(),
        deflated,
    )?;
    write_bytes(
        &mut archive,
        "EPUB/styles/page.css",
        documents.page_css.as_bytes(),
        deflated,
    )?;

    for page in &documents.pages {
        let path = epub_path(&page.path);
        write_bytes(&mut archive, &path, page.contents.as_bytes(), deflated)?;
    }

    for (index, image) in images.iter().enumerate() {
        let path = format!(
            "{EPUB_DIRECTORY}/images/image-{index:04}.{}",
            image.format.extension()
        );
        write_image(&mut archive, &path, image, deflated)?;
    }

    archive.finish().map_err(PackageError::Zip)?;
    Ok(())
}

fn write_bytes(
    archive: &mut ZipWriter<File>,
    path: &str,
    contents: &[u8],
    options: SimpleFileOptions,
) -> Result<(), PackageError> {
    // テキストリソースは、アーカイブへ入る前に完全なバイト列になっている
    archive
        .start_file(path, options)
        .map_err(PackageError::Zip)?;
    archive
        .write_all(contents)
        .map_err(PackageError::WriteArchive)
}

fn write_image(
    archive: &mut ZipWriter<File>,
    path: &str,
    image: &SourceImage,
    options: SimpleFileOptions,
) -> Result<(), PackageError> {
    // 入力画像のバイト列を ZIP エントリへ直接ストリームする。
    // デコーダーもエンコーダーも使わないため、格納する画像のバイト列は変わらない
    let mut input = File::open(&image.path).map_err(|source| PackageError::ReadImage {
        path: image.path.clone(),
        source,
    })?;
    archive
        .start_file(path, options)
        .map_err(PackageError::Zip)?;
    io::copy(&mut input, archive).map_err(PackageError::WriteArchive)?;
    Ok(())
}

fn epub_path(path: &str) -> String {
    // 文書生成処理は、EPUB ディレクトリからの相対パスを返す
    format!("{EPUB_DIRECTORY}/{path}")
}

// 単体テストでは、CLI コマンドに依存せずに書き出した ZIP アーカイブを検査する
#[cfg(test)]
mod tests {
    use std::{
        fs,
        fs::File,
        io::Read,
        path::{Path, PathBuf},
        sync::atomic::{AtomicUsize, Ordering},
    };

    use sha2::{Digest, Sha256};
    use zip::{CompressionMethod, ZipArchive};

    use super::{MIMETYPE, write_epub};
    use crate::{ImageDimensions, ImageFormat, MinimalMetadata, SourceImage, generate_documents};

    static NEXT_TEST_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn writes_an_ocf_archive_with_an_uncompressed_first_mimetype_entry() {
        let directory = TestDirectory::new();
        let images = images(directory.path());
        let documents = generate_documents(&images, &metadata()).unwrap();
        let output = directory.path().join("book.epub");

        write_epub(&output, &images, &documents).unwrap();

        let raw_archive = fs::read(&output).unwrap();
        let mut archive = ZipArchive::new(File::open(output).unwrap()).unwrap();
        let mut mimetype = archive.by_index(0).unwrap();
        let mut contents = String::new();
        mimetype.read_to_string(&mut contents).unwrap();

        assert_eq!(mimetype.name(), "mimetype");
        assert_eq!(mimetype.compression(), CompressionMethod::Stored);
        assert_eq!(contents, MIMETYPE);
        assert_eq!(&raw_archive[..4], b"PK\x03\x04");
        assert_eq!(u16::from_le_bytes([raw_archive[8], raw_archive[9]]), 0);
        assert_eq!(u16::from_le_bytes([raw_archive[28], raw_archive[29]]), 0);
    }

    #[test]
    fn stores_the_original_image_bytes_without_modification() {
        let directory = TestDirectory::new();
        let images = images(directory.path());
        let documents = generate_documents(&images, &metadata()).unwrap();
        let output = directory.path().join("book.epub");

        write_epub(&output, &images, &documents).unwrap();

        let source = fs::read(&images[0].path).unwrap();
        let mut archive = ZipArchive::new(File::open(output).unwrap()).unwrap();
        let mut packaged = Vec::new();
        archive
            .by_name("EPUB/images/image-0000.jpg")
            .unwrap()
            .read_to_end(&mut packaged)
            .unwrap();

        assert_eq!(sha256(&source), sha256(&packaged));
    }

    #[test]
    // PNG は正規化した拡張子で格納し、入力と同じバイト列を維持する
    fn stores_a_png_with_its_normalized_extension_without_modification() {
        let directory = TestDirectory::new();
        let path = directory.path().join("source.png");
        let source = png_header(1200, 1759);
        fs::write(&path, &source).unwrap();
        let images = vec![SourceImage {
            path,
            format: ImageFormat::Png,
            dimensions: ImageDimensions {
                width: 1200,
                height: 1759,
            },
        }];
        let documents = generate_documents(&images, &metadata()).unwrap();
        let output = directory.path().join("book.epub");

        write_epub(&output, &images, &documents).unwrap();

        let mut archive = ZipArchive::new(File::open(output).unwrap()).unwrap();
        let mut packaged = Vec::new();
        archive
            .by_name("EPUB/images/image-0000.png")
            .unwrap()
            .read_to_end(&mut packaged)
            .unwrap();

        assert_eq!(sha256(&source), sha256(&packaged));
    }

    #[test]
    fn writes_every_generated_resource_to_its_expected_path() {
        let directory = TestDirectory::new();
        let images = images(directory.path());
        let documents = generate_documents(&images, &metadata()).unwrap();
        let output = directory.path().join("book.epub");

        write_epub(&output, &images, &documents).unwrap();

        let mut archive = ZipArchive::new(File::open(output).unwrap()).unwrap();
        for path in [
            "META-INF/container.xml",
            "EPUB/package.opf",
            "EPUB/nav.xhtml",
            "EPUB/styles/page.css",
            "EPUB/pages/page-0000.xhtml",
            "EPUB/images/image-0000.jpg",
        ] {
            assert!(archive.by_name(path).is_ok(), "missing {path}");
        }
    }

    #[test]
    fn refuses_to_overwrite_an_existing_output_file() {
        let directory = TestDirectory::new();
        let images = images(directory.path());
        let documents = generate_documents(&images, &metadata()).unwrap();
        let output = directory.path().join("book.epub");
        fs::write(&output, "existing output").unwrap();

        let error = write_epub(&output, &images, &documents).unwrap_err();

        assert!(matches!(error, super::PackageError::CreateOutput { .. }));
        assert_eq!(fs::read_to_string(output).unwrap(), "existing output");
    }

    fn metadata() -> MinimalMetadata {
        MinimalMetadata {
            title: "Untitled".to_owned(),
            title_file_as: None,
            creator: None,
            description: None,
            publisher: None,
            identifier: "urn:uuid:00000000-0000-0000-0000-000000000000".to_owned(),
            language: "ja".to_owned(),
            modified: "2026-08-26T00:00:00Z".to_owned(),
        }
    }

    fn images(directory: &Path) -> Vec<SourceImage> {
        let path = directory.join("source.jpg");
        let bytes = jpeg_header(1200, 1759);
        fs::write(&path, bytes).unwrap();

        vec![SourceImage {
            path,
            format: ImageFormat::Jpeg,
            dimensions: ImageDimensions {
                width: 1200,
                height: 1759,
            },
        }]
    }

    fn jpeg_header(width: u16, height: u16) -> Vec<u8> {
        // SOF0 セグメントは、パッケージングテスト用の小さく有効なヘッダーになる
        let mut bytes = vec![0xff, 0xd8, 0xff, 0xc0, 0x00, 0x11, 0x08];
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&[0x03, 0x01, 0x11, 0x00, 0x02, 0x11, 0x00, 0x03, 0x11, 0x00]);
        bytes.extend_from_slice(&[0xff, 0xd9]);
        bytes
    }

    fn png_header(width: u32, height: u32) -> Vec<u8> {
        // IHDR を持つ最小の PNG ヘッダーを、パッケージング用の入力画像として使う
        let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        bytes.extend_from_slice(&13_u32.to_be_bytes());
        bytes.extend_from_slice(b"IHDR");
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes
    }

    fn sha256(bytes: &[u8]) -> [u8; 32] {
        // ダイジェスト値を使用することで、画像の複製を保持せずに安定したバイト単位比較を行える
        Sha256::digest(bytes).into()
    }

    struct TestDirectory {
        path: PathBuf,
    }

    impl TestDirectory {
        fn new() -> Self {
            let unique_id = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "epub-core-package-test-{}-{unique_id}",
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
