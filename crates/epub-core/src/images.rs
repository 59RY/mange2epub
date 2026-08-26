use std::{
    cmp::Ordering,
    error::Error,
    fmt,
    fs::{self, File},
    io::{self, BufReader, Read},
    path::{Path, PathBuf},
};

/// The pixel dimensions read from a JPEG file without decoding or changing it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ImageDimensions {
    pub width: u32,
    pub height: u32,
}

/// An input JPEG selected for inclusion in the publication.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceImage {
    pub path: PathBuf,
    pub dimensions: ImageDimensions,
}

/// Errors that can occur while collecting JPEG input files.
#[derive(Debug)]
pub enum ImageCollectionError {
    ReadDirectory { path: PathBuf, source: io::Error },
    ReadDirectoryEntry { path: PathBuf, source: io::Error },
    ReadImage { path: PathBuf, source: io::Error },
    InvalidJpeg { path: PathBuf, reason: &'static str },
    NoImages { directory: PathBuf },
}

impl fmt::Display for ImageCollectionError {
    /// Presents each error in a form that the CLI can show directly to a user.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadDirectory { path, .. } => {
                write!(
                    formatter,
                    "could not read image directory: {}",
                    path.display()
                )
            }
            Self::ReadDirectoryEntry { path, .. } => {
                write!(
                    formatter,
                    "could not read a directory entry in: {}",
                    path.display()
                )
            }
            Self::ReadImage { path, .. } => {
                write!(formatter, "could not read image: {}", path.display())
            }
            Self::InvalidJpeg { path, reason } => {
                write!(formatter, "invalid JPEG image {}: {reason}", path.display())
            }
            Self::NoImages { directory } => {
                write!(
                    formatter,
                    "no JPEG images found in: {}",
                    directory.display()
                )
            }
        }
    }
}

impl Error for ImageCollectionError {
    /// Preserves the operating-system error when one caused this higher-level error.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ReadDirectory { source, .. }
            | Self::ReadDirectoryEntry { source, .. }
            | Self::ReadImage { source, .. } => Some(source),
            Self::InvalidJpeg { .. } | Self::NoImages { .. } => None,
        }
    }
}

/// Collects JPEG files directly inside `directory` in deterministic natural filename order.
///
/// Only `.jpg` and `.jpeg` extensions are included, ignoring their ASCII case.
/// The JPEG header is read only far enough to determine the image dimensions.
pub fn collect_jpeg_images(directory: &Path) -> Result<Vec<SourceImage>, ImageCollectionError> {
    let entries =
        fs::read_dir(directory).map_err(|source| ImageCollectionError::ReadDirectory {
            path: directory.to_path_buf(),
            source,
        })?;

    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| ImageCollectionError::ReadDirectoryEntry {
            path: directory.to_path_buf(),
            source,
        })?;
        let path = entry.path();

        if path.is_file() && has_jpeg_extension(&path) {
            paths.push(path);
        }
    }

    paths.sort_by(|left, right| natural_path_compare(left, right));

    if paths.is_empty() {
        return Err(ImageCollectionError::NoImages {
            directory: directory.to_path_buf(),
        });
    }

    paths
        .into_iter()
        .map(|path| {
            let dimensions = read_jpeg_dimensions(&path)?;
            Ok(SourceImage { path, dimensions })
        })
        .collect()
}

fn has_jpeg_extension(path: &Path) -> bool {
    // File extensions are a quick filter;
    // `read_jpeg_dimensions` still validates the contents before the image is accepted.
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("jpg") || extension.eq_ignore_ascii_case("jpeg")
        })
}

fn natural_path_compare(left: &Path, right: &Path) -> Ordering {
    // The filename determines the user-visible order.
    // The complete path breaks otherwise equal filenames deterministically.
    let left_name = left.file_name().unwrap_or_default().to_string_lossy();
    let right_name = right.file_name().unwrap_or_default().to_string_lossy();

    natural_compare(&left_name, &right_name).then_with(|| left.cmp(right))
}

