//! コマンドラインアプリケーションと将来の GUI で共有する EPUB 生成ロジック
//!
//! この crate は、入力をどの UI から受け取ったかを意図的に扱わない。
//! 公開 API は EPUB の概念を表し、今後はビルド処理自体も公開する。

mod build;
mod documents;
mod images;
mod metadata;
mod package;
mod pages;

pub use build::{BuildError, BuildReport, BuildRequest, build_epub};
pub use documents::{
    DocumentError, GeneratedDocuments, MinimalMetadata, PageDocument, generate_documents,
};
pub use images::{
    ImageCollectionError, ImageDimensions, ImageFormat, InvalidImageReason, SourceImage,
    collect_images,
};
pub use metadata::{AlternateScript, CreatorMetadata, MetadataError, PublicationMetadata};
pub use package::{PackageError, write_epub};
pub use pages::{
    PageOverride, PageOverrideError, PagePlacement, default_page_placement, resolve_page_placements,
};
