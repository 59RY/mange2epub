use std::{error::Error, fmt};

/// EPUB Navigation Document へ出力する 1 件の目次項目
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TocEntry {
    /// ビューアーの目次に表示する文字列
    pub label: String,
    /// 画像を並べた後の 1 始まりのページ番号
    pub page_number: usize,
}

/// 目次項目の検証時に発生しうるエラー
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TocError {
    EmptyLabel,
    PageNumberMustBePositive,
    PageOutOfRange {
        page_number: usize,
        page_count: usize,
    },
}

impl fmt::Display for TocError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyLabel => write!(formatter, "table of contents label must not be empty"),
            Self::PageNumberMustBePositive => {
                write!(
                    formatter,
                    "table of contents page number must be at least 1"
                )
            }
            Self::PageOutOfRange {
                page_number,
                page_count,
            } => write!(
                formatter,
                "table of contents page number {page_number} exceeds page count {page_count}"
            ),
        }
    }
}

impl Error for TocError {}

/// 目次項目が、生成対象のページへ安全にリンクできることを検証する
pub fn validate_toc_entries(page_count: usize, entries: &[TocEntry]) -> Result<(), TocError> {
    for entry in entries {
        if entry.label.trim().is_empty() {
            return Err(TocError::EmptyLabel);
        }
        if entry.page_number == 0 {
            return Err(TocError::PageNumberMustBePositive);
        }
        if entry.page_number > page_count {
            return Err(TocError::PageOutOfRange {
                page_number: entry.page_number,
                page_count,
            });
        }
    }

    Ok(())
}

// 単体テストでは、目次項目の入力境界と許可する指定を確認する
#[cfg(test)]
mod tests {
    use super::{TocEntry, TocError, validate_toc_entries};

    #[test]
    // 空の目次は、書籍タイトルを使う既定項目へ置き換えるため許可する
    fn accepts_an_empty_table_of_contents() {
        assert_eq!(validate_toc_entries(3, &[]), Ok(()));
    }

    #[test]
    // ラベルの重複と同じページへの複数リンクは、有効な目次表現として許可する
    fn accepts_duplicate_labels_and_page_numbers() {
        let entries = vec![
            TocEntry {
                label: "本編".to_owned(),
                page_number: 2,
            },
            TocEntry {
                label: "本編".to_owned(),
                page_number: 2,
            },
        ];

        assert_eq!(validate_toc_entries(3, &entries), Ok(()));
    }

    #[test]
    // ビューアー上で意味のある表示名を持たない目次項目を拒否する
    fn rejects_an_empty_label() {
        let entries = vec![TocEntry {
            label: " \n\t".to_owned(),
            page_number: 1,
        }];

        assert_eq!(validate_toc_entries(1, &entries), Err(TocError::EmptyLabel));
    }

    #[test]
    // ページ番号は利用者向けに 1 始まりとしているため、0 を拒否する
    fn rejects_page_number_zero() {
        let entries = vec![TocEntry {
            label: "表紙".to_owned(),
            page_number: 0,
        }];

        assert_eq!(
            validate_toc_entries(1, &entries),
            Err(TocError::PageNumberMustBePositive)
        );
    }

    #[test]
    // 生成されない XHTML を参照する目次項目を拒否する
    fn rejects_a_page_number_out_of_range() {
        let entries = vec![TocEntry {
            label: "本編".to_owned(),
            page_number: 4,
        }];

        assert_eq!(
            validate_toc_entries(3, &entries),
            Err(TocError::PageOutOfRange {
                page_number: 4,
                page_count: 3,
            })
        );
    }
}
