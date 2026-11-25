//! The Lucide icons of the interface, drawn as vectors in the text colour around them.
//!
//! The icons are the `Lucide` constants of `nsh-icon-lucide`. `nsh-icon` 0.1.0 is built on
//! nsh-core 0.6, so its `icon` and `labeled_icon` make elements that the 0.7 views cannot take.
//! The two functions here do the same thing with nsh-core 0.7's `svg`. Replace them with the
//! crate's functions when `nsh-icon` moves to nsh-core 0.7.

use nsh_core::element::{ImageSrc, Svg};
use nsh_core::prelude::*;
pub use nsh_icon_lucide::{Icon, Lucide};

/// The size of an icon that sets none, in points, as in `nsh-icon`.
pub const DEFAULT_SIZE: f32 = 16.0;

/// The image source of `icon`. It borrows the icon's `data:` URI and does not copy it.
pub fn src(icon: Icon) -> ImageSrc {
    ImageSrc::from(icon.data_uri())
}

/// The image source of `icon` drawn with lines `width` units wide (Lucide's are 2 of 24). A
/// wide line fills a small shape: `Lucide::Dot` at 9 is a solid dot.
pub fn src_with_stroke_width(icon: Icon, width: f32) -> ImageSrc {
    ImageSrc::from(icon.data_uri().replace("stroke-width='2'", &format!("stroke-width='{width}'")))
}

/// A decorative icon: hidden from screen readers, because the text or control beside it says
/// what it means. `src` is an image source, or a closure that gives one.
pub fn icon(src: impl IntoReactive<ImageSrc>) -> Svg {
    svg(src).size(px(DEFAULT_SIZE)).flex_shrink_0().accessibility_elements_hidden(true)
}

/// An icon that stands alone and means something, such as the only content of a button.
/// Screen readers say `label` for it, as an image.
pub fn labeled_icon(src: impl IntoReactive<ImageSrc>, label: impl IntoReactive<SharedString>) -> Svg {
    svg(src).size(px(DEFAULT_SIZE)).flex_shrink_0().accessible(true).role(AccessibilityRole::Image).label(label)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wide_line_replaces_every_stroke_width() {
        let wide = src_with_stroke_width(Lucide::Dot, 9.0);
        assert_eq!(wide, ImageSrc::from(Lucide::Dot.data_uri().replace("stroke-width='2'", "stroke-width='9'")));
        assert_ne!(wide, src(Lucide::Dot));
        assert!(Lucide::Dot.data_uri().contains("stroke-width='2'"), "the dot has the width the function replaces");
    }
}
