#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::too_many_lines,
    clippy::unused_async_trait_impl
)]

use super::{Ratio, Tag};

#[derive(Clone)]
pub(super) struct TaggedRatio {
    ratio: Ratio,
    left_tag: Tag,
    right_tag: Tag,
}

impl TaggedRatio {
    pub(super) fn new(ratio: Ratio, left_tag: Tag, right_tag: Tag) -> Self {
        Self {
            ratio,
            left_tag,
            right_tag,
        }
    }

    pub(super) fn ratio(&self) -> &Ratio {
        &self.ratio
    }

    pub(super) fn left_tag(&self) -> Tag {
        self.left_tag
    }

    pub(super) fn right_tag(&self) -> Tag {
        self.right_tag
    }
}
