use std::{error::Error, fmt};

/// 利用者が指定する書誌情報
///
/// CLI や将来の設定ファイルはこの構造を作る。
/// UUID や更新日時など、EPUB 生成時に決まる値はここに含めない。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicationMetadata {
    pub title: String,
    pub title_file_as: Option<String>,
    pub creator: Option<CreatorMetadata>,
    pub description: Option<String>,
    pub publisher: Option<String>,
    pub language: String,
    pub identifier: Option<String>,
}

/// 1名の著者に関する書誌情報
///
/// 複数著者や複数の役割は、利用例と入力形式を確定してから追加する。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreatorMetadata {
    pub name: String,
    pub file_as: Option<String>,
    pub role: Option<String>,
    pub alternate_script: Option<AlternateScript>,
}

/// 著者名の別の文字体系による表記
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AlternateScript {
    pub value: String,
    pub language: String,
}

/// 書誌情報の検証時に発生しうるエラー
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MetadataError {
    EmptyTitle,
    EmptyTitleFileAs,
    EmptyCreatorName,
    EmptyCreatorFileAs,
    EmptyCreatorRole,
    EmptyCreatorAlternateScript,
    EmptyCreatorAlternateScriptLanguage,
    EmptyDescription,
    EmptyPublisher,
    EmptyLanguage,
    EmptyIdentifier,
}

impl PublicationMetadata {
    /// 必須のタイトルを受け取り、既定の言語を設定して書誌情報を作る
    pub fn new(title: String) -> Self {
        Self {
            title,
            title_file_as: None,
            creator: None,
            description: None,
            publisher: None,
            language: "ja".to_owned(),
            identifier: None,
        }
    }

    /// EPUB へ出力する前に、指定された値が空でないことを確認する
    ///
    /// 値の前後の空白は検査だけに使用し、値そのものは変更しない。
    pub fn validate(&self) -> Result<(), MetadataError> {
        require_value(&self.title, MetadataError::EmptyTitle)?;
        require_optional_value(&self.title_file_as, MetadataError::EmptyTitleFileAs)?;
        require_optional_value(&self.description, MetadataError::EmptyDescription)?;
        require_optional_value(&self.publisher, MetadataError::EmptyPublisher)?;
        require_value(&self.language, MetadataError::EmptyLanguage)?;
        require_optional_value(&self.identifier, MetadataError::EmptyIdentifier)?;

        if let Some(creator) = &self.creator {
            creator.validate()?;
        }

        Ok(())
    }
}

impl CreatorMetadata {
    /// 著者に指定された値が空でないことを確認する
    fn validate(&self) -> Result<(), MetadataError> {
        require_value(&self.name, MetadataError::EmptyCreatorName)?;
        require_optional_value(&self.file_as, MetadataError::EmptyCreatorFileAs)?;
        require_optional_value(&self.role, MetadataError::EmptyCreatorRole)?;

        if let Some(alternate_script) = &self.alternate_script {
            require_value(
                &alternate_script.value,
                MetadataError::EmptyCreatorAlternateScript,
            )?;
            require_value(
                &alternate_script.language,
                MetadataError::EmptyCreatorAlternateScriptLanguage,
            )?;
        }

        Ok(())
    }
}

impl fmt::Display for MetadataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let field = match self {
            Self::EmptyTitle => "title",
            Self::EmptyTitleFileAs => "title file-as",
            Self::EmptyCreatorName => "creator name",
            Self::EmptyCreatorFileAs => "creator file-as",
            Self::EmptyCreatorRole => "creator role",
            Self::EmptyCreatorAlternateScript => "creator alternate-script",
            Self::EmptyCreatorAlternateScriptLanguage => "creator alternate-script language",
            Self::EmptyDescription => "description",
            Self::EmptyPublisher => "publisher",
            Self::EmptyLanguage => "language",
            Self::EmptyIdentifier => "identifier",
        };

        write!(formatter, "{field} must not be empty")
    }
}

impl Error for MetadataError {}

/// 文字列が空白だけでないことを確認する
fn require_value(value: &str, error: MetadataError) -> Result<(), MetadataError> {
    if value.trim().is_empty() {
        Err(error)
    } else {
        Ok(())
    }
}

/// 任意の文字列が指定されている場合だけ、空白だけでないことを確認する
fn require_optional_value(
    value: &Option<String>,
    error: MetadataError,
) -> Result<(), MetadataError> {
    if let Some(value) = value {
        require_value(value, error)
    } else {
        Ok(())
    }
}

// 利用者入力を表す構造と、入力値の検証を独立して確認する
#[cfg(test)]
mod tests {
    use super::{AlternateScript, CreatorMetadata, MetadataError, PublicationMetadata};

    #[test]
    // 必須のタイトルだけで作ると、既定の日本語メタデータが利用される
    fn creates_metadata_with_japanese_as_the_default_language() {
        let metadata = PublicationMetadata::new("書籍のタイトル".to_owned());

        assert_eq!(metadata.language, "ja");
        assert_eq!(metadata.creator, None);
        assert!(metadata.validate().is_ok());
    }

