use epub_core::{
    BuildError, BuildReport, DocumentError, ImageCollectionError, InvalidImageReason,
    MetadataError, PackageError, PageOverrideError,
};
use rust_i18n::t;

use crate::{ApplicationError, Locale, config::ConfigError};

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

pub fn build_failed(error: &ApplicationError, locale: Locale) -> String {
    let message = match error {
        ApplicationError::Config(error) => config_error(error, locale),
        ApplicationError::Build(error) => core_build_error(error, locale),
    };

    t!(
        "error.with_prefix",
        locale = locale.as_str(),
        message = message
    )
    .into_owned()
}

/// EPUB コアから返されたエラーを、表示ロケールに対応する文言へ変換する
fn core_build_error(error: &BuildError, locale: Locale) -> String {
    match error {
        BuildError::InvalidMetadata(error) => metadata_error(*error, locale),
        BuildError::CollectImages(error) => image_error(error, locale),
        BuildError::ResolvePagePlacements(error) => page_override_error(*error, locale),
        BuildError::GenerateDocuments(error) => document_error(error, locale),
        BuildError::WritePackage(error) => package_error(error, locale),
        BuildError::TruncateModifiedTime(_) => {
            t!("error.truncate_modified", locale = locale.as_str()).into_owned()
        }
        BuildError::FormatModifiedTime(_) => {
            t!("error.format_modified", locale = locale.as_str()).into_owned()
        }
    }
}

/// ページ配置の上書きに関するエラーを、表示ロケールに対応する文言へ変換する
fn page_override_error(error: PageOverrideError, locale: Locale) -> String {
    let locale = locale.as_str();
    match error {
        PageOverrideError::PageNumberMustBePositive => {
            t!("error.page_number_must_be_positive", locale = locale)
        }
        PageOverrideError::PageOutOfRange {
            page_number,
            page_count,
        } => t!(
            "error.page_number_out_of_range",
            locale = locale,
            page_number = page_number,
            page_count = page_count
        ),
        PageOverrideError::DuplicatePageNumber { page_number } => t!(
            "error.duplicate_page_number",
            locale = locale,
            page_number = page_number
        ),
    }
    .into_owned()
}

/// YAML 設定ファイルに固有のエラーを、表示ロケールに対応する文言へ変換する
fn config_error(error: &ConfigError, locale: Locale) -> String {
    let locale = locale.as_str();
    match error {
        ConfigError::Read { path, .. } => t!(
            "error.read_configuration",
            locale = locale,
            path = path.display()
        ),
        ConfigError::Parse { path, .. } => t!(
            "error.parse_configuration",
            locale = locale,
            path = path.display()
        ),
        ConfigError::UnsupportedVersion { path, version } => t!(
            "error.unsupported_configuration_version",
            locale = locale,
            path = path.display(),
            version = version
        ),
    }
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
        MetadataError::EmptyDate => "error.empty_date",
        MetadataError::InvalidDate => "error.invalid_date",
        MetadataError::EmptyType => "error.empty_type",
        MetadataError::EmptySubject => "error.empty_subject",
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
        ImageCollectionError::EmptyImageOrder => {
            t!("error.empty_image_order", locale = locale)
        }
        ImageCollectionError::DuplicateImage { path } => t!(
            "error.duplicate_image",
            locale = locale,
            path = path.display()
        ),
        ImageCollectionError::UnsupportedImage { path } => t!(
            "error.unsupported_image",
            locale = locale,
            path = path.display()
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
        DocumentError::PagePlacementCountMismatch { .. } => "error.page_placement_count_mismatch",
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

    use epub_core::{
        BuildError, BuildReport, ImageCollectionError, MetadataError, PageOverrideError,
    };

    use crate::{ApplicationError, config::ConfigError};

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
        let error =
            ApplicationError::Build(BuildError::CollectImages(ImageCollectionError::NoImages {
                directory: PathBuf::from("images"),
            }));

        assert_eq!(
            build_failed(&error, Locale::Ja),
            "エラー: 対応画像が見つかりません: images"
        );
        assert_eq!(
            build_failed(&error, Locale::En),
            "Error: no supported images found in: images"
        );

        let error = ApplicationError::Build(BuildError::CollectImages(
            ImageCollectionError::DuplicateImage {
                path: PathBuf::from("images/page.jpg"),
            },
        ));

        assert_eq!(
            build_failed(&error, Locale::Ja),
            "エラー: 同じ画像が明示順序に複数回指定されています: images/page.jpg"
        );
        assert_eq!(
            build_failed(&error, Locale::En),
            "Error: image is specified more than once in the explicit order: images/page.jpg"
        );
    }

    #[test]
    fn translates_a_configuration_error_to_both_locales() {
        let error = ApplicationError::Config(ConfigError::UnsupportedVersion {
            path: PathBuf::from("book.yaml"),
            version: 2,
        });

        assert_eq!(
            build_failed(&error, Locale::Ja),
            "エラー: 未対応の設定ファイルバージョンです: 2 (book.yaml)"
        );
        assert_eq!(
            build_failed(&error, Locale::En),
            "Error: unsupported configuration file version: 2 (book.yaml)"
        );
    }

    #[test]
    fn translates_a_metadata_error_to_both_locales() {
        let error = ApplicationError::Build(BuildError::InvalidMetadata(MetadataError::EmptyTitle));

        assert_eq!(
            build_failed(&error, Locale::Ja),
            "エラー: 書籍のタイトルを空にすることはできません"
        );
        assert_eq!(
            build_failed(&error, Locale::En),
            "Error: book title must not be empty"
        );

        let error =
            ApplicationError::Build(BuildError::InvalidMetadata(MetadataError::InvalidDate));

        assert_eq!(
            build_failed(&error, Locale::Ja),
            "エラー: 出版日時は YYYY-MM-DD または RFC 3339 形式の日時で指定してください"
        );
        assert_eq!(
            build_failed(&error, Locale::En),
            "Error: date must use YYYY-MM-DD or an RFC 3339 date-time"
        );

        let error = ApplicationError::Build(BuildError::ResolvePagePlacements(
            PageOverrideError::PageOutOfRange {
                page_number: 4,
                page_count: 3,
            },
        ));

        assert_eq!(
            build_failed(&error, Locale::Ja),
            "エラー: ページ番号 4 は入力画像のページ数 3 を超えています"
        );
        assert_eq!(
            build_failed(&error, Locale::En),
            "Error: page number 4 exceeds the available image count 3"
        );
    }
}
