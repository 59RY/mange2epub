//! EPUB generation logic shared by the command-line application and a future GUI.
//!
//! This crate deliberately does not know how a user supplied its input. Its public
//! API models EPUB concepts and will later expose the build operation itself.

mod documents;
mod images;
mod pages;

pub use documents::{
    DocumentError, GeneratedDocuments, MinimalMetadata, PageDocument, generate_documents,
};
pub use images::{ImageCollectionError, ImageDimensions, SourceImage, collect_jpeg_images};
pub use pages::{PagePlacement, default_page_placement};
