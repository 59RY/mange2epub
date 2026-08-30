use std::{error::Error, fmt, io};

use quick_xml::{
    Writer,
    events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event},
};

use crate::{
    CreatorMetadata, ImageDimensions, PagePlacement, PublicationMetadata, SourceImage, TocEntry,
    TocError, validate_toc_entries,
};

const CONTAINER_PATH: &str = "EPUB/package.opf";
const PAGE_CSS_PATH: &str = "styles/page.css";
const NAVIGATION_PATH: &str = "nav.xhtml";

/// EPUB のパッケージ文書に必要な最小限のメタデータ
///
/// 後の入力処理では、利用者が指定した書誌情報からこの値を作成する
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MinimalMetadata {
    pub title: String,
    pub title_file_as: Option<String>,
    pub creators: Vec<CreatorMetadata>,
    pub description: Option<String>,
    pub publisher: Option<String>,
    pub date: Option<String>,
    pub types: Vec<String>,
    pub subjects: Vec<String>,
    pub identifier: String,
    pub language: String,
    pub modified: String,
}

impl MinimalMetadata {
    /// 利用者が指定した書誌情報と、ビルド時に決まる値から EPUB 出力用の値を作る
    ///
    /// `identifier` には、利用者の指定値または自動生成した UUID を呼び出し側で渡す。
    pub fn from_publication(
        metadata: &PublicationMetadata,
        identifier: String,
        modified: String,
    ) -> Self {
        Self {
            title: metadata.title.clone(),
            title_file_as: metadata.title_file_as.clone(),
            creators: metadata.creators.clone(),
            description: metadata.description.clone(),
            publisher: metadata.publisher.clone(),
            date: metadata.date.clone(),
            types: metadata.types.clone(),
            subjects: metadata.subjects.clone(),
            identifier,
            language: metadata.language.clone(),
            modified,
        }
    }
}

/// 生成した1つの XHTML コンテンツ文書と、その EPUB 内の相対パス
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageDocument {
    pub path: String,
    pub contents: String,
}

/// OCF ZIP コンテナへ書き込む前に生成する EPUB のテキストリソース。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedDocuments {
    pub container_xml: String,
    pub package_opf: String,
    pub navigation_xhtml: String,
    pub page_css: String,
    pub pages: Vec<PageDocument>,
}

/// EPUB のテキストリソース生成時に発生しうるエラー
#[derive(Debug)]
pub enum DocumentError {
    NoPages,
    PagePlacementCountMismatch {
        image_count: usize,
        placement_count: usize,
    },
    InvalidToc(TocError),
    WriteXml(io::Error),
    InvalidUtf8(std::string::FromUtf8Error),
}

impl fmt::Display for DocumentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoPages => write!(formatter, "cannot generate EPUB documents without pages"),
            Self::PagePlacementCountMismatch {
                image_count,
                placement_count,
            } => write!(
                formatter,
                "cannot generate documents for {image_count} images with {placement_count} page placements"
            ),
            Self::InvalidToc(error) => write!(formatter, "{error}"),
            Self::WriteXml(_) => write!(formatter, "could not write an EPUB XML document"),
            Self::InvalidUtf8(_) => write!(formatter, "generated XML was not valid UTF-8"),
        }
    }
}

impl Error for DocumentError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::NoPages => None,
            Self::PagePlacementCountMismatch { .. } => None,
            Self::InvalidToc(error) => Some(error),
            Self::WriteXml(source) => Some(source),
            Self::InvalidUtf8(source) => Some(source),
        }
    }
}

