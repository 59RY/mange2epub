use std::{
    cmp::Ordering,
    collections::HashSet,
    error::Error,
    fmt,
    fs::{self, File},
    io::{self, BufReader, Read},
    path::{Path, PathBuf},
};

/// EPUB へ収録できるラスター画像の形式
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImageFormat {
    Jpeg,
    Png,
}

impl ImageFormat {
    /// EPUB 内部で使用する正規化済みの拡張子を返す
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
        }
    }
    /// OPF manifest に出力する MIME type を返す
    pub const fn media_type(self) -> &'static str {
        match self {
            Self::Jpeg => "image/jpeg",
            Self::Png => "image/png",
        }
    }
}

/// 画像ファイルから読み取ったピクセル寸法
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ImageDimensions {
    pub width: u32,
    pub height: u32,
}

/// EPUB へ収録する入力画像
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceImage {
    pub path: PathBuf,
    pub format: ImageFormat,
    pub dimensions: ImageDimensions,
}

/// 画像の構造を検証できなかった理由
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvalidImageReason {
    InvalidHeader,
    InvalidDimensions,
    InvalidStructure,
}

impl fmt::Display for InvalidImageReason {
    // ロケールを扱わない呼び出し元向けに、英語の標準表現を返す
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidHeader => "invalid image header",
            Self::InvalidDimensions => "image dimensions must be greater than zero",
            Self::InvalidStructure => "invalid image structure",
        })
    }
}

/// 入力画像ファイルの収集時に発生しうるエラー
#[derive(Debug)]
pub enum ImageCollectionError {
    ReadDirectory {
        path: PathBuf,
        source: io::Error,
    },
    ReadDirectoryEntry {
        path: PathBuf,
        source: io::Error,
    },
    ReadImage {
        path: PathBuf,
        source: io::Error,
    },
    InvalidImage {
        path: PathBuf,
        reason: InvalidImageReason,
    },
    EmptyImageOrder,
    DuplicateImage {
        path: PathBuf,
    },
    UnsupportedImage {
        path: PathBuf,
    },
    NoImages {
        directory: PathBuf,
    },
}

impl fmt::Display for ImageCollectionError {
    // 画像収集に失敗した経路を、開発者向けの英語メッセージへ整形する
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadDirectory { path, .. } => {
                write!(f, "could not read image directory: {}", path.display())
            }
            Self::ReadDirectoryEntry { path, .. } => {
                write!(f, "could not read a directory entry in: {}", path.display())
            }
            Self::ReadImage { path, .. } => write!(f, "could not read image: {}", path.display()),
            Self::InvalidImage { path, reason } => {
                write!(f, "invalid image {}: {reason}", path.display())
            }
            Self::EmptyImageOrder => write!(f, "explicit image order must not be empty"),
            Self::DuplicateImage { path } => {
                write!(f, "image is specified more than once: {}", path.display())
            }
            Self::UnsupportedImage { path } => {
                write!(f, "unsupported image format: {}", path.display())
            }
            Self::NoImages { directory } => {
                write!(f, "no supported images found in: {}", directory.display())
            }
        }
    }
}

impl Error for ImageCollectionError {
    // OS の入出力エラーだけを原因として公開し、検証エラーは独立したエラーとして扱う
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ReadDirectory { source, .. }
            | Self::ReadDirectoryEntry { source, .. }
            | Self::ReadImage { source, .. } => Some(source),
            Self::InvalidImage { .. }
            | Self::EmptyImageOrder
            | Self::DuplicateImage { .. }
            | Self::UnsupportedImage { .. }
            | Self::NoImages { .. } => None,
        }
    }
}