fn natural_compare(left: &str, right: &str) -> Ordering {
    // Compare ASCII digit runs as numbers without parsing an integer.
    // This avoids overflow for unusually long page numbers.
    let left = left.as_bytes();
    let right = right.as_bytes();
    let (mut left_index, mut right_index) = (0, 0);

    while left_index < left.len() && right_index < right.len() {
        if left[left_index].is_ascii_digit() && right[right_index].is_ascii_digit() {
            let left_end = digit_run_end(left, left_index);
            let right_end = digit_run_end(right, right_index);
            let comparison =
                compare_digit_runs(&left[left_index..left_end], &right[right_index..right_end]);

            if comparison != Ordering::Equal {
                return comparison;
            }

            left_index = left_end;
            right_index = right_end;
        } else {
            let comparison = left[left_index].cmp(&right[right_index]);
            if comparison != Ordering::Equal {
                return comparison;
            }

            left_index += 1;
            right_index += 1;
        }
    }

    left.len().cmp(&right.len())
}

fn digit_run_end(value: &[u8], start: usize) -> usize {
    // `start` always points at a digit, so the returned index is after at least 1 byte
    // and can safely be used as a slice boundary.
    value[start..]
        .iter()
        .position(|byte| !byte.is_ascii_digit())
        .map_or(value.len(), |offset| start + offset)
}

fn compare_digit_runs(left: &[u8], right: &[u8]) -> Ordering {
    // Numeric strings compare by their significant digit count first.
    // If their numeric values match, fewer leading zeroes sorts first.
    let left_significant = trim_leading_zeroes(left);
    let right_significant = trim_leading_zeroes(right);

    left_significant
        .len()
        .cmp(&right_significant.len())
        .then_with(|| left_significant.cmp(right_significant))
        .then_with(|| left.len().cmp(&right.len()))
}

fn trim_leading_zeroes(value: &[u8]) -> &[u8] {
    // Keep one zero for an all-zero run
    // so its numeric representation is never an empty slice.
    let first_significant = value
        .iter()
        .position(|byte| *byte != b'0')
        .unwrap_or(value.len());

    if first_significant == value.len() {
        &value[value.len() - 1..]
    } else {
        &value[first_significant..]
    }
}

fn read_jpeg_dimensions(path: &Path) -> Result<ImageDimensions, ImageCollectionError> {
    // JPEG records width and height in a Start Of Frame segment.
    // Reading only up to that segment preserves the input bytes
    // and avoids allocating pixel data.
    let file = File::open(path).map_err(|source| ImageCollectionError::ReadImage {
        path: path.to_path_buf(),
        source,
    })?;
    let mut reader = BufReader::new(file);

    let start = read_u16(&mut reader, path)?;
    if start != 0xffd8 {
        return Err(invalid_jpeg(path, "missing start-of-image marker"));
    }

    loop {
        let marker = next_marker(&mut reader, path)?;

        if is_start_of_frame(marker) {
            let segment_length = read_u16(&mut reader, path)?;
            if segment_length < 8 {
                return Err(invalid_jpeg(path, "start-of-frame segment is too short"));
            }

            let _precision = read_byte(&mut reader, path)?;
            let height = read_u16(&mut reader, path)? as u32;
            let width = read_u16(&mut reader, path)? as u32;

            if width == 0 || height == 0 {
                return Err(invalid_jpeg(
                    path,
                    "image dimensions must be greater than zero",
                ));
            }

            return Ok(ImageDimensions { width, height });
        }

        match marker {
            0xd9 => return Err(invalid_jpeg(path, "missing start-of-frame marker")),
            0xda => {
                return Err(invalid_jpeg(
                    path,
                    "start-of-frame marker appears after image data",
                ));
            }
            0xd8 | 0x01 | 0xd0..=0xd7 => continue,
            _ => {
                let segment_length = read_u16(&mut reader, path)?;
                if segment_length < 2 {
                    return Err(invalid_jpeg(
                        path,
                        "segment length is smaller than its header",
                    ));
                }

                skip_bytes(&mut reader, usize::from(segment_length - 2), path)?;
            }
        }
    }
}

