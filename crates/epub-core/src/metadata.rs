use std::{error::Error, fmt};

use time::{Date, Month, OffsetDateTime, format_description::well_known::Rfc3339};

/// 利用者が指定する書誌情報
///
/// CLI や将来の設定ファイルはこの構造を作る。
/// UUID や更新日時など、EPUB 生成時に決まる値はここに含めない。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicationMetadata {
    pub title: String,
    pub title_file_as: Option<String>,
    pub creators: Vec<CreatorMetadata>,
    pub description: Option<String>,
    pub publisher: Option<String>,
    pub date: Option<String>,
    pub types: Vec<String>,
    pub subjects: Vec<String>,
    pub language: String,
    pub identifier: Option<String>,
}

/// 1名の著者に関する書誌情報
///
/// 複数の役割と別表記は、それぞれ指定した順序で EPUB へ出力する。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreatorMetadata {
    pub name: String,
    pub file_as: Option<String>,
    pub roles: Vec<String>,
    pub alternate_scripts: Vec<AlternateScript>,
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
    EmptyDate,
    InvalidDate,
    EmptyType,
    EmptySubject,
    EmptyLanguage,
    EmptyIdentifier,
}

impl PublicationMetadata {
    /// 必須のタイトルを受け取り、既定の言語を設定して書誌情報を作る
    pub fn new(title: String) -> Self {
        Self {
            title,
            title_file_as: None,
            creators: Vec::new(),
            description: None,
            publisher: None,
            date: None,
            types: Vec::new(),
            subjects: Vec::new(),
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
        require_optional_value(&self.date, MetadataError::EmptyDate)?;
        if self
            .date
            .as_deref()
            .is_some_and(|date| !is_valid_date(date))
        {
            return Err(MetadataError::InvalidDate);
        }
        for value in &self.types {
            require_value(value, MetadataError::EmptyType)?;
        }
        for subject in &self.subjects {
            require_value(subject, MetadataError::EmptySubject)?;
        }
        require_value(&self.language, MetadataError::EmptyLanguage)?;
        require_optional_value(&self.identifier, MetadataError::EmptyIdentifier)?;

        for creator in &self.creators {
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
        for role in &self.roles {
            require_value(role, MetadataError::EmptyCreatorRole)?;
        }

        for alternate_script in &self.alternate_scripts {
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
        if matches!(self, Self::InvalidDate) {
            return write!(
                formatter,
                "date must use YYYY-MM-DD or an RFC 3339 date-time"
            );
        }

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
            Self::EmptyDate => "date",
            Self::EmptyType => "type",
            Self::EmptySubject => "subject",
            Self::EmptyLanguage => "language",
            Self::EmptyIdentifier => "identifier",
            Self::InvalidDate => unreachable!("invalid dates are handled before field lookup"),
        };

        write!(formatter, "{field} must not be empty")
    }
}

/// 日付が `YYYY-MM-DD` または RFC 3339 の日時であることを確認する
fn is_valid_date(value: &str) -> bool {
    if OffsetDateTime::parse(value, &Rfc3339).is_ok() {
        return true;
    }

    if value.len() != 10 || !value.is_ascii() || &value[4..5] != "-" || &value[7..8] != "-" {
        return false;
    }

    let Ok(year) = value[0..4].parse::<i32>() else {
        return false;
    };
    let Ok(month_number) = value[5..7].parse::<u8>() else {
        return false;
    };
    let Ok(month) = Month::try_from(month_number) else {
        return false;
    };
    let Ok(day) = value[8..10].parse::<u8>() else {
        return false;
    };

    Date::from_calendar_date(year, month, day).is_ok()
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
        assert!(metadata.creators.is_empty());
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
            (metadata_with_empty_date(), MetadataError::EmptyDate),
            (metadata_with_empty_type(), MetadataError::EmptyType),
            (metadata_with_empty_subject(), MetadataError::EmptySubject),
        ];

        for (metadata, expected_error) in cases {
            assert_eq!(metadata.validate(), Err(expected_error));
        }
    }

