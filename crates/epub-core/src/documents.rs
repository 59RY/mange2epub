use std::{error::Error, fmt, io};

use quick_xml::{
    Writer,
    events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event},
};

use crate::{ImageDimensions, PagePlacement, SourceImage, default_page_placement};

const CONTAINER_PATH: &str = "EPUB/package.opf";
const PAGE_CSS_PATH: &str = "styles/page.css";
const NAVIGATION_PATH: &str = "nav.xhtml";

/// EPUBのパッケージ文書に必要な最小限のメタデータ。
///
/// 後の入力処理では、利用者が指定した書誌情報からこの値を作成する。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MinimalMetadata {
    pub title: String,
    pub identifier: String,
    pub language: String,
    pub modified: String,
}

/// 生成した1つのXHTMLコンテンツ文書と、そのEPUB内の相対パス。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageDocument {
    pub path: String,
    pub contents: String,
}

/// OCF ZIPコンテナへ書き込む前に生成するEPUBのテキストリソース。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedDocuments {
    pub container_xml: String,
    pub package_opf: String,
    pub navigation_xhtml: String,
    pub page_css: String,
    pub pages: Vec<PageDocument>,
}

/// EPUBのテキストリソース生成時に発生しうるエラー。
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

/// 順序付けられた画像リストからXHTML、CSS、OPF、コンテナ文書を生成する。
///
/// 最初の画像が共通の論理的なviewportを決める。
/// 生成するEPUB内のパスには、入力画像のパスを意図的に含めない。
/// EPUB内のパスは画像の番号で正規化する。
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
    // `container.xml`は、パッケージ文書の保存先をEPUBリーダーへ伝える。
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
    // パッケージ文書は、メタデータ、manifest、読書順をまとめて持つ。
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
        empty(
            &mut writer,
            "item",
            &[
                ("id", page_id.as_str()),
                ("href", page_path.as_str()),
                ("media-type", "application/xhtml+xml"),
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
        let placement = placement_property(default_page_placement(index));
        empty(
            &mut writer,
            "itemref",
            &[("idref", page_id.as_str()), ("properties", placement)],
        )?;
    }
    end(&mut writer, "spine")?;
    end(&mut writer, "package")?;

    into_string(writer)
}

fn generate_navigation_xhtml(title: &str, language: &str) -> Result<String, DocumentError> {
    // 利用者が定義する目次項目がまだなくても、ナビゲーション文書は必要である。
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
    // 各画像に1つのXHTML文書を割り当てる。
    // すべてのページで、最初の画像から得た共通のviewport寸法を使う。
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
    // このスタイルシートはブラウザの既定値を取り除き、画像をviewport全体に表示する。
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
    // IDは順序付けたページ番号だけを基にするため、安定している。
    format!("page-{index:04}")
}

fn image_id(index: usize) -> String {
    // 画像IDは、XHTMLページのIDと衝突しないよう別の接頭辞を使う。
    format!("image-{index:04}")
}

fn page_path(index: usize) -> String {
    // EPUB内部の名前からは、元の入力ファイル名が分からないようにする。
    format!("pages/page-{index:04}.xhtml")
}

fn image_path(index: usize) -> String {
    // 出力では、この版が対応する正規化済みのJPEG拡張子を常に使う。
    format!("images/image-{index:04}.jpg")
}

fn placement_property(placement: PagePlacement) -> &'static str {
    // コアコードではenumを使い、出力境界でEPUBの語彙へ変換する。
    // この値は、パッケージ文書のspineにある`itemref`要素へ書き出す。
    match placement {
        PagePlacement::Left => "rendition:page-spread-left",
        PagePlacement::Right => "rendition:page-spread-right",
        PagePlacement::Center => "rendition:page-spread-center",
    }
}

fn xml_writer() -> Writer<Vec<u8>> {
    // インデントを付けてもXMLの意味は変わらず、生成文書を確認しやすくできる。
    Writer::new_with_indent(Vec::new(), b' ', 2)
}

fn write_declaration(writer: &mut Writer<Vec<u8>>) -> Result<(), DocumentError> {
    // EPUBのXML文書はUTF-8のため、文字エンコーディングを明示する。
    write_event(
        writer,
        Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)),
    )
}

fn write_doctype(writer: &mut Writer<Vec<u8>>) -> Result<(), DocumentError> {
    // XHTMLコンテンツ文書にはHTMLのdoctypeを使う。
    write_event(writer, Event::DocType(BytesText::new("html")))
}

fn start(
    writer: &mut Writer<Vec<u8>>,
    name: &str,
    attributes: &[(&str, &str)],
) -> Result<(), DocumentError> {
    // 属性値が適切にエスケープされるよう、`quick-xml`で属性を組み立てる。
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
    // XHTMLのメタデータとOPFのmanifest項目には空要素を使う。
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
    // テキストノードも`quick-xml`を通すことで、XMLエスケープを任せる。
    start(writer, name, attributes)?;
    write_event(writer, Event::Text(BytesText::new(text)))?;
    end(writer, name)
}

fn end(writer: &mut Writer<Vec<u8>>, name: &str) -> Result<(), DocumentError> {
    // 対応する終了タグを1つのヘルパーにまとめ、writer呼び出しの重複を減らす。
    write_event(writer, Event::End(BytesEnd::new(name)))
}

fn write_event(writer: &mut Writer<Vec<u8>>, event: Event<'_>) -> Result<(), DocumentError> {
    // 低水準のwriterエラーを、文書生成処理のエラー型へ変換する。
    writer.write_event(event).map_err(DocumentError::WriteXml)
}

fn into_string(writer: Writer<Vec<u8>>) -> Result<String, DocumentError> {
    // XMLはUTF-8のバイト列として出力してから、EPUBのテキストリソースにする。
    String::from_utf8(writer.into_inner()).map_err(DocumentError::InvalidUtf8)
}

// 単体テストでは、後の出力処理がZIPエントリへ書き込む前のテキストリソースを確認する。
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
        assert!(documents.package_opf.contains(
            "<itemref idref=\"page-0000\" properties=\"rendition:page-spread-center\"/>"
        ));
        assert!(
            documents.package_opf.contains(
                "<itemref idref=\"page-0001\" properties=\"rendition:page-spread-right\"/>"
            )
        );
        assert!(
            documents.package_opf.contains(
                "<itemref idref=\"page-0002\" properties=\"rendition:page-spread-left\"/>"
            )
        );
        assert!(
            documents
                .package_opf
                .contains("<item id=\"page-0000\" href=\"pages/page-0000.xhtml\" media-type=\"application/xhtml+xml\"/>")
        );
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
        // 固定メタデータを使うことで、文書テストを将来の入力処理から独立させる。
        MinimalMetadata {
            title: "Untitled".to_owned(),
            identifier: "urn:uuid:00000000-0000-0000-0000-000000000000".to_owned(),
            language: "ja".to_owned(),
            modified: "2026-08-26T00:00:00Z".to_owned(),
        }
    }

    fn images() -> Vec<SourceImage> {
        // 生成する名前はページ番号を使うため、入力パスは意図的に任意の値にしている。
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
