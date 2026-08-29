use std::{error::Error, fmt};

/// ビューアー が Synthetic Spread で表示するときのページ位置
///
/// この値は EPUB の `rendition:page-spread-*` プロパティへ対応する。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PagePlacement {
    Left,
    Right,
    Center,
}

/// 1 始まりのページ番号に対して指定する配置の上書き
///
/// YAML や将来の GUI は、この利用者向けのページ番号をそのまま渡す。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PageOverride {
    pub page_number: usize,
    pub placement: PagePlacement,
}

/// ページ配置の上書きを解決するときに発生しうるエラー
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PageOverrideError {
    PageNumberMustBePositive,
    PageOutOfRange {
        page_number: usize,
        page_count: usize,
    },
    DuplicatePageNumber {
        page_number: usize,
    },
}

impl fmt::Display for PageOverrideError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PageNumberMustBePositive => {
                write!(formatter, "page override numbers must be at least 1")
            }
            Self::PageOutOfRange {
                page_number,
                page_count,
            } => write!(
                formatter,
                "page override {page_number} is outside the available page range 1..={page_count}"
            ),
            Self::DuplicatePageNumber { page_number } => {
                write!(
                    formatter,
                    "page override {page_number} is specified more than once"
                )
            }
        }
    }
}

impl Error for PageOverrideError {}

/// 0 始まりのページ番号に対応するデフォルト位置を返す
///
/// - 1 ページ目は表紙として中央に配置する
/// - 2 ページ目以降は、将来の上書き機能を考慮せずに右・左を交互に配置する
pub fn default_page_placement(page_index: usize) -> PagePlacement {
    match page_index {
        0 => PagePlacement::Center,
        index if index % 2 == 1 => PagePlacement::Right,
        _ => PagePlacement::Left,
    }
}

/// 既定配置へ利用者指定の上書きを適用し、各ページの最終配置を返す
///
/// center を指定したページの直後は right から再開する。
/// left と right の上書きは、後続ページの既定配置を変えない。
pub fn resolve_page_placements(
    page_count: usize,
    overrides: &[PageOverride],
) -> Result<Vec<PagePlacement>, PageOverrideError> {
    let mut override_placements = vec![None; page_count];

    for page_override in overrides {
        let Some(page_index) = page_override.page_number.checked_sub(1) else {
            return Err(PageOverrideError::PageNumberMustBePositive);
        };
        if page_index >= page_count {
            return Err(PageOverrideError::PageOutOfRange {
                page_number: page_override.page_number,
                page_count,
            });
        }
        if override_placements[page_index].is_some() {
            return Err(PageOverrideError::DuplicatePageNumber {
                page_number: page_override.page_number,
            });
        }

        override_placements[page_index] = Some(page_override.placement);
    }

    let mut placements = Vec::with_capacity(page_count);
    let mut next_automatic_placement = PagePlacement::Right;

    for (page_index, override_placement) in override_placements.into_iter().enumerate() {
        let default_placement = if page_index == 0 {
            PagePlacement::Center
        } else {
            next_automatic_placement
        };
        let placement = override_placement.unwrap_or(default_placement);

        if page_index != 0 {
            next_automatic_placement = if placement == PagePlacement::Center {
                PagePlacement::Right
            } else {
                opposite_page_placement(default_placement)
            };
        }
        placements.push(placement);
    }

    Ok(placements)
}

/// 既定の left と right を交互に切り替える
fn opposite_page_placement(placement: PagePlacement) -> PagePlacement {
    match placement {
        PagePlacement::Left => PagePlacement::Right,
        PagePlacement::Right => PagePlacement::Left,
        PagePlacement::Center => PagePlacement::Right,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PageOverride, PageOverrideError, PagePlacement, default_page_placement,
        resolve_page_placements,
    };

    #[test]
    fn places_the_cover_in_the_center() {
        assert_eq!(default_page_placement(0), PagePlacement::Center);
    }

    #[test]
    fn alternates_right_and_left_after_the_cover() {
        let placements = (0..4).map(default_page_placement).collect::<Vec<_>>();

        assert_eq!(
            placements,
            vec![
                PagePlacement::Center,
                PagePlacement::Right,
                PagePlacement::Left,
                PagePlacement::Right,
            ]
        );
    }

    #[test]
    // center の直後では、未指定ページの自動配置を right から再開する
    fn restarts_automatic_page_placement_after_a_center_override() {
        let placements = resolve_page_placements(
            6,
            &[PageOverride {
                page_number: 4,
                placement: PagePlacement::Center,
            }],
        )
        .unwrap();

        assert_eq!(
            placements,
            vec![
                PagePlacement::Center,
                PagePlacement::Right,
                PagePlacement::Left,
                PagePlacement::Center,
                PagePlacement::Right,
                PagePlacement::Left,
            ]
        );
    }

    #[test]
    // left と right の上書きは、後続ページの自動配置を変えない
    fn does_not_shift_following_page_placements_after_a_left_or_right_override() {
        let placements = resolve_page_placements(
            6,
            &[
                PageOverride {
                    page_number: 2,
                    placement: PagePlacement::Left,
                },
                PageOverride {
                    page_number: 3,
                    placement: PagePlacement::Right,
                },
            ],
        )
        .unwrap();

        assert_eq!(
            placements,
            vec![
                PagePlacement::Center,
                PagePlacement::Left,
                PagePlacement::Right,
                PagePlacement::Right,
                PagePlacement::Left,
                PagePlacement::Right,
            ]
        );
    }

    #[test]
    // すべてのページを指定すると、既定配置を使わずに完全な手動配置を作れる
    fn applies_overrides_for_every_page() {
        let placements = resolve_page_placements(
            3,
            &[
                PageOverride {
                    page_number: 1,
                    placement: PagePlacement::Left,
                },
                PageOverride {
                    page_number: 2,
                    placement: PagePlacement::Center,
                },
                PageOverride {
                    page_number: 3,
                    placement: PagePlacement::Right,
                },
            ],
        )
        .unwrap();

        assert_eq!(
            placements,
            vec![
                PagePlacement::Left,
                PagePlacement::Center,
                PagePlacement::Right,
            ]
        );
    }

    #[test]
    // 1 始まりの利用者向け番号では、0 を有効なページ番号として扱わない
    fn rejects_page_number_zero() {
        let error = resolve_page_placements(
            3,
            &[PageOverride {
                page_number: 0,
                placement: PagePlacement::Center,
            }],
        )
        .unwrap_err();

        assert_eq!(error, PageOverrideError::PageNumberMustBePositive);
    }

    #[test]
    // 実際に読み込んだ画像数を超えるページ番号を拒否する
    fn rejects_a_page_number_outside_the_available_range() {
        let error = resolve_page_placements(
            3,
            &[PageOverride {
                page_number: 4,
                placement: PagePlacement::Center,
            }],
        )
        .unwrap_err();

        assert_eq!(
            error,
            PageOverrideError::PageOutOfRange {
                page_number: 4,
                page_count: 3,
            }
        );
    }

    #[test]
    // 同じページを複数回指定した場合は、後の指定で静かに上書きしない
    fn rejects_duplicate_page_overrides() {
        let error = resolve_page_placements(
            3,
            &[
                PageOverride {
                    page_number: 2,
                    placement: PagePlacement::Left,
                },
                PageOverride {
                    page_number: 2,
                    placement: PagePlacement::Center,
                },
            ],
        )
        .unwrap_err();

        assert_eq!(
            error,
            PageOverrideError::DuplicatePageNumber { page_number: 2 }
        );
    }
}