fn next_marker(reader: &mut impl Read, path: &Path) -> Result<u8, ImageCollectionError> {
    // JPEG markers begin with 0xFF. Repeated 0xFF bytes are fill bytes,
    // while 0xFF00 is byte-stuffing and not a marker.
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

fn is_start_of_frame(marker: u8) -> bool {
    // JPEG defines several SOF variants. They all store dimensions in the same initial fields,
    // while the excluded marker values have other meanings.
    matches!(
        marker,
        0xc0..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf
    )
}

fn read_u16(reader: &mut impl Read, path: &Path) -> Result<u16, ImageCollectionError> {
    // JPEG stores multi-byte fields in big-endian byte order.
    let high = read_byte(reader, path)?;
    let low = read_byte(reader, path)?;
    Ok(u16::from_be_bytes([high, low]))
}

fn read_byte(reader: &mut impl Read, path: &Path) -> Result<u8, ImageCollectionError> {
    // Convert a short read into the same path-aware error used for other I/O.
    let mut buffer = [0];
    reader
        .read_exact(&mut buffer)
        .map_err(|source| ImageCollectionError::ReadImage {
            path: path.to_path_buf(),
            source,
        })?;
    Ok(buffer[0])
}

fn skip_bytes(
    reader: &mut impl Read,
    byte_count: usize,
    path: &Path,
) -> Result<(), ImageCollectionError> {
    // Segment lengths are controlled by the input file, so skip in a fixed-size buffer
    // instead of allocating an input-sized temporary vector.
    let mut remaining = byte_count;
    let mut buffer = [0; 1024];

    while remaining > 0 {
        let count = remaining.min(buffer.len());
        reader.read_exact(&mut buffer[..count]).map_err(|source| {
            ImageCollectionError::ReadImage {
                path: path.to_path_buf(),
                source,
            }
        })?;
        remaining -= count;
    }

    Ok(())
}

fn invalid_jpeg(path: &Path, reason: &'static str) -> ImageCollectionError {
    // Keep construction of validation errors uniform and retain the source path.
    ImageCollectionError::InvalidJpeg {
        path: path.to_path_buf(),
        reason,
    }
}

// Unit tests compile only when `cargo test` runs.
// They cover file selection, natural sorting, JPEG header parsing, and invalid input handling.
#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicUsize, Ordering},
    };

    use super::{ImageCollectionError, ImageDimensions, collect_jpeg_images, natural_compare};

    static NEXT_TEST_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn sorts_filenames_in_natural_order() {
        let directory = TestDirectory::new();
        write_jpeg(directory.path().join("page-10.jpg"), 1200, 1800);
        write_jpeg(directory.path().join("page-2.JPG"), 1200, 1800);
        write_jpeg(directory.path().join("page-1.jpeg"), 1200, 1800);
        fs::write(directory.path().join("notes.txt"), "not an image").unwrap();

        let images = collect_jpeg_images(directory.path()).unwrap();
        let names = images
            .iter()
            .map(|image| {
                image
                    .path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect::<Vec<_>>();

        assert_eq!(names, ["page-1.jpeg", "page-2.JPG", "page-10.jpg"]);
    }

    #[test]
    fn reads_jpeg_dimensions_without_decoding_the_image() {
        let directory = TestDirectory::new();
        write_jpeg(directory.path().join("page-1.jpg"), 1200, 1759);

        let images = collect_jpeg_images(directory.path()).unwrap();

        assert_eq!(
            images[0].dimensions,
            ImageDimensions {
                width: 1200,
                height: 1759,
            }
        );
    }

    #[test]
    fn rejects_a_jpeg_extension_with_an_invalid_header() {
        let directory = TestDirectory::new();
        fs::write(directory.path().join("page-1.jpg"), "not a JPEG").unwrap();

        let error = collect_jpeg_images(directory.path()).unwrap_err();

        assert!(matches!(error, ImageCollectionError::InvalidJpeg { .. }));
    }

    #[test]
    fn compares_equal_numbers_with_shorter_zero_padding_first() {
        assert!(natural_compare("page-1.jpg", "page-01.jpg").is_lt());
    }

    struct TestDirectory {
        path: PathBuf,
    }

    impl TestDirectory {
        /// Creates an isolated directory so parallel tests cannot share files.
        fn new() -> Self {
            let unique_id = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "epub-core-images-test-{}-{unique_id}",
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
        /// Removes the temporary fixture directory after each test, including on panic.
        fn drop(&mut self) {
            fs::remove_dir_all(&self.path).unwrap();
        }
    }

    fn write_jpeg(path: PathBuf, width: u16, height: u16) {
        // A SOF0 segment is sufficient for the header reader;
        // no compressed image data is needed by these focused tests.
        let mut bytes = vec![0xff, 0xd8, 0xff, 0xc0, 0x00, 0x11, 0x08];
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&[0x03, 0x01, 0x11, 0x00, 0x02, 0x11, 0x00, 0x03, 0x11, 0x00]);
        bytes.extend_from_slice(&[0xff, 0xd9]);
        fs::write(path, bytes).unwrap();
    }
}