/// `directory` 直下の対応画像を、決定的な自然順で収集する
///
/// 拡張子の大文字小文字を区別せず、JPEG と PNG を対象にする
pub fn collect_images(directory: &Path) -> Result<Vec<SourceImage>, ImageCollectionError> {
    let entries =
        fs::read_dir(directory).map_err(|source| ImageCollectionError::ReadDirectory {
            path: directory.to_path_buf(),
            source,
        })?;
    let mut images = Vec::new();
    for entry in entries {
        let path = entry
            .map_err(|source| ImageCollectionError::ReadDirectoryEntry {
                path: directory.to_path_buf(),
                source,
            })?
            .path();
        if path.is_file() {
            if let Some(format) = image_format_from_extension(&path) {
                images.push((path, format));
            }
        }
    }
    images.sort_by(|(left, _), (right, _)| natural_path_compare(left, right));
    if images.is_empty() {
        return Err(ImageCollectionError::NoImages {
            directory: directory.to_path_buf(),
        });
    }
    images
        .into_iter()
        .map(|(path, format)| source_image(path, format))
        .collect()
}

/// 指定されたパスだけを、指定順のまま EPUB へ収録する画像として収集する
///
/// 相対パスは `directory` を基準に解決する。入力ディレクトリの走査は行わない。
pub fn collect_images_in_order(
    directory: &Path,
    image_order: &[PathBuf],
) -> Result<Vec<SourceImage>, ImageCollectionError> {
    if image_order.is_empty() {
        return Err(ImageCollectionError::EmptyImageOrder);
    }

    let paths = image_order
        .iter()
        .map(|requested_path| {
            if requested_path.is_absolute() {
                requested_path.clone()
            } else {
                directory.join(requested_path)
            }
        })
        .collect::<Vec<_>>();
    let mut unique_paths = HashSet::with_capacity(paths.len());
    for path in &paths {
        if !unique_paths.insert(path) {
            return Err(ImageCollectionError::DuplicateImage { path: path.clone() });
        }
    }

    paths
        .into_iter()
        .map(|path| {
            let format = image_format_from_extension(&path)
                .ok_or_else(|| ImageCollectionError::UnsupportedImage { path: path.clone() })?;

            source_image(path, format)
        })
        .collect()
}

/// 1 ファイルを検証し、EPUB へ収録する画像情報へ変換する
fn source_image(path: PathBuf, format: ImageFormat) -> Result<SourceImage, ImageCollectionError> {
    Ok(SourceImage {
        dimensions: read_image_dimensions(&path, format)?,
        path,
        format,
    })
}

/// 拡張子から対応する画像形式を判定する
fn image_format_from_extension(path: &Path) -> Option<ImageFormat> {
    let extension = path.extension()?.to_str()?;
    if extension.eq_ignore_ascii_case("jpg") || extension.eq_ignore_ascii_case("jpeg") {
        Some(ImageFormat::Jpeg)
    } else if extension.eq_ignore_ascii_case("png") {
        Some(ImageFormat::Png)
    } else {
        None
    }
}

/// 画像形式に応じて幅と高さだけを読み取る
fn read_image_dimensions(
    path: &Path,
    format: ImageFormat,
) -> Result<ImageDimensions, ImageCollectionError> {
    match format {
        ImageFormat::Jpeg => read_jpeg_dimensions(path),
        ImageFormat::Png => read_png_dimensions(path),
    }
}

/// JPEG の Start Of Frame セグメントから幅と高さを読み取る
fn read_jpeg_dimensions(path: &Path) -> Result<ImageDimensions, ImageCollectionError> {
    let mut reader = BufReader::new(open_image(path)?);
    if read_u16(&mut reader, path)? != 0xffd8 {
        return Err(invalid_image(path, InvalidImageReason::InvalidHeader));
    }
    loop {
        let marker = next_jpeg_marker(&mut reader, path)?;
        if is_start_of_frame(marker) {
            if read_u16(&mut reader, path)? < 8 {
                return Err(invalid_image(path, InvalidImageReason::InvalidStructure));
            }
            let _precision = read_byte(&mut reader, path)?;
            // JPEG の SOF は、精度の直後に高さ、幅の順で寸法を格納する
            let height = u32::from(read_u16(&mut reader, path)?);
            let width = u32::from(read_u16(&mut reader, path)?);
            return dimensions_or_error(path, width, height);
        }
        match marker {
            0xd9 | 0xda => return Err(invalid_image(path, InvalidImageReason::InvalidStructure)),
            0xd8 | 0x01 | 0xd0..=0xd7 => (),
            _ => {
                let length = read_u16(&mut reader, path)?;
                if length < 2 {
                    return Err(invalid_image(path, InvalidImageReason::InvalidStructure));
                }
                skip_bytes(&mut reader, usize::from(length - 2), path)?;
            }
        }
    }
}

