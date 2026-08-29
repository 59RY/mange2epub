use std::{path::PathBuf, process::ExitCode};

use clap::{Args, Parser, Subcommand, ValueEnum};
use epub_core::{
    AlternateScript, BuildError, BuildReport, BuildRequest, CreatorMetadata, PublicationMetadata,
    build_epub,
};

mod config;
mod i18n;

rust_i18n::i18n!("locales", fallback = "en");

/// 漫画の EPUB ファイルを作成する CLI
#[derive(Debug, Parser)]
#[command(name = "manga2epub", version, about)]
struct Cli {
    /// 表示言語 / Display locale
    #[arg(long, global = true, value_enum)]
    locale: Option<Locale>,

    #[command(subcommand)]
    command: Command,
}

/// CLI が利用者向けメッセージに使用するロケール
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum Locale {
    En,
    Ja,
}

impl Locale {
    const fn as_str(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Ja => "ja",
        }
    }

    fn from_system_locale(locale: &str) -> Option<Self> {
        let language = locale
            .split(['-', '_', '.', '@'])
            .next()
            .unwrap_or(locale)
            .to_ascii_lowercase();

        match language.as_str() {
            "ja" => Some(Self::Ja),
            "en" => Some(Self::En),
            _ => None,
        }
    }
}

/// 現在アプリケーションが対応しているコマンド
#[derive(Debug, Subcommand)]
enum Command {
    /// 画像ディレクトリまたは YAML 設定ファイルから EPUB を生成する
    Build(BuildArguments),
}

/// `build` コマンドが受け取る引数
#[derive(Args, Debug)]
struct BuildArguments {
    /// YAML 設定ファイルのパス
    #[arg(
        long,
        value_name = "FILE",
        conflicts_with_all = [
            "image_directory",
            "output",
            "title",
            "title_file_as",
            "creator",
            "creator_file_as",
            "creator_role",
            "creator_alternate_script",
            "creator_alternate_script_language",
            "description",
            "publisher",
            "date",
            "types",
            "subjects",
            "language",
            "identifier"
        ]
    )]
    config: Option<PathBuf>,

    /// ページ画像が入ったディレクトリ
    #[arg(required_unless_present = "config")]
    image_directory: Option<PathBuf>,

    /// 生成する EPUB ファイルのパス
    #[arg(short, long, required_unless_present = "config")]
    output: Option<PathBuf>,

    /// 書籍のタイトル
    #[arg(long, required_unless_present = "config")]
    title: Option<String>,

    /// タイトルの読み
    #[arg(long)]
    title_file_as: Option<String>,

    /// 著者名
    #[arg(long)]
    creator: Option<String>,

    /// 著者名の読み
    #[arg(long, requires = "creator")]
    creator_file_as: Option<String>,

    /// 著者の役割
    #[arg(long, requires = "creator")]
    creator_role: Option<String>,

    /// 著者名の別表記
    #[arg(long, requires_all = ["creator", "creator_alternate_script_language"])]
    creator_alternate_script: Option<String>,

    /// 著者名の別表記に対応する言語タグ
    #[arg(long, requires_all = ["creator", "creator_alternate_script"])]
    creator_alternate_script_language: Option<String>,

    /// 書籍の説明文
    #[arg(long)]
    description: Option<String>,

    /// 発行元
    #[arg(long)]
    publisher: Option<String>,

    /// 出版日時
    #[arg(long)]
    date: Option<String>,

    /// 内容の性質またはジャンル
    #[arg(long = "type", value_name = "TEXT")]
    types: Vec<String>,

    /// 内容の主題
    #[arg(long = "subject", value_name = "TEXT")]
    subjects: Vec<String>,

    /// 書籍の言語
    #[arg(long)]
    language: Option<String>,

    /// Primary Identifier
    #[arg(long)]
    identifier: Option<String>,
}

impl BuildArguments {
    /// CLI 引数または YAML 設定ファイルを、ビルド処理の入力へ変換する
    fn into_build_request(self) -> Result<BuildRequest, config::ConfigError> {
        if let Some(path) = self.config {
            return config::load_build_request(&path);
        }

        let alternate_scripts = self
            .creator_alternate_script
            .zip(self.creator_alternate_script_language)
            .map(|(value, language)| AlternateScript { value, language })
            .into_iter()
            .collect();
        let creators = self
            .creator
            .map(|name| CreatorMetadata {
                name,
                file_as: self.creator_file_as,
                roles: self.creator_role.into_iter().collect(),
                alternate_scripts,
            })
            .into_iter()
            .collect();

        Ok(BuildRequest {
            // clap が config 未指定時に必須入力を検証済みであるため、ここでは値が存在する
            image_directory: self
                .image_directory
                .expect("clap requires an image directory without --config"),
            output_path: self
                .output
                .expect("clap requires an output path without --config"),
            metadata: PublicationMetadata {
                title: self.title.expect("clap requires a title without --config"),
                title_file_as: self.title_file_as,
                creators,
                description: self.description,
                publisher: self.publisher,
                date: self.date,
                types: self.types,
                subjects: self.subjects,
                language: self.language.unwrap_or_else(|| "ja".to_owned()),
                identifier: self.identifier,
            },
            page_overrides: Vec::new(),
        })
    }
}