    #[test]
    // 日付のみ、UTC、UTC オフセット付きの日時を受け付ける
    fn accepts_supported_publication_date_formats() {
        for date in [
            "2026-08-31",
            "2026-08-31T15:00:00Z",
            "2026-09-01T00:00:00+09:00",
            "2026-08-31T08:00:00-07:00",
        ] {
            let mut metadata = PublicationMetadata::new("書籍のタイトル".to_owned());
            metadata.date = Some(date.to_owned());

            assert!(metadata.validate().is_ok());
        }
    }

    #[test]
    // 存在しない日付時刻、形式外の値、タイムゾーンのない日時を受け付けない
    fn rejects_an_invalid_publication_date() {
        for date in [
            "2026-02-30",                // 存在しない日付
            "2026-13-32",                // 存在しない日付
            "0000-00-00",                // 存在しない日付
            "2026-3-1",                  // 形式外(ゼロ埋めされていない)
            "２０２６－０４－０２",      // 形式外(全角)
            "2025/11/02",                // 形式外(スラッシュ区切り)
            "2026-08-30T15:00:00",       // タイムゾーンがない
            "2026-08-30T78:00:00Z",      // 存在しない時刻
            "2026-08-30T22:75:90Z",      // 存在しない時刻
            "2026-09-02T02:00:00+50:00", // RFC 3339 のオフセット上限を超過
        ] {
            let mut metadata = PublicationMetadata::new("書籍のタイトル".to_owned());
            metadata.date = Some(date.to_owned());

            assert_eq!(metadata.validate(), Err(MetadataError::InvalidDate));
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
            metadata.creators = vec![creator];

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

    /// Date が空白だけである書誌情報を作る
    fn metadata_with_empty_date() -> PublicationMetadata {
        let mut metadata = PublicationMetadata::new("書籍のタイトル".to_owned());
        metadata.date = Some("  ".to_owned());
        metadata
    }

    /// Type が空白だけである書誌情報を作る
    fn metadata_with_empty_type() -> PublicationMetadata {
        let mut metadata = PublicationMetadata::new("書籍のタイトル".to_owned());
        metadata.types = vec!["\t".to_owned()];
        metadata
    }

    /// Subject が空白だけである書誌情報を作る
    fn metadata_with_empty_subject() -> PublicationMetadata {
        let mut metadata = PublicationMetadata::new("書籍のタイトル".to_owned());
        metadata.subjects = vec!["\n".to_owned()];
        metadata
    }

    /// 著者名が空である著者情報を作る
    fn creator_with_empty_name() -> CreatorMetadata {
        CreatorMetadata {
            name: String::new(),
            file_as: None,
            roles: Vec::new(),
            alternate_scripts: Vec::new(),
        }
    }

    /// 著者読みが空白だけである著者情報を作る
    fn creator_with_empty_file_as() -> CreatorMetadata {
        CreatorMetadata {
            name: "著者名".to_owned(),
            file_as: Some(" ".to_owned()),
            roles: Vec::new(),
            alternate_scripts: Vec::new(),
        }
    }

    /// 著者の役割が空白だけである著者情報を作る
    fn creator_with_empty_role() -> CreatorMetadata {
        CreatorMetadata {
            name: "著者名".to_owned(),
            file_as: None,
            roles: vec!["\t".to_owned()],
            alternate_scripts: Vec::new(),
        }
    }

    /// 著者の別表記が空である著者情報を作る
    fn creator_with_empty_alternate_script() -> CreatorMetadata {
        CreatorMetadata {
            name: "著者名".to_owned(),
            file_as: None,
            roles: Vec::new(),
            alternate_scripts: vec![AlternateScript {
                value: String::new(),
                language: "ja-Kana".to_owned(),
            }],
        }
    }

    /// 著者の別表記の言語が空白だけである著者情報を作る
    fn creator_with_empty_alternate_script_language() -> CreatorMetadata {
        CreatorMetadata {
            name: "著者名".to_owned(),
            file_as: None,
            roles: Vec::new(),
            alternate_scripts: vec![AlternateScript {
                value: "チョシャメイ".to_owned(),
                language: " ".to_owned(),
            }],
        }
    }
}