/// PNG の IHDR チャンクから幅と高さを読み取る
fn read_png_dimensions(path: &Path) -> Result<ImageDimensions, ImageCollectionError> {
    // PNG の先頭チャンクは IHDR であり、幅と高さを読み取るために必要な範囲だけを読む
    let mut reader = BufReader::new(open_image(path)?);
    let mut header = [0; 24];
    reader
        .read_exact(&mut header)
        .map_err(|source| ImageCollectionError::ReadImage {
            path: path.to_path_buf(),
            source,
        })?;
    let signature = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    if header[..8] != signature || header[8..16] != [0, 0, 0, 13, b'I', b'H', b'D', b'R'] {
        return Err(invalid_image(path, InvalidImageReason::InvalidHeader));
    }
    dimensions_or_error(
        path,
        u32::from_be_bytes(header[16..20].try_into().unwrap()),
        u32::from_be_bytes(header[20..24].try_into().unwrap()),
    )
}

/// 画像ファイルを開く
fn open_image(path: &Path) -> Result<File, ImageCollectionError> {
    File::open(path).map_err(|source| ImageCollectionError::ReadImage {
        path: path.to_path_buf(),
        source,
    })
}

/// 幅と高さが有効であることを確認する
fn dimensions_or_error(
    path: &Path,
    width: u32,
    height: u32,
) -> Result<ImageDimensions, ImageCollectionError> {
    if width == 0 || height == 0 {
        Err(invalid_image(path, InvalidImageReason::InvalidDimensions))
    } else {
        Ok(ImageDimensions { width, height })
    }
}

/// JPEG マーカーを読み進める
fn next_jpeg_marker(reader: &mut impl Read, path: &Path) -> Result<u8, ImageCollectionError> {
    loop {
        if read_byte(reader, path)? != 0xff {
            continue;
        }
        let mut marker = read_byte(reader, path)?;
        while marker == 0xff {
            marker = read_byte(reader, path)?;
        }
        if marker != 0x00 {
            return Ok(marker);
        }
    }
}

/// JPEG の Start Of Frame マーカーであるかを判定する
fn is_start_of_frame(marker: u8) -> bool {
    matches!(marker, 0xc0..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf)
}

/// ビッグエンディアンの16ビット値を読み取る
fn read_u16(reader: &mut impl Read, path: &Path) -> Result<u16, ImageCollectionError> {
    Ok(u16::from_be_bytes([
        read_byte(reader, path)?,
        read_byte(reader, path)?,
    ]))
}

/// 1バイトを読み取る
fn read_byte(reader: &mut impl Read, path: &Path) -> Result<u8, ImageCollectionError> {
    let mut buffer = [0];
    reader
        .read_exact(&mut buffer)
        .map_err(|source| ImageCollectionError::ReadImage {
            path: path.to_path_buf(),
            source,
        })?;
    Ok(buffer[0])
}

/// セグメントの残りを読み飛ばす
fn skip_bytes(
    reader: &mut impl Read,
    mut count: usize,
    path: &Path,
) -> Result<(), ImageCollectionError> {
    let mut buffer = [0; 1024];
    while count > 0 {
        let length = count.min(buffer.len());
        reader.read_exact(&mut buffer[..length]).map_err(|source| {
            ImageCollectionError::ReadImage {
                path: path.to_path_buf(),
                source,
            }
        })?;
        count -= length;
    }
    Ok(())
}

/// 画像構造の検証エラーへパスを付加する
fn invalid_image(path: &Path, reason: InvalidImageReason) -> ImageCollectionError {
    ImageCollectionError::InvalidImage {
        path: path.to_path_buf(),
        reason,
    }
}

