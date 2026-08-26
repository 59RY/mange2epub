use std::{error::Error, fmt, io};

use quick_xml::{
    Writer,
    events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event},
};

use crate::{ImageDimensions, PagePlacement, SourceImage, default_page_placement};

const CONTAINER_PATH: &str = "EPUB/package.opf";
const PAGE_CSS_PATH: &str = "styles/page.css";
const NAVIGATION_PATH: &str = "nav.xhtml";

/// The minimum metadata EPUB requires for a package document.
///
/// Later input layers will create this value from user-provided book metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MinimalMetadata {
    pub title: String,
    pub identifier: String,
    pub language: String,
    pub modified: String,
}

/// One generated XHTML content document and its EPUB-relative path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageDocument {
    pub path: String,
    pub contents: String,
}

/// EPUB text resources generated before they are written into an OCF ZIP container.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedDocuments {
    pub container_xml: String,
    pub package_opf: String,
    pub navigation_xhtml: String,
    pub page_css: String,
    pub pages: Vec<PageDocument>,
}

/// Errors that can occur while generating EPUB text resources.
#[derive(Debug)]
pub enum DocumentError {
    NoPages,
    WriteXml(io::Error),
    InvalidUtf8(std::string::FromUtf8Error),
}

impl fmt::Display for DocumentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoPages => write!(formatter, "cannot generate EPUB documents without pages"),
            Self::WriteXml(_) => write!(formatter, "could not write an EPUB XML document"),
            Self::InvalidUtf8(_) => write!(formatter, "generated XML was not valid UTF-8"),
        }
    }
}

impl Error for DocumentError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::NoPages => None,
            Self::WriteXml(source) => Some(source),
            Self::InvalidUtf8(source) => Some(source),
        }
    }
}

