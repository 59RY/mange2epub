use epub_core::{
    BuildError, BuildReport, DocumentError, ImageCollectionError, InvalidImageReason,
    MetadataError, PackageError,
};
use rust_i18n::t;

use crate::Locale;

pub fn build_succeeded(report: &BuildReport, locale: Locale) -> String {
    let key = if report.page_count == 1 {
        "build.succeeded_one_page"
    } else {
        "build.succeeded_multiple_pages"
    };

    t!(
        key,
        locale = locale.as_str(),
        path = report.output_path.display(),
        count = report.page_count
    )
    .into_owned()
}

pub fn build_failed(error: &BuildError, locale: Locale) -> String {
    let message = match error {
        BuildError::InvalidMetadata(error) => metadata_error(*error, locale),
        BuildError::CollectImages(error) => image_error(error, locale),
        BuildError::GenerateDocuments(error) => document_error(error, locale),
        BuildError::WritePackage(error) => package_error(error, locale),
        BuildError::TruncateModifiedTime(_) => {
            t!("error.truncate_modified", locale = locale.as_str()).into_owned()
        }
        BuildError::FormatModifiedTime(_) => {
            t!("error.format_modified", locale = locale.as_str()).into_owned()
        }
    };

    t!(
        "error.with_prefix",
        locale = locale.as_str(),
        message = message
    )
    .into_owned()
}

/// 構造化された書誌情報エラーを、表示ロケールに対応する文言へ変換する
fn metadata_error(error: MetadataError, locale: Locale) -> String {
    let key = match error {
        MetadataError::EmptyTitle => "error.empty_title",
        MetadataError::EmptyTitleFileAs => "error.empty_title_file_as",
        MetadataError::EmptyCreatorName => "error.empty_creator_name",
        MetadataError::EmptyCreatorFileAs => "error.empty_creator_file_as",
        MetadataError::EmptyCreatorRole => "error.empty_creator_role",
        MetadataError::EmptyCreatorAlternateScript => "error.empty_creator_alternate_script",
        MetadataError::EmptyCreatorAlternateScriptLanguage => {
            "error.empty_creator_alternate_script_language"
        }
        MetadataError::EmptyDescription => "error.empty_description",
        MetadataError::EmptyPublisher => "error.empty_publisher",
        MetadataError::EmptyLanguage => "error.empty_language",
        MetadataError::EmptyIdentifier => "error.empty_identifier",
    };

    t!(key, locale = locale.as_str()).into_owned()
}

fn image_error(error: &ImageCollectionError, locale: Locale) -> String {
    let locale = locale.as_str();
    match error {
        ImageCollectionError::ReadDirectory { path, .. } => t!(
            "error.read_directory",
            locale = locale,
            path = path.display()
        ),
        ImageCollectionError::ReadDirectoryEntry { path, .. } => t!(
            "error.read_directory_entry",
            locale = locale,
            path = path.display()
        ),
        ImageCollectionError::ReadImage { path, .. } => {
            t!("error.read_image", locale = locale, path = path.display())
        }
        ImageCollectionError::InvalidImage { path, reason } => t!(
            "error.invalid_image",
            locale = locale,
            path = path.display(),
            reason = invalid_image_reason(*reason, locale)
        ),
        ImageCollectionError::NoImages { directory } => t!(
            "error.no_images",
            locale = locale,
            path = directory.display()
        ),
    }
    .into_owned()
}

/// 画像形式に依存しない検証エラーを、表示ロケールに対応する文言へ変換する
fn invalid_image_reason(reason: InvalidImageReason, locale: &str) -> String {
    let key = match reason {
        InvalidImageReason::InvalidHeader => "image.invalid_header",
        InvalidImageReason::InvalidDimensions => "image.invalid_dimensions",
        InvalidImageReason::InvalidStructure => "image.invalid_structure",
    };
    t!(key, locale = locale).into_owned()
}

fn document_error(error: &DocumentError, locale: Locale) -> String {
    let key = match error {
        DocumentError::NoPages => "error.no_pages",
        DocumentError::WriteXml(_) => "error.write_xml",
        DocumentError::InvalidUtf8(_) => "error.invalid_utf8",
    };
    t!(key, locale = locale.as_str()).into_owned()
}

fn package_error(error: &PackageError, locale: Locale) -> String {
    let locale = locale.as_str();
    match error {
        PackageError::CreateOutput { path, .. } => t!(
            "error.create_output",
            locale = locale,
            path = path.display()
        ),
        PackageError::ReadImage { path, .. } => t!(
            "error.read_image_for_output",
            locale = locale,
            path = path.display()
        ),
        PackageError::WriteArchive(_) => t!("error.write_archive", locale = locale),
        PackageError::Zip(_) => t!("error.create_zip", locale = locale),
        PackageError::PageCountMismatch {
            image_count,
            page_count,
        } => t!(
            "error.page_count_mismatch",
            locale = locale,
            image_count = image_count,
            page_count = page_count
        ),
    }
    .into_owned()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use epub_core::{BuildError, BuildReport, ImageCollectionError, MetadataError};

    use super::{Locale, build_failed, build_succeeded};

    #[test]
    fn translates_a_success_message_to_both_locales() {
        let report = BuildReport {
            output_path: PathBuf::from("book.epub"),
            page_count: 2,
        };

        assert_eq!(
            build_succeeded(&report, Locale::Ja),
            "EPUB を生成しました: book.epub (2 ページ)"
        );
        assert_eq!(
            build_succeeded(&report, Locale::En),
            "Generated EPUB: book.epub (2 pages)"
        );
    }

    #[test]
    fn translates_a_core_error_to_both_locales() {
        let error = BuildError::CollectImages(ImageCollectionError::NoImages {
            directory: PathBuf::from("images"),
        });

        assert_eq!(
            build_failed(&error, Locale::Ja),
            "エラー: 対応画像が見つかりません: images"
        );
        assert_eq!(
            build_failed(&error, Locale::En),
            "Error: no supported images found in: images"
        );
    }

    #[test]
    fn translates_a_metadata_error_to_both_locales() {
        let error = BuildError::InvalidMetadata(MetadataError::EmptyTitle);

        assert_eq!(
            build_failed(&error, Locale::Ja),
            "エラー: 書籍のタイトルを空にすることはできません"
        );
        assert_eq!(
            build_failed(&error, Locale::En),
            "Error: book title must not be empty"
        );
    }
}