/// CLI 入力処理または EPUB 生成処理で発生しうるエラー
#[derive(Debug)]
enum ApplicationError {
    Config(config::ConfigError),
    Build(BuildError),
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let locale = resolve_locale(cli.locale, sys_locale::get_locale().as_deref());

    match run(cli.command) {
        Ok(report) => {
            println!("{}", i18n::build_succeeded(&report, locale));
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{}", i18n::build_failed(&error, locale));
            ExitCode::FAILURE
        }
    }
}

fn resolve_locale(explicit: Option<Locale>, system_locale: Option<&str>) -> Locale {
    explicit
        .or_else(|| system_locale.and_then(Locale::from_system_locale))
        .unwrap_or(Locale::En)
}

fn run(command: Command) -> Result<BuildReport, ApplicationError> {
    // 引数解析はこの crate で行い、EPUB 生成処理は epub-core に置く
    match command {
        Command::Build(arguments) => {
            let request = arguments
                .into_build_request()
                .map_err(ApplicationError::Config)?;
            build_epub(&request).map_err(ApplicationError::Build)
        }
    }
}

// 単体テストでは、EPUB ファイルを生成せずにコマンドラインの仕様を検証する
#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicUsize, Ordering},
    };

    use clap::Parser;

    use super::{BuildArguments, Cli, Command, Locale, resolve_locale, run};

    static NEXT_TEST_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

    #[test]
    // 必須のタイトルと出力先だけで、読みなどの任意項目を省略して実行できる
    fn parses_the_build_command_with_required_arguments() {
        let cli = Cli::try_parse_from([
            "manga2epub",
            "build",
            "./images",
            "--output",
            "./book.epub",
            "--title",
            "書籍のタイトル",
        ])
        .unwrap();

        let locale = cli.locale;
        let Command::Build(arguments) = cli.command;
        let request = arguments.into_build_request().unwrap();

        assert_eq!(locale, None);
        assert_eq!(request.image_directory.to_string_lossy(), "./images");
        assert_eq!(request.output_path.to_string_lossy(), "./book.epub");
        assert_eq!(request.metadata.title, "書籍のタイトル");
        assert_eq!(request.metadata.language, "ja");
        assert_eq!(request.metadata.title_file_as, None);
        assert!(request.metadata.creators.is_empty());
        assert_eq!(request.metadata.date, None);
        assert!(request.metadata.types.is_empty());
        assert!(request.metadata.subjects.is_empty());
    }

    #[test]
    // EPUB の必須メタデータであるタイトルを、CLI でも必須の入力にする
    fn rejects_the_build_command_without_a_title() {
        let result =
            Cli::try_parse_from(["manga2epub", "build", "./images", "--output", "./book.epub"]);

        assert!(result.is_err());
    }

    #[test]
    // 出力先は生成結果の保存先を明確にするため、引き続き必須とする
    fn rejects_the_build_command_without_an_output_path() {
        let result = Cli::try_parse_from([
            "manga2epub",
            "build",
            "./images",
            "--title",
            "書籍のタイトル",
        ]);

        assert!(result.is_err());
    }

    #[test]
    // YAML 設定ファイルだけを指定する場合は、画像・出力先・タイトルを CLI で求めない
    fn parses_the_build_command_with_a_configuration_file() {
        let cli = Cli::try_parse_from(["manga2epub", "build", "--config", "./book.yaml"]).unwrap();

        let Command::Build(arguments) = cli.command;

        assert_eq!(arguments.config, Some(PathBuf::from("./book.yaml")));
    }

    #[test]
    // 設定ファイルと CLI 個別指定を混在させず、どちらが優先されるかを曖昧にしない
    fn rejects_mixing_a_configuration_file_with_cli_build_arguments() {
        let result = Cli::try_parse_from([
            "manga2epub",
            "build",
            "--config",
            "./book.yaml",
            "--title",
            "書籍のタイトル",
        ]);

        assert!(result.is_err());
    }

    #[test]
    // 別表記は言語タグと一組で扱い、不完全な値をコアへ渡さない
    fn rejects_an_alternate_script_without_its_language() {
        let result = Cli::try_parse_from([
            "manga2epub",
            "build",
            "./images",
            "--output",
            "./book.epub",
            "--title",
            "書籍のタイトル",
            "--creator",
            "著者名",
            "--creator-alternate-script",
            "チョシャメイ",
        ]);

        assert!(result.is_err());
    }

    #[test]
    // すべてのメタデータオプションを、コアが利用する構造へ変換する
    fn parses_the_build_command_with_all_metadata_options() {
        let cli = Cli::try_parse_from([
            "manga2epub",
            "build",
            "./images",
            "--output",
            "./book.epub",
            "--title",
            "書籍のタイトル",
            "--title-file-as",
            "ショセキノタイトル",
            "--creator",
            "著者名",
            "--creator-file-as",
            "チョシャメイ",
            "--creator-role",
            "edt",
            "--creator-alternate-script",
            "チョシャメイ",
            "--creator-alternate-script-language",
            "ja-Kana",
            "--description",
            "説明文",
            "--publisher",
            "発行元",
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
            "https://example.com/books/123",
        ])
        .unwrap();

        let Command::Build(arguments) = cli.command;
        let request = arguments.into_build_request().unwrap();
        let creator = request.metadata.creators.into_iter().next().unwrap();
        let alternate_script = creator.alternate_scripts.into_iter().next().unwrap();

        assert_eq!(
            request.metadata.title_file_as.as_deref(),
            Some("ショセキノタイトル")
        );
        assert_eq!(request.metadata.description.as_deref(), Some("説明文"));
        assert_eq!(request.metadata.publisher.as_deref(), Some("発行元"));
        assert_eq!(
            request.metadata.date.as_deref(),
            Some("2026-08-31T15:00:00Z")
        );
        assert_eq!(request.metadata.types, ["comic", "image"]);
        assert_eq!(request.metadata.subjects, ["Illustration", "Fiction"]);
        assert_eq!(request.metadata.language, "ja");
        assert_eq!(
            request.metadata.identifier.as_deref(),
            Some("https://example.com/books/123")
        );
        assert_eq!(creator.name, "著者名");
        assert_eq!(creator.file_as.as_deref(), Some("チョシャメイ"));
        assert_eq!(creator.roles, ["edt"]);
        assert_eq!(alternate_script.value, "チョシャメイ");
        assert_eq!(alternate_script.language, "ja-Kana");
    }

    #[test]
    fn build_command_delegates_to_the_core_builder() {
        let directory = TestDirectory::new();
        write_jpeg(directory.path().join("page-1.jpg"));
        let output = directory.path().join("book.epub");

        let report = run(Command::Build(build_arguments(
            directory.path().to_path_buf(),
            output.clone(),
        )))
        .unwrap();

        assert_eq!(report.output_path, output);
        assert_eq!(report.page_count, 1);
        assert!(report.output_path.is_file());
    }

    #[test]
    fn parses_an_english_display_locale() {
        let cli = Cli::try_parse_from([
            "manga2epub",
            "--locale",
            "en",
            "build",
            "./images",
            "--output",
            "./book.epub",
            "--title",
            "書籍のタイトル",
        ])
        .unwrap();

        assert_eq!(cli.locale, Some(Locale::En));
    }

    #[test]
    fn selects_the_display_locale_in_priority_order() {
        assert_eq!(resolve_locale(Some(Locale::En), Some("ja_JP")), Locale::En);
        assert_eq!(resolve_locale(None, Some("ja_JP.UTF-8")), Locale::Ja);
        assert_eq!(resolve_locale(None, Some("en-US")), Locale::En);
        assert_eq!(resolve_locale(None, Some("fr-FR")), Locale::En);
        assert_eq!(resolve_locale(None, None), Locale::En);
    }

    /// ビルド実行テスト用の、必須項目だけを持つ CLI 引数を作る
    fn build_arguments(image_directory: PathBuf, output: PathBuf) -> BuildArguments {
        BuildArguments {
            config: None,
            image_directory: Some(image_directory),
            output: Some(output),
            title: Some("書籍のタイトル".to_owned()),
            title_file_as: None,
            creator: None,
            creator_file_as: None,
            creator_role: None,
            creator_alternate_script: None,
            creator_alternate_script_language: None,
            description: None,
            publisher: None,
            date: None,
            types: Vec::new(),
            subjects: Vec::new(),
            language: None,
            identifier: None,
        }
    }

    fn write_jpeg(path: PathBuf) {
        // SOF0 セグメントだけで、コアの入力処理が画像サイズを取得できる
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
            let path = std::env::temp_dir()
                .join(format!("epub-cli-test-{}-{unique_id}", std::process::id()));
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
