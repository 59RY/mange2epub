use std::{path::PathBuf, process::ExitCode};

use clap::{Args, Parser, Subcommand};
use epub_core::{BuildError, BuildReport, BuildRequest, build_epub};

/// 漫画のEPUBファイルを作成するコマンドラインインターフェース。
#[derive(Debug, Parser)]
#[command(name = "manga2epub", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

/// 現在アプリケーションが対応しているコマンド。
#[derive(Debug, Subcommand)]
enum Command {
    /// JPEG画像のディレクトリからEPUBを生成する。
    Build(BuildArguments),
}

/// `build`コマンドが受け取る引数。
#[derive(Args, Debug)]
struct BuildArguments {
    /// ページ画像のJPEGが入ったディレクトリ。
    image_directory: PathBuf,

    /// 生成するEPUBファイルのパス。
    #[arg(short, long)]
    output: PathBuf,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match run(cli.command) {
        Ok(report) => {
            println!(
                "EPUBを生成しました: {} ({}ページ)",
                report.output_path.display(),
                report.page_count
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("エラー: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(command: Command) -> Result<BuildReport, BuildError> {
    // 引数解析はこのcrateで行い、EPUB生成処理はepub-coreに置く。
    match command {
        Command::Build(arguments) => build_epub(&BuildRequest {
            image_directory: arguments.image_directory,
            output_path: arguments.output,
        }),
    }
}

// 単体テストでは、EPUBファイルを生成せずにコマンドラインの契約を確認する。
#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicUsize, Ordering},
    };

    use clap::Parser;

    use super::{BuildArguments, Cli, Command, run};

    static NEXT_TEST_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn parses_the_build_command_with_an_output_path() {
        let cli =
            Cli::try_parse_from(["manga2epub", "build", "./images", "--output", "./book.epub"])
                .unwrap();

        let Command::Build(arguments) = cli.command;
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

    fn write_jpeg(path: PathBuf) {
        // コアの入力処理が画像サイズを取得するには、SOF0セグメントだけで十分である。
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
