use std::{
    error::Error,
    fmt, fs, io,
    path::{Path, PathBuf},
};

use epub_core::{
    AlternateScript, BuildRequest, CreatorMetadata, PageOverride, PagePlacement,
    PublicationMetadata, TocEntry,
};
use serde::Deserialize;

/// YAML 設定ファイルの読み込み・解釈時に発生しうるエラー
#[derive(Debug)]
pub(crate) enum ConfigError {
    Read {
        path: PathBuf,
        source: io::Error,
    },
    Parse {
        path: PathBuf,
        source: yaml_serde::Error,
    },
    UnsupportedVersion {
        path: PathBuf,
        version: u32,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, .. } => {
                write!(
                    formatter,
                    "could not read configuration file: {}",
                    path.display()
                )
            }
            Self::Parse { path, .. } => {
                write!(
                    formatter,
                    "could not parse configuration file: {}",
                    path.display()
                )
            }
            Self::UnsupportedVersion { path, version } => write!(
                formatter,
                "configuration file has unsupported version {version}: {}",
                path.display()
            ),
        }
    }
}

impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            Self::Parse { source, .. } => Some(source),
            Self::UnsupportedVersion { .. } => None,
        }
    }
}

/// YAML 設定ファイルを読み込み、EPUB 生成処理の入力へ変換する
pub(crate) fn load_build_request(path: &Path) -> Result<BuildRequest, ConfigError> {
    let contents = fs::read_to_string(path).map_err(|source| ConfigError::Read {
        path: path.to_path_buf(),
        source,
    })?;

    parse_build_request(path, &contents)
}

/// 設定ファイルの内容を読み取り、相対パスを設定ファイル基準で解決する
fn parse_build_request(path: &Path, contents: &str) -> Result<BuildRequest, ConfigError> {
    let configuration: BookConfiguration =
        yaml_serde::from_str(contents).map_err(|source| ConfigError::Parse {
            path: path.to_path_buf(),
            source,
        })?;

    if configuration.version != 1 {
        return Err(ConfigError::UnsupportedVersion {
            path: path.to_path_buf(),
            version: configuration.version,
        });
    }

    Ok(configuration.into_build_request(path))
}

/// `book.yaml` 全体の、現在対応している最小スキーマ
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BookConfiguration {
    version: u32,
    output: PathBuf,
    book: BookMetadataConfiguration,
    images: ImageConfiguration,
    #[serde(default)]
    pages: PageConfiguration,
    #[serde(default)]
    toc: TocConfiguration,
}

impl BookConfiguration {
    /// YAML の書誌情報とパスを、コアが受け取る入力構造へ変換する
    fn into_build_request(self, configuration_path: &Path) -> BuildRequest {
        BuildRequest {
            image_directory: resolve_path(configuration_path, self.images.directory),
            image_order: self.images.order,
            output_path: resolve_path(configuration_path, self.output),
            metadata: self.book.into_publication_metadata(),
            page_overrides: self.pages.into_page_overrides(),
            toc_entries: self.toc.into_toc_entries(),
        }
    }
}

/// `images` セクションに記述する入力画像の設定
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ImageConfiguration {
    directory: PathBuf,
    #[serde(default)]
    order: Option<Vec<PathBuf>>,
}

/// `pages` セクションに記述するページ配置の上書き設定
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct PageConfiguration {
    #[serde(default)]
    overrides: Vec<PageOverrideConfiguration>,
}

impl PageConfiguration {
    /// YAML の配置設定を、EPUB 生成用のページ配置へ変換する
    fn into_page_overrides(self) -> Vec<PageOverride> {
        self.overrides
            .into_iter()
            .map(PageOverrideConfiguration::into_page_override)
            .collect()
    }
}

/// `toc` セクションに記述する目次設定
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct TocConfiguration {
    #[serde(default)]
    entries: Vec<TocEntryConfiguration>,
}

impl TocConfiguration {
    /// YAML の目次設定を、EPUB Navigation Document 用の項目へ変換する
    fn into_toc_entries(self) -> Vec<TocEntry> {
        self.entries
            .into_iter()
            .map(TocEntryConfiguration::into_toc_entry)
            .collect()
    }
}

/// `toc.entries` の各要素に記述するラベルとリンク先ページ
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TocEntryConfiguration {
    label: String,
    page: usize,
}

impl TocEntryConfiguration {
    /// YAML の 1 始まりのページ番号を維持して、コアの目次項目へ変換する
    fn into_toc_entry(self) -> TocEntry {
        TocEntry {
            label: self.label,
            page_number: self.page,
        }
    }
}

/// `pages.overrides` の各要素に記述するページ配置
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PageOverrideConfiguration {
    page: usize,
    placement: PagePlacementConfiguration,
}

impl PageOverrideConfiguration {
    /// YAML のページ番号と配置を、EPUB 生成用のページ配置へ変換する
    fn into_page_override(self) -> PageOverride {
        PageOverride {
            page_number: self.page,
            placement: self.placement.into_page_placement(),
        }
    }
}

