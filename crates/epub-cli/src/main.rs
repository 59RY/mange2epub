use std::{path::PathBuf, process::ExitCode};

use clap::{Args, Parser, Subcommand, ValueEnum};
use epub_core::{BuildError, BuildReport, BuildRequest, PublicationMetadata, build_epub};

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
    /// JPEG 画像のディレクトリから EPUB を生成する
    Build(BuildArguments),
}

/// `build` コマンドが受け取る引数
#[derive(Args, Debug)]
struct BuildArguments {
    /// ページ画像の JPEG が入ったディレクトリ
    image_directory: PathBuf,

    /// 生成する EPUB ファイルのパス
    #[arg(short, long)]
    output: PathBuf,
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

fn run(command: Command) -> Result<BuildReport, BuildError> {
    // 引数解析はこの crate で行い、EPUB 生成処理は epub-core に置く
    match command {
        Command::Build(arguments) => build_epub(&BuildRequest {
            image_directory: arguments.image_directory,
            output_path: arguments.output,
            // 書誌情報の入力オプションを追加するまでは、既存の既定値を明示して渡す
            metadata: PublicationMetadata::new("Untitled".to_owned()),
        }),
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
    fn parses_the_build_command_with_an_output_path() {
        let cli =
            Cli::try_parse_from(["manga2epub", "build", "./images", "--output", "./book.epub"])
                .unwrap();

        let Command::Build(arguments) = cli.command;
        assert_eq!(cli.locale, None);
        assert_eq!(arguments.image_directory.to_string_lossy(), "./images");
        assert_eq!(arguments.output.to_string_lossy(), "./book.epub");
    }

    #[test]
    fn rejects_the_build_command_without_an_output_path() {
        let result = Cli::try_parse_from(["manga2epub", "build", "./images"]);

        assert!(result.is_err());
    }

    #[test]
    fn build_command_delegates_to_the_core_builder() {
        let directory = TestDirectory::new();
        write_jpeg(directory.path().join("page-1.jpg"));
        let output = directory.path().join("book.epub");

        let report = run(Command::Build(BuildArguments {
            image_directory: directory.path().to_path_buf(),
            output: output.clone(),
        }))
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