/// パスを自然順で比較する
fn natural_path_compare(left: &Path, right: &Path) -> Ordering {
    natural_compare(
        &left.file_name().unwrap_or_default().to_string_lossy(),
        &right.file_name().unwrap_or_default().to_string_lossy(),
    )
    .then_with(|| left.cmp(right))
}

/// ファイル名に含まれる連続数字を数値として扱う
fn natural_compare(left: &str, right: &str) -> Ordering {
    let (left, right) = (left.as_bytes(), right.as_bytes());
    let (mut l, mut r) = (0, 0);
    while l < left.len() && r < right.len() {
        if left[l].is_ascii_digit() && right[r].is_ascii_digit() {
            let le = digit_run_end(left, l);
            let re = digit_run_end(right, r);
            let order = compare_digit_runs(&left[l..le], &right[r..re]);
            if order != Ordering::Equal {
                return order;
            }
            l = le;
            r = re;
        } else {
            let order = left[l].cmp(&right[r]);
            if order != Ordering::Equal {
                return order;
            }
            l += 1;
            r += 1;
        }
    }
    left.len().cmp(&right.len())
}

/// 連続する数字の末尾位置を返す
fn digit_run_end(value: &[u8], start: usize) -> usize {
    value[start..]
        .iter()
        .position(|byte| !byte.is_ascii_digit())
        .map_or(value.len(), |offset| start + offset)
}

/// 数字列を有効桁数、数値、ゼロ埋めの順に比較する
fn compare_digit_runs(left: &[u8], right: &[u8]) -> Ordering {
    let left_length = left.len();
    let right_length = right.len();
    let left = trim_leading_zeroes(left);
    let right = trim_leading_zeroes(right);
    left.len()
        .cmp(&right.len())
        .then_with(|| left.cmp(right))
        .then_with(|| left_length.cmp(&right_length))
}

/// 先行ゼロを取り除く
fn trim_leading_zeroes(value: &[u8]) -> &[u8] {
    let start = value
        .iter()
        .position(|byte| *byte != b'0')
        .unwrap_or(value.len());
    if start == value.len() {
        &value[value.len() - 1..]
    } else {
        &value[start..]
    }
}