/// YAML で受け付けるページ配置の名前
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum PagePlacementConfiguration {
    Left,
    Right,
    Center,
}

impl PagePlacementConfiguration {
    /// YAML の配置名を、EPUB 生成処理で使うページ配置へ変換する
    fn into_page_placement(self) -> PagePlacement {
        match self {
            Self::Left => PagePlacement::Left,
            Self::Right => PagePlacement::Right,
            Self::Center => PagePlacement::Center,
        }
    }
}

/// `book` セクションに記述する書誌情報
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BookMetadataConfiguration {
    title: String,
    #[serde(default)]
    title_file_as: Option<String>,
    #[serde(default)]
    creators: Vec<CreatorConfiguration>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    publisher: Option<String>,
    #[serde(default)]
    date: Option<String>,
    #[serde(default)]
    types: Vec<String>,
    #[serde(default)]
    subjects: Vec<String>,
    #[serde(default = "default_language")]
    language: String,
    #[serde(default)]
    identifier: Option<String>,
}

impl BookMetadataConfiguration {
    /// YAML の書誌情報を、EPUB 出力用の書誌情報へ変換する
    fn into_publication_metadata(self) -> PublicationMetadata {
        PublicationMetadata {
            title: self.title,
            title_file_as: self.title_file_as,
            creators: self
                .creators
                .into_iter()
                .map(CreatorConfiguration::into_creator_metadata)
                .collect(),
            description: self.description,
            publisher: self.publisher,
            date: self.date,
            types: self.types,
            subjects: self.subjects,
            language: self.language,
            identifier: self.identifier,
        }
    }
}

/// `creators` の各要素に記述する著者情報
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CreatorConfiguration {
    name: String,
    #[serde(default)]
    file_as: Option<String>,
    #[serde(default)]
    roles: Vec<String>,
    #[serde(default)]
    alternate_scripts: Vec<AlternateScriptConfiguration>,
}

impl CreatorConfiguration {
    /// YAML の著者情報を、EPUB 出力用の著者情報へ変換する
    fn into_creator_metadata(self) -> CreatorMetadata {
        CreatorMetadata {
            name: self.name,
            file_as: self.file_as,
            roles: self.roles,
            alternate_scripts: self
                .alternate_scripts
                .into_iter()
                .map(AlternateScriptConfiguration::into_alternate_script)
                .collect(),
        }
    }
}

/// `alternate_scripts` の各要素に記述する別表記
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AlternateScriptConfiguration {
    lang: String,
    value: String,
}

impl AlternateScriptConfiguration {
    /// YAML の別表記を、EPUB 出力用の別表記へ変換する
    fn into_alternate_script(self) -> AlternateScript {
        AlternateScript {
            value: self.value,
            language: self.lang,
        }
    }
}

/// YAML で省略した書籍言語に使用する既定値
fn default_language() -> String {
    "ja".to_owned()
}

/// 相対パスを設定ファイルの親ディレクトリを基準にしたパスへ変換する
fn resolve_path(configuration_path: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        configuration_path
            .parent()
            .unwrap_or(Path::new("."))
            .join(path)
    }
}