    #[test]
    // 任意項目も、指定するなら空白だけの値を受け付けない
    fn rejects_an_empty_required_or_specified_value() {
        let cases = [
            (metadata_with_empty_title(), MetadataError::EmptyTitle),
            (
                metadata_with_empty_title_file_as(),
                MetadataError::EmptyTitleFileAs,
            ),
            (metadata_with_empty_language(), MetadataError::EmptyLanguage),
            (
                metadata_with_empty_identifier(),
                MetadataError::EmptyIdentifier,
            ),
            (
                metadata_with_empty_description(),
                MetadataError::EmptyDescription,
            ),
            (
                metadata_with_empty_publisher(),
                MetadataError::EmptyPublisher,
            ),
        ];

        for (metadata, expected_error) in cases {
            assert_eq!(metadata.validate(), Err(expected_error));
        }
    }

    #[test]
    // 著者に属する任意項目も、指定するなら空白だけの値を受け付けない
    fn rejects_empty_values_in_creator_metadata() {
        let cases = [
            (creator_with_empty_name(), MetadataError::EmptyCreatorName),
            (
                creator_with_empty_file_as(),
                MetadataError::EmptyCreatorFileAs,
            ),
            (creator_with_empty_role(), MetadataError::EmptyCreatorRole),
            (
                creator_with_empty_alternate_script(),
                MetadataError::EmptyCreatorAlternateScript,
            ),
            (
                creator_with_empty_alternate_script_language(),
                MetadataError::EmptyCreatorAlternateScriptLanguage,
            ),
        ];

        for (creator, expected_error) in cases {
            let mut metadata = PublicationMetadata::new("書籍のタイトル".to_owned());
            metadata.creator = Some(creator);

            assert_eq!(metadata.validate(), Err(expected_error));
        }
    }

    /// 必須のタイトルが空白だけである書誌情報を作る
    fn metadata_with_empty_title() -> PublicationMetadata {
        PublicationMetadata::new("  ".to_owned())
    }

    /// タイトル読みが空白だけである書誌情報を作る
    fn metadata_with_empty_title_file_as() -> PublicationMetadata {
        let mut metadata = PublicationMetadata::new("書籍のタイトル".to_owned());
        metadata.title_file_as = Some("\n".to_owned());
        metadata
    }

    /// 言語が空である書誌情報を作る
    fn metadata_with_empty_language() -> PublicationMetadata {
        let mut metadata = PublicationMetadata::new("書籍のタイトル".to_owned());
        metadata.language = String::new();
        metadata
    }

    /// identifier が空白だけである書誌情報を作る
    fn metadata_with_empty_identifier() -> PublicationMetadata {
        let mut metadata = PublicationMetadata::new("書籍のタイトル".to_owned());
        metadata.identifier = Some(" ".to_owned());
        metadata
    }

    /// Description が空白だけである書誌情報を作る
    fn metadata_with_empty_description() -> PublicationMetadata {
        let mut metadata = PublicationMetadata::new("書籍のタイトル".to_owned());
        metadata.description = Some("  ".to_owned());
        metadata
    }

    /// Publisher が空白だけである書誌情報を作る
    fn metadata_with_empty_publisher() -> PublicationMetadata {
        let mut metadata = PublicationMetadata::new("書籍のタイトル".to_owned());
        metadata.publisher = Some("  ".to_owned());
        metadata
    }

    /// 著者名が空である著者情報を作る
    fn creator_with_empty_name() -> CreatorMetadata {
        CreatorMetadata {
            name: String::new(),
            file_as: None,
            role: None,
            alternate_script: None,
        }
    }

    /// 著者読みが空白だけである著者情報を作る
    fn creator_with_empty_file_as() -> CreatorMetadata {
        CreatorMetadata {
            name: "著者名".to_owned(),
            file_as: Some(" ".to_owned()),
            role: None,
            alternate_script: None,
        }
    }

    /// 著者の役割が空白だけである著者情報を作る
    fn creator_with_empty_role() -> CreatorMetadata {
        CreatorMetadata {
            name: "著者名".to_owned(),
            file_as: None,
            role: Some("\t".to_owned()),
            alternate_script: None,
        }
    }

    /// 著者の別表記が空である著者情報を作る
    fn creator_with_empty_alternate_script() -> CreatorMetadata {
        CreatorMetadata {
            name: "著者名".to_owned(),
            file_as: None,
            role: None,
            alternate_script: Some(AlternateScript {
                value: String::new(),
                language: "ja-Kana".to_owned(),
            }),
        }
    }

    /// 著者の別表記の言語が空白だけである著者情報を作る
    fn creator_with_empty_alternate_script_language() -> CreatorMetadata {
        CreatorMetadata {
            name: "著者名".to_owned(),
            file_as: None,
            role: None,
            alternate_script: Some(AlternateScript {
                value: "チョシャメイ".to_owned(),
                language: " ".to_owned(),
            }),
        }
    }
}
