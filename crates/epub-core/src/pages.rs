/// A page's position when a reading system displays a synthetic spread.
///
/// The values map directly to EPUB's `rendition:page-spread-*` properties.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PagePlacement {
    Left,
    Right,
    Center,
}

/// Returns the default placement for a zero-based page index.
///
/// Page 1 is the cover and occupies the center. From page 2 onward, pages
/// alternate right and left without considering any later override feature.
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