// YAML の入力形式、既定値、設定ファイル固有のエラーを独立して確認する
#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use epub_core::{AlternateScript, CreatorMetadata, PageOverride, PagePlacement, TocEntry};

    use super::{ConfigError, parse_build_request};

    #[test]
    // 複数の著者・役割・別表記と、設定ファイル基準の相対パスを読み取る
    fn parses_a_complete_configuration_file() {
        let configuration_path = Path::new("fixtures/book.yaml");
        let request = parse_build_request(
            configuration_path,
            r#"
version: 1
output: output/book.epub
book:
  title: 書籍のタイトル
  title_file_as: ショセキノタイトル
  creators:
    - name: 著者名
      file_as: チョシャメイ
      roles:
        - aut
        - edt
      alternate_scripts:
        - lang: ja-Kana
          value: チョシャメイ
        - lang: ja-Latn
          value: Choshamei
    - name: 編集者名
  description: 説明文
  publisher: 発行元
  date: "2026-09-01T00:00:00+09:00"
  types:
    - comic
    - image
  subjects:
    - Illustration
    - Fiction
  language: ja
  identifier: urn:test:book
images:
  directory: images
  order:
    - page-02.png
    - page-01.jpg
pages:
  overrides:
    - page: 4
      placement: center
    - page: 2
      placement: left
toc:
  entries:
    - label: 本編
      page: 3
    - label: おまけ
      page: 5
"#,
        )
        .unwrap();

        assert_eq!(
            request.image_directory,
            PathBuf::from("fixtures").join("images")
        );
        assert_eq!(
            request.output_path,
            PathBuf::from("fixtures").join("output/book.epub")
        );
        assert_eq!(
            request.image_order,
            Some(vec![
                PathBuf::from("page-02.png"),
                PathBuf::from("page-01.jpg"),
            ])
        );
        assert_eq!(request.metadata.title, "書籍のタイトル");
        assert_eq!(
            request.metadata.title_file_as.as_deref(),
            Some("ショセキノタイトル")
        );
        assert_eq!(request.metadata.description.as_deref(), Some("説明文"));
        assert_eq!(request.metadata.publisher.as_deref(), Some("発行元"));
        assert_eq!(
            request.metadata.date.as_deref(),
            Some("2026-09-01T00:00:00+09:00")
        );
        assert_eq!(request.metadata.types, ["comic", "image"]);
        assert_eq!(request.metadata.subjects, ["Illustration", "Fiction"]);
        assert_eq!(request.metadata.language, "ja");
        assert_eq!(
            request.metadata.identifier.as_deref(),
            Some("urn:test:book")
        );
        assert_eq!(
            request.metadata.creators,
            vec![
                CreatorMetadata {
                    name: "著者名".to_owned(),
                    file_as: Some("チョシャメイ".to_owned()),
                    roles: vec!["aut".to_owned(), "edt".to_owned()],
                    alternate_scripts: vec![
                        AlternateScript {
                            value: "チョシャメイ".to_owned(),
                            language: "ja-Kana".to_owned(),
                        },
                        AlternateScript {
                            value: "Choshamei".to_owned(),
                            language: "ja-Latn".to_owned(),
                        },
                    ],
                },
                CreatorMetadata {
                    name: "編集者名".to_owned(),
                    file_as: None,
                    roles: Vec::new(),
                    alternate_scripts: Vec::new(),
                },
            ]
        );
        assert_eq!(
            request.page_overrides,
            vec![
                PageOverride {
                    page_number: 4,
                    placement: PagePlacement::Center,
                },
                PageOverride {
                    page_number: 2,
                    placement: PagePlacement::Left,
                },
            ]
        );
        assert_eq!(
            request.toc_entries,
            vec![
                TocEntry {
                    label: "本編".to_owned(),
                    page_number: 3,
                },
                TocEntry {
                    label: "おまけ".to_owned(),
                    page_number: 5,
                },
            ]
        );
    }

    #[test]
    // language を省略した設定には、CLI と同じ既定値の ja を設定する
    fn uses_japanese_as_the_default_language() {
        let request = parse_build_request(
            Path::new("book.yaml"),
            r#"
version: 1
output: book.epub
book:
  title: 書籍のタイトル
images:
  directory: images
"#,
        )
        .unwrap();

        assert_eq!(request.metadata.language, "ja");
        assert_eq!(request.image_order, None);
        assert!(request.page_overrides.is_empty());
        assert!(request.toc_entries.is_empty());
    }

    #[test]
    // 将来のスキーマと混同しないよう、未対応の設定バージョンを明示的に拒否する
    fn rejects_an_unsupported_configuration_version() {
        let path = Path::new("book.yaml");
        let error = parse_build_request(
            path,
            r#"
version: 2
output: book.epub
book:
  title: 書籍のタイトル
images:
  directory: images
"#,
        )
        .unwrap_err();

        assert!(matches!(
            error,
            ConfigError::UnsupportedVersion { path: error_path, version: 2 }
                if error_path == path
        ));
    }

    #[test]
    // YAML のキーを誤記した場合は、静かに無視せず入力エラーとして返す
    fn rejects_an_unknown_configuration_key() {
        let error = parse_build_request(
            Path::new("book.yaml"),
            r#"
version: 1
output: book.epub
book:
  title: 書籍のタイトル
  titel: typo
images:
  directory: images
"#,
        )
        .unwrap_err();

        assert!(matches!(error, ConfigError::Parse { .. }));
    }

    #[test]
    // 配置名の入力を限定し、誤記を既定配置として静かに扱わない
    fn rejects_an_unknown_page_placement() {
        let error = parse_build_request(
            Path::new("book.yaml"),
            r#"
version: 1
output: book.epub
book:
  title: 書籍のタイトル
images:
  directory: images
pages:
  overrides:
    - page: 2
      placement: middle
"#,
        )
        .unwrap_err();

        assert!(matches!(error, ConfigError::Parse { .. }));
    }

    #[test]
    // pages 内のキーも検証し、将来の設定項目を意図せず受け付けない
    fn rejects_an_unknown_page_configuration_key() {
        let error = parse_build_request(
            Path::new("book.yaml"),
            r#"
version: 1
output: book.epub
book:
  title: 書籍のタイトル
images:
  directory: images
pages:
  overrids: []  # テストの性質上、英単語typoでOK
"#,
        )
        .unwrap_err();

        assert!(matches!(error, ConfigError::Parse { .. }));
    }

    #[test]
    // toc.entries 内の誤記も、未使用の設定として見落とさずに拒否する
    fn rejects_an_unknown_toc_entry_key() {
        let error = parse_build_request(
            Path::new("book.yaml"),
            r#"
version: 1
output: book.epub
book:
  title: 書籍のタイトル
images:
  directory: images
toc:
  entries:
    - label: 本編
      pages: 2
"#,
        )
        .unwrap_err();

        assert!(matches!(error, ConfigError::Parse { .. }));
    }
}
