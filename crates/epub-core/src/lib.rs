//! コマンドラインアプリケーションと将来のGUIで共有するEPUB生成ロジック。
//!
//! このcrateは、入力をどのUIから受け取ったかを意図的に扱わない。
//! 公開APIはEPUBの概念を表し、今後はビルド処理自体も公開する。

mod build;
mod documents;
mod images;
mod package;
mod pages;

pub use build::{BuildError, BuildReport, BuildRequest, build_epub};
pub use documents::{
    DocumentError, GeneratedDocuments, MinimalMetadata, PageDocument, generate_documents,
};
pub use images::{ImageCollectionError, ImageDimensions, SourceImage, collect_jpeg_images};
pub use package::{PackageError, write_epub};
pub use pages::{PagePlacement, default_page_placement};