/// Generates the XHTML, CSS, OPF, and container documents for an ordered image list.
///
/// The first image establishes the shared logical viewport. The image source paths
/// are intentionally absent from generated EPUB paths, which are normalized by index.
pub fn generate_documents(
    images: &[SourceImage],
    metadata: &MinimalMetadata,
) -> Result<GeneratedDocuments, DocumentError> {
    let viewport = images.first().ok_or(DocumentError::NoPages)?.dimensions;
    let pages = images
        .iter()
        .enumerate()
        .map(|(index, image)| {
            generate_page_document(index, viewport, &metadata.title, &metadata.language, image)
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(GeneratedDocuments {
        container_xml: generate_container_xml()?,
        package_opf: generate_package_opf(images.len(), metadata)?,
        navigation_xhtml: generate_navigation_xhtml(&metadata.title, &metadata.language)?,
        page_css: page_css(),
        pages,
    })
}

fn generate_container_xml() -> Result<String, DocumentError> {
    // `container.xml` tells an EPUB reader where the package document is stored.
    let mut writer = xml_writer();
    write_declaration(&mut writer)?;

    start(
        &mut writer,
        "container",
        &[
            ("xmlns", "urn:oasis:names:tc:opendocument:xmlns:container"),
            ("version", "1.0"),
        ],
    )?;
    start(&mut writer, "rootfiles", &[])?;
    empty(
        &mut writer,
        "rootfile",
        &[
            ("full-path", CONTAINER_PATH),
            ("media-type", "application/oebps-package+xml"),
        ],
    )?;
    end(&mut writer, "rootfiles")?;
    end(&mut writer, "container")?;

    into_string(writer)
}

fn generate_package_opf(
    page_count: usize,
    metadata: &MinimalMetadata,
) -> Result<String, DocumentError> {
    // The package document owns metadata, the manifest, and the reading order.
    let mut writer = xml_writer();
    write_declaration(&mut writer)?;

    start(
        &mut writer,
        "package",
        &[
            ("xmlns", "http://www.idpf.org/2007/opf"),
            ("xmlns:dc", "http://purl.org/dc/elements/1.1/"),
            ("version", "3.0"),
            ("unique-identifier", "pub-id"),
            ("prefix", "rendition: http://www.idpf.org/vocab/rendition/#"),
        ],
    )?;

    start(&mut writer, "metadata", &[])?;
    text_element(
        &mut writer,
        "dc:identifier",
        &[("id", "pub-id")],
        &metadata.identifier,
    )?;
    text_element(&mut writer, "dc:title", &[], &metadata.title)?;
    text_element(&mut writer, "dc:language", &[], &metadata.language)?;
    text_element(
        &mut writer,
        "meta",
        &[("property", "dcterms:modified")],
        &metadata.modified,
    )?;
    text_element(
        &mut writer,
        "meta",
        &[("property", "rendition:layout")],
        "pre-paginated",
    )?;
    text_element(
        &mut writer,
        "meta",
        &[("property", "rendition:spread")],
        "landscape",
    )?;
    end(&mut writer, "metadata")?;

    start(&mut writer, "manifest", &[])?;
    empty(
        &mut writer,
        "item",
        &[
            ("id", "nav"),
            ("href", NAVIGATION_PATH),
            ("media-type", "application/xhtml+xml"),
            ("properties", "nav"),
        ],
    )?;
    empty(
        &mut writer,
        "item",
        &[
            ("id", "page-css"),
            ("href", PAGE_CSS_PATH),
            ("media-type", "text/css"),
        ],
    )?;

    for index in 0..page_count {
        let page_id = page_id(index);
        let page_path = page_path(index);
        let placement = placement_property(default_page_placement(index));
        empty(
            &mut writer,
            "item",
            &[
                ("id", page_id.as_str()),
                ("href", page_path.as_str()),
                ("media-type", "application/xhtml+xml"),
                ("properties", placement),
            ],
        )?;

        let image_id = image_id(index);
        let image_path = image_path(index);
        if index == 0 {
            empty(
                &mut writer,
                "item",
                &[
                    ("id", image_id.as_str()),
                    ("href", image_path.as_str()),
                    ("media-type", "image/jpeg"),
                    ("properties", "cover-image"),
                ],
            )?;
        } else {
            empty(
                &mut writer,
                "item",
                &[
                    ("id", image_id.as_str()),
                    ("href", image_path.as_str()),
                    ("media-type", "image/jpeg"),
                ],
            )?;
        }
    }
    end(&mut writer, "manifest")?;

    start(
        &mut writer,
        "spine",
        &[("page-progression-direction", "rtl")],
    )?;
    for index in 0..page_count {
        let page_id = page_id(index);
        empty(&mut writer, "itemref", &[("idref", page_id.as_str())])?;
    }
    end(&mut writer, "spine")?;
    end(&mut writer, "package")?;

    into_string(writer)
}

fn generate_navigation_xhtml(title: &str, language: &str) -> Result<String, DocumentError> {
    // A navigation document is required even before user-defined table-of-contents entries exist.
    let mut writer = xml_writer();
    write_declaration(&mut writer)?;
    write_doctype(&mut writer)?;

    start(
        &mut writer,
        "html",
        &[
            ("xmlns", "http://www.w3.org/1999/xhtml"),
            ("xmlns:epub", "http://www.idpf.org/2007/ops"),
            ("xml:lang", language),
        ],
    )?;
    start(&mut writer, "head", &[])?;
    text_element(&mut writer, "title", &[], title)?;
    end(&mut writer, "head")?;
    start(&mut writer, "body", &[])?;
    start(&mut writer, "nav", &[("epub:type", "toc")])?;
    start(&mut writer, "ol", &[])?;
    start(&mut writer, "li", &[])?;
    text_element(
        &mut writer,
        "a",
        &[("href", "pages/page-0000.xhtml")],
        "Start",
    )?;
    end(&mut writer, "li")?;
    end(&mut writer, "ol")?;
    end(&mut writer, "nav")?;
    end(&mut writer, "body")?;
    end(&mut writer, "html")?;

    into_string(writer)
}

fn generate_page_document(
    index: usize,
    viewport: ImageDimensions,
    title: &str,
    language: &str,
    _image: &SourceImage,
) -> Result<PageDocument, DocumentError> {
    // Each image receives one XHTML document.
    // All pages use the first image's shared viewport dimensions.
    let mut writer = xml_writer();
    write_declaration(&mut writer)?;
    write_doctype(&mut writer)?;

    start(
        &mut writer,
        "html",
        &[
            ("xmlns", "http://www.w3.org/1999/xhtml"),
            ("xml:lang", language),
        ],
    )?;
    start(&mut writer, "head", &[])?;
    text_element(&mut writer, "title", &[], title)?;
    empty(
        &mut writer,
        "meta",
        &[
            ("name", "viewport"),
            (
                "content",
                format!("width={}, height={}", viewport.width, viewport.height).as_str(),
            ),
        ],
    )?;
    empty(
        &mut writer,
        "link",
        &[
            ("rel", "stylesheet"),
            ("type", "text/css"),
            ("href", "../styles/page.css"),
        ],
    )?;
    end(&mut writer, "head")?;
    start(&mut writer, "body", &[])?;
    let image_href = format!("../{}", image_path(index));
    empty(
        &mut writer,
        "img",
        &[("src", image_href.as_str()), ("alt", "")],
    )?;
    end(&mut writer, "body")?;
    end(&mut writer, "html")?;

    Ok(PageDocument {
        path: page_path(index),
        contents: into_string(writer)?,
    })
}

fn page_css() -> String {
    // This stylesheet removes browser defaults and lets an image occupy its viewport.
    [
        "html, body {",
        "  width: 100%;",
        "  height: 100%;",
        "  margin: 0;",
        "  padding: 0;",
        "  overflow: hidden;",
        "}",
        "",
        "img {",
        "  display: block;",
        "  width: 100%;",
        "  height: 100%;",
        "}",
        "",
    ]
    .join("\n")
}

fn page_id(index: usize) -> String {
    // IDs remain stable because they are based only on the ordered page index.
    format!("page-{index:04}")
}

fn image_id(index: usize) -> String {
    // Image IDs use a separate prefix to avoid collisions with XHTML page IDs.
    format!("image-{index:04}")
}

fn page_path(index: usize) -> String {
    // EPUB-internal names do not expose the original input filename.
    format!("pages/page-{index:04}.xhtml")
}

fn image_path(index: usize) -> String {
    // The output always uses the normalized JPEG extension supported by this release.
    format!("images/image-{index:04}.jpg")
}

fn placement_property(placement: PagePlacement) -> &'static str {
    // Keep EPUB vocabulary at the output boundary while core code uses an enum.
    match placement {
        PagePlacement::Left => "rendition:page-spread-left",
        PagePlacement::Right => "rendition:page-spread-right",
        PagePlacement::Center => "rendition:page-spread-center",
    }
}

fn xml_writer() -> Writer<Vec<u8>> {
    // Indentation keeps generated documents inspectable without affecting XML semantics.
    Writer::new_with_indent(Vec::new(), b' ', 2)
}

fn write_declaration(writer: &mut Writer<Vec<u8>>) -> Result<(), DocumentError> {
    // EPUB XML documents are UTF-8, so declare that encoding explicitly.
    write_event(
        writer,
        Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)),
    )
}

