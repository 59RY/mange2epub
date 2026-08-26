/// Reading SystemがSynthetic Spreadで表示するときのページ位置。
///
/// この値はEPUBの`rendition:page-spread-*`プロパティへ対応する。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PagePlacement {
    Left,
    Right,
    Center,
}

/// 0始まりのページ番号に対応するデフォルト位置を返す。
///
/// 1ページ目は表紙として中央に配置する。
/// 2ページ目以降は、将来の上書き機能を考慮せずに右・左を交互に配置する。
pub fn default_page_placement(page_index: usize) -> PagePlacement {
    match page_index {
        0 => PagePlacement::Center,
        index if index % 2 == 1 => PagePlacement::Right,
        _ => PagePlacement::Left,
    }
}

#[cfg(test)]
mod tests {
    use super::{PagePlacement, default_page_placement};

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
}