/// 順序付けられた画像リストから XHTML、CSS、OPF、コンテナ文書を生成する
///
/// - 最初の画像が共通の論理的な viewport を決める
/// - 生成する EPUB 内のパスには、入力画像のパスを意図的に含めない
/// - EPUB 内のパスは画像の番号で正規化する
/// - ページ配置の数は画像数と一致している必要がある
/// - 目次のページ番号は並べ替え後の画像に対する 1 始まりの番号として扱う
pub fn generate_documents(
    images: &[SourceImage],
    metadata: &MinimalMetadata,
    placements: &[PagePlacement],
    toc_entries: &[TocEntry],
) -> Result<GeneratedDocuments, DocumentError> {
    let viewport = images.first().ok_or(DocumentError::NoPages)?.dimensions;
    if images.len() != placements.len() {
        return Err(DocumentError::PagePlacementCountMismatch {
            image_count: images.len(),
            placement_count: placements.len(),
        });
    }
    validate_toc_entries(images.len(), toc_entries).map_err(DocumentError::InvalidToc)?;
    let pages = images
        .iter()
        .enumerate()
        .map(|(index, image)| {
            generate_page_document(index, viewport, &metadata.title, &metadata.language, image)
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(GeneratedDocuments {
        container_xml: generate_container_xml()?,
        package_opf: generate_package_opf(images, metadata, placements)?,
        navigation_xhtml: generate_navigation_xhtml(
            &metadata.title,
            &metadata.language,
            toc_entries,
        )?,
        page_css: page_css(),
        pages,
    })
}

fn generate_container_xml() -> Result<String, DocumentError> {
    // `container.xml` は、パッケージ文書の保存先を EPUB 各種ビューアーに伝える
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
    images: &[SourceImage],
    metadata: &MinimalMetadata,
    placements: &[PagePlacement],
) -> Result<String, DocumentError> {
    // パッケージ文書は、メタデータ、manifest、読書順をまとめて持つ
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
    text_element(&mut writer, "dc:title", &[("id", "title")], &metadata.title)?;
    write_optional_refinement(
        &mut writer,
        "#title",
        "file-as",
        metadata.title_file_as.as_deref(),
    )?;
    write_creator_metadata(&mut writer, &metadata.creators)?;
    write_optional_dc_element(
        &mut writer,
        "dc:description",
        metadata.description.as_deref(),
    )?;
    write_optional_dc_element(&mut writer, "dc:publisher", metadata.publisher.as_deref())?;
    write_optional_dc_element(&mut writer, "dc:date", metadata.date.as_deref())?;
    for value in &metadata.types {
        text_element(&mut writer, "dc:type", &[], value)?;
    }
    for subject in &metadata.subjects {
        text_element(&mut writer, "dc:subject", &[], subject)?;
    }
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
    let cover_image_id = image_id(0);
    empty(
        &mut writer,
        "meta",
        &[("name", "cover"), ("content", cover_image_id.as_str())],
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

    for (index, image) in images.iter().enumerate() {
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
        let image_path = image_path(index, image.format);
        if index == 0 {
            empty(
                &mut writer,
                "item",
                &[
                    ("id", image_id.as_str()),
                    ("href", image_path.as_str()),
                    ("media-type", image.format.media_type()),
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
                    ("media-type", image.format.media_type()),
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
    for (index, placement) in placements.iter().copied().enumerate() {
        let page_id = page_id(index);
        let placement = placement_property(placement);
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

/// 著者と、それぞれの著者を対象にした refinement 要素を出力する
fn write_creator_metadata(
    writer: &mut Writer<Vec<u8>>,
    creators: &[CreatorMetadata],
) -> Result<(), DocumentError> {
    for (index, creator) in creators.iter().enumerate() {
        let creator_id = format!("creator-{index:04}");
        let creator_reference = format!("#{creator_id}");

        text_element(writer, "dc:creator", &[("id", &creator_id)], &creator.name)?;
        write_optional_refinement(
            writer,
            &creator_reference,
            "file-as",
            creator.file_as.as_deref(),
        )?;

        if creator.roles.is_empty() {
            write_creator_role(writer, &creator_reference, "aut")?;
        } else {
            for role in &creator.roles {
                write_creator_role(writer, &creator_reference, role)?;
            }
        }

        for alternate_script in &creator.alternate_scripts {
            text_element(
                writer,
                "meta",
                &[
                    ("property", "alternate-script"),
                    ("refines", &creator_reference),
                    ("xml:lang", &alternate_script.language),
                ],
                &alternate_script.value,
            )?;
        }
    }

    Ok(())
}

/// 著者の役割を表す EPUB refinement 要素を出力する
fn write_creator_role(
    writer: &mut Writer<Vec<u8>>,
    creator_reference: &str,
    role: &str,
) -> Result<(), DocumentError> {
    text_element(
        writer,
        "meta",
        &[
            ("property", "role"),
            ("refines", creator_reference),
            ("scheme", "marc:relators"),
        ],
        role,
    )
}

/// 指定された値を、既存要素を対象とする refinement として出力する
fn write_optional_refinement(
    writer: &mut Writer<Vec<u8>>,
    refines: &str,
    property: &str,
    value: Option<&str>,
) -> Result<(), DocumentError> {
    if let Some(value) = value {
        text_element(
            writer,
            "meta",
            &[("property", property), ("refines", refines)],
            value,
        )?;
    }

    Ok(())
}

/// 指定された値を任意の Dublin Core 要素として出力する
fn write_optional_dc_element(
    writer: &mut Writer<Vec<u8>>,
    name: &str,
    value: Option<&str>,
) -> Result<(), DocumentError> {
    if let Some(value) = value {
        text_element(writer, name, &[], value)?;
    }

    Ok(())
}

fn generate_navigation_xhtml(
    title: &str,
    language: &str,
    toc_entries: &[TocEntry],
) -> Result<String, DocumentError> {
    // 目次を省略した場合も、書籍タイトルで表紙へリンクする項目を生成する
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
    if toc_entries.is_empty() {
        write_navigation_entry(&mut writer, title, 0)?;
    } else {
        for entry in toc_entries {
            // 目次項目は検証済みであるため、1 始まりから安全に変換できる
            write_navigation_entry(&mut writer, &entry.label, entry.page_number - 1)?;
        }
    }
    end(&mut writer, "ol")?;
    end(&mut writer, "nav")?;
    end(&mut writer, "body")?;
    end(&mut writer, "html")?;

    into_string(writer)
}

/// 目次項目を、対象ページの正規化された XHTML パスとともに出力する
fn write_navigation_entry(
    writer: &mut Writer<Vec<u8>>,
    label: &str,
    page_index: usize,
) -> Result<(), DocumentError> {
    let page_path = page_path(page_index);
    start(writer, "li", &[])?;
    text_element(writer, "a", &[("href", &page_path)], label)?;
    end(writer, "li")
}

fn generate_page_document(
    index: usize,
    viewport: ImageDimensions,
    title: &str,
    language: &str,
    image: &SourceImage,
) -> Result<PageDocument, DocumentError> {
    // 各画像に1つの XHTML 文書を割り当てる
    // すべてのページで、最初の画像から得た共通の viewport 寸法を使用する
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
    let image_href = format!("../{}", image_path(index, image.format));
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
    // この CSS の主旨:
    // ブラウザの既定値を取り除き、画像を viewport 全体に表示する
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
    // ID は順序付けたページ番号だけを基にする
    format!("page-{index:04}")
}

fn image_id(index: usize) -> String {
    // 画像 ID は別の接頭辞を使用する。XHTML ページの ID と衝突しないため
    format!("image-{index:04}")
}

fn page_path(index: usize) -> String {
    // EPUB 内部の名前からは、元の入力ファイル名が分からないようにする
    format!("pages/page-{index:04}.xhtml")
}

fn image_path(index: usize, format: crate::ImageFormat) -> String {
    // 出力では、画像形式に対応する正規化済みの拡張子を使用する
    format!("images/image-{index:04}.{}", format.extension())
}

fn placement_property(placement: PagePlacement) -> &'static str {
    // コアコードでは enum を使い、出力境界で EPUB の語彙へ変換する
    // この値は、パッケージ文書の spine にある `itemref` 要素へ書き出す
    match placement {
        PagePlacement::Left => "rendition:page-spread-left",
        PagePlacement::Right => "rendition:page-spread-right",
        PagePlacement::Center => "rendition:page-spread-center",
    }
}

fn xml_writer() -> Writer<Vec<u8>> {
    // インデントを付与（視認性向上）
    Writer::new_with_indent(Vec::new(), b' ', 2)
}

fn write_declaration(writer: &mut Writer<Vec<u8>>) -> Result<(), DocumentError> {
    // 文字エンコーディングを明示
    write_event(
        writer,
        Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)),
    )
}

fn write_doctype(writer: &mut Writer<Vec<u8>>) -> Result<(), DocumentError> {
    // HTML の doctype を使用する
    write_event(writer, Event::DocType(BytesText::new("html")))
}

fn start(
    writer: &mut Writer<Vec<u8>>,
    name: &str,
    attributes: &[(&str, &str)],
) -> Result<(), DocumentError> {
    // `quick-xml` で属性を組み立てる（属性値が適切にエスケープされるために）
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
    // XHTML のメタデータと OPF の manifest 項目には空要素を使用する
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
    // テキストノードも `quick-xml` を通すことで、XML エスケープを任せる
    start(writer, name, attributes)?;
    write_event(writer, Event::Text(BytesText::new(text)))?;
    end(writer, name)
}

fn end(writer: &mut Writer<Vec<u8>>, name: &str) -> Result<(), DocumentError> {
    // 対応する終了タグを1つのヘルパーにまとめ、writer 呼び出しの重複を減らす
    write_event(writer, Event::End(BytesEnd::new(name)))
}

fn write_event(writer: &mut Writer<Vec<u8>>, event: Event<'_>) -> Result<(), DocumentError> {
    // 低水準の writer エラーを、文書生成処理のエラー型へ変換する
    writer.write_event(event).map_err(DocumentError::WriteXml)
}

fn into_string(writer: Writer<Vec<u8>>) -> Result<String, DocumentError> {
    // XML は UTF-8 のバイト列として出力してから、EPUB のテキストリソースにする
    String::from_utf8(writer.into_inner()).map_err(DocumentError::InvalidUtf8)
}

// 単体テストでは、後の出力処理が ZIP エントリへ書き込む前のテキストリソースを確認する
#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{DocumentError, MinimalMetadata, generate_documents};
    use crate::{
        AlternateScript, CreatorMetadata, ImageDimensions, ImageFormat, PagePlacement,
        PublicationMetadata, SourceImage, TocEntry, TocError, default_page_placement,
    };

    #[test]
    fn generates_expected_fixed_layout_documents() {
        let documents =
            generate_documents(&images(), &metadata(), &default_placements(3), &[]).unwrap();

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
                .contains("<meta name=\"cover\" content=\"image-0000\"/>")
        );
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
        assert!(
            documents
                .navigation_xhtml
                .contains("<a href=\"pages/page-0000.xhtml\">Untitled</a>"),
            "{}",
            documents.navigation_xhtml
        );
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

        let documents =
            generate_documents(&images(), &metadata, &default_placements(3), &[]).unwrap();

        assert!(documents.package_opf.contains("A &amp; B &lt; C"));
        assert!(documents.navigation_xhtml.contains("A &amp; B &lt; C"));
    }

    #[test]
    // 指定順を維持し、各ページ番号を正規化された XHTML パスへ変換する
    fn generates_requested_navigation_entries() {
        let entries = vec![
            TocEntry {
                label: "導入".to_owned(),
                page_number: 2,
            },
            TocEntry {
                label: "本編 & おまけ".to_owned(),
                page_number: 3,
            },
        ];

        let documents =
            generate_documents(&images(), &metadata(), &default_placements(3), &entries).unwrap();

        let first_entry = documents
            .navigation_xhtml
            .find("<a href=\"pages/page-0001.xhtml\">導入</a>")
            .unwrap();
        let second_entry = documents
            .navigation_xhtml
            .find("<a href=\"pages/page-0002.xhtml\">本編 &amp; おまけ</a>")
            .unwrap();

        assert!(first_entry < second_entry);
        assert!(!documents.navigation_xhtml.contains(">Untitled</a>"));
    }

    #[test]
    // 入力画像の範囲を超えるリンクを持つ Navigation Document は生成しない
    fn rejects_a_navigation_entry_out_of_range() {
        let entries = vec![TocEntry {
            label: "本編".to_owned(),
            page_number: 4,
        }];

        let error = generate_documents(&images(), &metadata(), &default_placements(3), &entries)
            .unwrap_err();

        assert!(matches!(
            error,
            DocumentError::InvalidToc(TocError::PageOutOfRange {
                page_number: 4,
                page_count: 3,
            })
        ));
    }

    #[test]
    // 画像ごとに拡張子と MIME type を出し分け、XHTML の参照先も一致させる
    fn uses_each_image_format_in_the_manifest_and_page_document() {
        let mut images = images();
        images[0].format = ImageFormat::Png;

        let documents =
            generate_documents(&images, &metadata(), &default_placements(images.len()), &[])
                .unwrap();

        assert!(documents.package_opf.contains(
            "<item id=\"image-0000\" href=\"images/image-0000.png\" media-type=\"image/png\" properties=\"cover-image\"/>"
        ));
        assert!(
            documents.pages[0]
                .contents
                .contains("../images/image-0000.png")
        );
        assert!(documents.package_opf.contains(
            "<item id=\"image-0001\" href=\"images/image-0001.jpg\" media-type=\"image/jpeg\"/>"
        ));
    }

    #[test]
    // 指定した書誌情報を、OPF の基本要素と refinement 要素へ分けて出力する
    fn generates_requested_publication_metadata() {
        let metadata = MinimalMetadata::from_publication(
            &publication_metadata(),
            "https://example.com/books/123".to_owned(),
            "2026-08-27T00:00:00Z".to_owned(),
        );

        let documents =
            generate_documents(&images(), &metadata, &default_placements(3), &[]).unwrap();

        assert!(documents.package_opf.contains(
            "<dc:identifier id=\"pub-id\">https://example.com/books/123</dc:identifier>"
        ));
        assert!(
            documents
                .package_opf
                .contains("<dc:title id=\"title\">書籍のタイトル</dc:title>")
        );
        assert!(
            documents.package_opf.contains(
                "<meta property=\"file-as\" refines=\"#title\">ショセキノタイトル</meta>"
            )
        );
        assert!(
            documents
                .package_opf
                .contains("<dc:creator id=\"creator-0000\">著者名</dc:creator>")
        );
        assert!(
            documents.package_opf.contains(
                "<meta property=\"file-as\" refines=\"#creator-0000\">チョシャメイ</meta>"
            )
        );
        assert!(documents.package_opf.contains(
            "<meta property=\"role\" refines=\"#creator-0000\" scheme=\"marc:relators\">aut</meta>"
        ));
        assert!(documents.package_opf.contains(
            "<meta property=\"role\" refines=\"#creator-0000\" scheme=\"marc:relators\">edt</meta>"
        ));
        assert!(documents.package_opf.contains(
            "<meta property=\"alternate-script\" refines=\"#creator-0000\" xml:lang=\"ja-Kana\">チョシャメイ</meta>"
        ));
        assert!(documents.package_opf.contains(
            "<meta property=\"alternate-script\" refines=\"#creator-0000\" xml:lang=\"ja-Latn\">Choshamei</meta>"
        ));
        assert!(
            documents
                .package_opf
                .contains("<dc:creator id=\"creator-0001\">編集者名</dc:creator>")
        );
        assert!(documents.package_opf.contains(
            "<meta property=\"role\" refines=\"#creator-0001\" scheme=\"marc:relators\">aut</meta>"
        ));
        assert!(
            documents
                .package_opf
                .contains("<dc:description>説明文</dc:description>")
        );
        assert!(
            documents
                .package_opf
                .contains("<dc:publisher>発行元</dc:publisher>")
        );
        assert!(
            documents
                .package_opf
                .contains("<dc:date>2026-08-31T15:00:00Z</dc:date>")
        );
        assert_eq!(documents.package_opf.matches("<dc:type>").count(), 2);
        assert!(documents.package_opf.contains("<dc:type>comic</dc:type>"));
        assert!(documents.package_opf.contains("<dc:type>image</dc:type>"));
        assert_eq!(documents.package_opf.matches("<dc:subject>").count(), 2);
        assert!(
            documents
                .package_opf
                .contains("<dc:subject>Illustration</dc:subject>")
        );
        assert!(
            documents
                .package_opf
                .contains("<dc:subject>Fiction</dc:subject>")
        );
        assert!(
            documents
                .package_opf
                .contains("<dc:language>ja</dc:language>")
        );
    }

    #[test]
    fn rejects_an_empty_image_list() {
        let error = generate_documents(&[], &metadata(), &[], &[]).unwrap_err();

        assert!(matches!(error, DocumentError::NoPages));
    }

    #[test]
    // 画像数と配置数が異なる場合は、不完全な spine を生成せずに拒否する
    fn rejects_page_placement_count_mismatches() {
        let error = generate_documents(&images(), &metadata(), &[], &[]).unwrap_err();

        assert!(matches!(
            error,
            DocumentError::PagePlacementCountMismatch {
                image_count: 3,
                placement_count: 0,
            }
        ));
    }

    /// 既定配置を使用する文書生成テスト用のページ配置一覧を作る
    fn default_placements(page_count: usize) -> Vec<PagePlacement> {
        (0..page_count).map(default_page_placement).collect()
    }

    fn metadata() -> MinimalMetadata {
        // 固定メタデータを使用することで、文書テストを将来の入力処理から独立させる
        MinimalMetadata {
            title: "Untitled".to_owned(),
            title_file_as: None,
            creators: Vec::new(),
            description: None,
            publisher: None,
            date: None,
            types: Vec::new(),
            subjects: Vec::new(),
            identifier: "urn:uuid:00000000-0000-0000-0000-000000000000".to_owned(),
            language: "ja".to_owned(),
            modified: "2026-08-26T00:00:00Z".to_owned(),
        }
    }

    /// 任意項目をすべて含む書誌情報を作る
    fn publication_metadata() -> PublicationMetadata {
        PublicationMetadata {
            title: "書籍のタイトル".to_owned(),
            title_file_as: Some("ショセキノタイトル".to_owned()),
            creators: vec![
                CreatorMetadata {
                    name: "著者名".to_owned(),
                    file_as: Some("チョシャメイ".to_owned()),
                    roles: vec!["aut".to_owned(), "edt".to_owned()],
                    alternate_scripts: vec![
                        AlternateScript {
                            value: "チョシャメイ".to_owned(),
                            language: "ja-Kana".to_owned(),
                        },
                        AlternateScript {
                            value: "Choshamei".to_owned(),
                            language: "ja-Latn".to_owned(),
                        },
                    ],
                },
                CreatorMetadata {
                    name: "編集者名".to_owned(),
                    file_as: None,
                    roles: Vec::new(),
                    alternate_scripts: Vec::new(),
                },
            ],
            description: Some("説明文".to_owned()),
            publisher: Some("発行元".to_owned()),
            date: Some("2026-08-31T15:00:00Z".to_owned()),
            types: vec!["comic".to_owned(), "image".to_owned()],
            subjects: vec!["Illustration".to_owned(), "Fiction".to_owned()],
            language: "ja".to_owned(),
            identifier: Some("https://example.com/books/123".to_owned()),
        }
    }

    fn images() -> Vec<SourceImage> {
        // 生成する名前はページ番号を使用するため、入力パスは意図的に任意の値にしている
        (0..3)
            .map(|index| SourceImage {
                path: PathBuf::from(format!("source-{index}.jpg")),
                format: ImageFormat::Jpeg,
                dimensions: ImageDimensions {
                    width: 1200,
                    height: 1759,
                },
            })
            .collect()
    }
}