fn write_doctype(writer: &mut Writer<Vec<u8>>) -> Result<(), DocumentError> {
    // XHTML content documents use the HTML doctype.
    write_event(writer, Event::DocType(BytesText::new("html")))
}

fn start(
    writer: &mut Writer<Vec<u8>>,
    name: &str,
    attributes: &[(&str, &str)],
) -> Result<(), DocumentError> {
    // Build attributes through `quick-xml` so their values are escaped correctly.
    let mut element = BytesStart::new(name);
    for (key, value) in attributes {
        element.push_attribute((*key, *value));
    }
    write_event(writer, Event::Start(element))
}

fn empty(
    writer: &mut Writer<Vec<u8>>,
    name: &str,
    attributes: &[(&str, &str)],
) -> Result<(), DocumentError> {
    // Empty elements are used for XHTML metadata and OPF manifest entries.
    let mut element = BytesStart::new(name);
    for (key, value) in attributes {
        element.push_attribute((*key, *value));
    }
    write_event(writer, Event::Empty(element))
}

fn text_element(
    writer: &mut Writer<Vec<u8>>,
    name: &str,
    attributes: &[(&str, &str)],
    text: &str,
) -> Result<(), DocumentError> {
    // Text nodes also pass through `quick-xml`, which handles XML escaping.
    start(writer, name, attributes)?;
    write_event(writer, Event::Text(BytesText::new(text)))?;
    end(writer, name)
}