#[cfg(test)]
// 画像形式の検出、画像サイズの読み取り、自然順を小さな入力ファイルで検証する
mod tests {
    use super::{
        ImageCollectionError, ImageDimensions, ImageFormat, collect_images,
        collect_images_in_order, natural_compare,
    };
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicUsize, Ordering},
    };
    static NEXT: AtomicUsize = AtomicUsize::new(0);

    #[test]
    // 拡張子の大文字小文字を問わず、JPEG と PNG を1つの自然順で扱える
    fn collects_jpeg_and_png_in_natural_order() {
        let directory = TestDirectory::new();
        write_png(directory.path().join("page-10.PNG"), 1200, 1800);
        write_jpeg(directory.path().join("page-2.JPG"), 1200, 1800);
        write_png(directory.path().join("page-1.png"), 1200, 1759);
        let images = collect_images(directory.path()).unwrap();
        assert_eq!(
            images
                .iter()
                .map(|image| image
                    .path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned())
                .collect::<Vec<_>>(),
            ["page-1.png", "page-2.JPG", "page-10.PNG"]
        );
        assert_eq!(images[0].format, ImageFormat::Png);
        assert_eq!(images[1].format, ImageFormat::Jpeg);
        assert_eq!(
            images[1].dimensions,
            ImageDimensions {
                width: 1200,
                height: 1800
            }
        );
        assert_eq!(
            images[0].dimensions,
            ImageDimensions {
                width: 1200,
                height: 1759
            }
        );
    }

    #[test]
    // 対応拡張子でもヘッダーが不正なら、画像として収集しない
    fn rejects_an_image_extension_with_an_invalid_header() {
        let directory = TestDirectory::new();
        fs::write(directory.path().join("page-1.png"), [0; 24]).unwrap();
        assert!(collect_images(directory.path()).is_err());
    }

    #[test]
    // JPEG と PNG を、ファイル名の自然順ではなく利用者が指定した順に収集する
    fn collects_jpeg_and_png_in_the_explicit_order() {
        let directory = TestDirectory::new();
        write_png(directory.path().join("page-1.png"), 1200, 1759);
        write_jpeg(directory.path().join("page-2.jpg"), 1200, 1800);

        let images = collect_images_in_order(
            directory.path(),
            &[PathBuf::from("page-2.jpg"), PathBuf::from("page-1.png")],
        )
        .unwrap();

        assert_eq!(
            images
                .iter()
                .map(|image| image.path.file_name().unwrap())
                .collect::<Vec<_>>(),
            ["page-2.jpg", "page-1.png"]
        );
        assert_eq!(images[0].format, ImageFormat::Jpeg);
        assert_eq!(images[1].format, ImageFormat::Png);
    }

    #[test]
    // 同じ画像を複数ページとして意図せず収録しないよう、重複指定を拒否する
    fn rejects_a_duplicate_image_in_the_explicit_order() {
        let directory = TestDirectory::new();
        write_jpeg(directory.path().join("page.jpg"), 1200, 1800);

        let error = collect_images_in_order(
            directory.path(),
            &[PathBuf::from("page.jpg"), PathBuf::from("page.jpg")],
        )
        .unwrap_err();

        assert!(matches!(error, ImageCollectionError::DuplicateImage { .. }));
    }

    #[test]
    // 明示順序を指定した場合は、1 枚以上の画像を必要とする
    fn rejects_an_empty_explicit_image_order() {
        let directory = TestDirectory::new();

        let error = collect_images_in_order(directory.path(), &[]).unwrap_err();

        assert!(matches!(error, ImageCollectionError::EmptyImageOrder));
    }

    #[test]
    // 明示順序では非対応ファイルを無視せず、入力エラーとして返す
    fn rejects_an_unsupported_file_in_the_explicit_order() {
        let directory = TestDirectory::new();
        fs::write(directory.path().join("notes.txt"), b"not an image").unwrap();

        let error =
            collect_images_in_order(directory.path(), &[PathBuf::from("notes.txt")]).unwrap_err();

        assert!(matches!(
            error,
            ImageCollectionError::UnsupportedImage { .. }
        ));
    }

    #[test]
    // 同じ数値なら、ゼロ埋めが短いファイル名を先にする
    fn compares_equal_numbers_with_shorter_zero_padding_first() {
        assert!(natural_compare("page-1.png", "page-01.jpg").is_lt());
    }
    struct TestDirectory {
        path: PathBuf,
    }
    impl TestDirectory {
        // テストごとに衝突しない一時ディレクトリを作る
        fn new() -> Self {
            let id = NEXT.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir()
                .join(format!("epub-core-images-test-{}-{id}", std::process::id()));
            fs::create_dir(&path).unwrap();
            Self { path }
        }

        // テスト用に作成したディレクトリを返す
        fn path(&self) -> &Path {
            &self.path
        }
    }
    impl Drop for TestDirectory {
        // テスト後に一時ディレクトリを削除する
        fn drop(&mut self) {
            fs::remove_dir_all(&self.path).unwrap();
        }
    }

    // SOF0 を持つ最小の JPEG ヘッダーを書き込む
    fn write_jpeg(path: PathBuf, width: u16, height: u16) {
        let mut bytes = vec![0xff, 0xd8, 0xff, 0xc0, 0x00, 0x11, 0x08];
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&[
            0x03, 0x01, 0x11, 0x00, 0x02, 0x11, 0x00, 0x03, 0x11, 0x00, 0xff, 0xd9,
        ]);
        fs::write(path, bytes).unwrap();
    }

    // IHDR を持つ最小の PNG ヘッダーを書き込む
    fn write_png(path: PathBuf, width: u32, height: u32) {
        let mut bytes = vec![
            0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13, b'I', b'H', b'D', b'R',
        ];
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&height.to_be_bytes());
        fs::write(path, bytes).unwrap();
    }
}
