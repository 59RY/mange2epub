use std::{path::PathBuf, process::ExitCode};

use clap::{Args, Parser, Subcommand};
use epub_core::{BuildError, BuildReport, BuildRequest, build_epub};

/// Command-line interface for creating manga EPUB files.
#[derive(Debug, Parser)]
#[command(name = "manga2epub", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

/// Commands that the application currently supports.
#[derive(Debug, Subcommand)]
enum Command {
    /// Build an EPUB from a directory of JPEG images.
    Build(BuildArguments),
}

/// Arguments accepted by the `build` command.
#[derive(Args, Debug)]
struct BuildArguments {
    /// Directory containing JPEG page images.
    image_directory: PathBuf,

    /// Path of the EPUB file to create.
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
    // Argument parsing stays in this crate, while EPUB creation remains in epub-core.
    match command {
        Command::Build(arguments) => build_epub(&BuildRequest {
            image_directory: arguments.image_directory,
            output_path: arguments.output,
        }),
    }
}

// Unit tests verify the command-line contract without creating EPUB files.
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
        // A SOF0 segment is enough for the core input reader to obtain dimensions.
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