fn end(writer: &mut Writer<Vec<u8>>, name: &str) -> Result<(), DocumentError> {
    // Keeping matching end tags in one helper reduces repetitive writer calls.
    write_event(writer, Event::End(BytesEnd::new(name)))
}

fn write_event(writer: &mut Writer<Vec<u8>>, event: Event<'_>) -> Result<(), DocumentError> {
    // Convert low-level writer failures into the document generator's error type.
    writer.write_event(event).map_err(DocumentError::WriteXml)
}

fn into_string(writer: Writer<Vec<u8>>) -> Result<String, DocumentError> {
    // XML is emitted as UTF-8 bytes before it becomes an EPUB text resource.
    String::from_utf8(writer.into_inner()).map_err(DocumentError::InvalidUtf8)
}

// Unit tests exercise text resources before a later output layer writes them to ZIP entries.
#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{MinimalMetadata, generate_documents};
    use crate::{ImageDimensions, SourceImage};

    #[test]
    fn generates_expected_fixed_layout_documents() {
        let documents = generate_documents(&images(), &metadata()).unwrap();

        assert!(
            documents
                .container_xml
                .contains("full-path=\"EPUB/package.opf\"")
        );
        assert!(
            documents
                .package_opf
                .contains("unique-identifier=\"pub-id\"")
        );
        assert!(
            documents
                .package_opf
                .contains("<meta property=\"rendition:layout\">pre-paginated</meta>")
        );
        assert!(
            documents
                .package_opf
                .contains("<meta property=\"rendition:spread\">landscape</meta>")
        );
        assert!(
            documents
                .package_opf
                .contains("page-progression-direction=\"rtl\"")
        );
        assert!(documents.package_opf.contains("properties=\"cover-image\""));
        assert!(
            documents
                .package_opf
                .contains("rendition:page-spread-center")
        );
        assert!(
            documents
                .package_opf
                .contains("rendition:page-spread-right")
        );
        assert!(documents.package_opf.contains("rendition:page-spread-left"));
        assert!(documents.navigation_xhtml.contains("epub:type=\"toc\""));
        assert_eq!(documents.pages.len(), 3);
        assert!(
            documents.pages[0]
                .contents
                .contains("width=1200, height=1759")
        );
        assert!(
            documents.pages[0]
                .contents
                .contains("../images/image-0000.jpg")
        );
    }

    #[test]
    fn escapes_metadata_text() {
        let mut metadata = metadata();
        metadata.title = "A & B < C".to_owned();

        let documents = generate_documents(&images(), &metadata).unwrap();

        assert!(documents.package_opf.contains("A &amp; B &lt; C"));
        assert!(documents.navigation_xhtml.contains("A &amp; B &lt; C"));
    }

    #[test]
    fn rejects_an_empty_image_list() {
        let error = generate_documents(&[], &metadata()).unwrap_err();

        assert!(matches!(error, super::DocumentError::NoPages));
    }

    fn metadata() -> MinimalMetadata {
        // Fixed metadata keeps document tests independent from a future input layer.
        MinimalMetadata {
            title: "Untitled".to_owned(),
            identifier: "urn:uuid:00000000-0000-0000-0000-000000000000".to_owned(),
            language: "ja".to_owned(),
            modified: "2026-08-26T00:00:00Z".to_owned(),
        }
    }

    fn images() -> Vec<SourceImage> {
        // Source paths are deliberately arbitrary because generated names use page indexes.
        (0..3)
            .map(|index| SourceImage {
                path: PathBuf::from(format!("source-{index}.jpg")),
                dimensions: ImageDimensions {
                    width: 1200,
                    height: 1759,
                },
            })
            .collect()
    }
}
