//! The Lucide icons of the interface, drawn as vectors in the text colour around them.
//!
//! The app carries the few icons it draws as `data:` URIs. They come from
//! [Lucide](https://lucide.dev) 1.43.0, under the ISC License. Copy a new icon from Lucide's
//! SVG, with its `<` and `>` written as `%3C` and `%3E`.

use nsh_core::element::{ImageSrc, Svg};
use nsh_core::prelude::*;

/// The size of an icon that sets none, in points.
pub const DEFAULT_SIZE: f32 = 16.0;

/// One icon: an SVG with lines 2 units wide on a 24-unit grid, in `currentColor`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Icon(&'static str);

impl Icon {
    /// The icon as a `data:image/svg+xml` URI.
    pub const fn data_uri(self) -> &'static str {
        self.0
    }
}

/// The Lucide icons that the app draws. Names are Lucide's names in PascalCase.
pub struct Lucide;

#[allow(non_upper_case_globals)]
impl Lucide {
    pub const ArrowRight: Icon = Icon(
        "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='M5 12h14'/%3E%3Cpath d='m12 5 7 7-7 7'/%3E%3C/svg%3E",
    );
    pub const Check: Icon = Icon(
        "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='M20 6 9 17l-5-5'/%3E%3C/svg%3E",
    );
    pub const ChevronDown: Icon = Icon(
        "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='m6 9 6 6 6-6'/%3E%3C/svg%3E",
    );
    pub const ChevronRight: Icon = Icon(
        "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='m9 18 6-6-6-6'/%3E%3C/svg%3E",
    );
    pub const Circle: Icon = Icon(
        "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Ccircle cx='12' cy='12' r='10'/%3E%3C/svg%3E",
    );
    pub const CircleDot: Icon = Icon(
        "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Ccircle cx='12' cy='12' r='1'/%3E%3Ccircle cx='12' cy='12' r='10'/%3E%3C/svg%3E",
    );
    pub const Diff: Icon = Icon(
        "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='M12 3v14'/%3E%3Cpath d='M5 10h14'/%3E%3Cpath d='M5 21h14'/%3E%3C/svg%3E",
    );
    pub const Dot: Icon = Icon(
        "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Ccircle cx='12' cy='12' r='1'/%3E%3C/svg%3E",
    );
    pub const GitBranch: Icon = Icon(
        "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='M15 6a9 9 0 0 0-9 9V3'/%3E%3Ccircle cx='18' cy='6' r='3'/%3E%3Ccircle cx='6' cy='18' r='3'/%3E%3C/svg%3E",
    );
    pub const GitCompareArrows: Icon = Icon(
        "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Ccircle cx='5' cy='6' r='3'/%3E%3Cpath d='M12 6h5a2 2 0 0 1 2 2v7'/%3E%3Cpath d='m15 9-3-3 3-3'/%3E%3Ccircle cx='19' cy='18' r='3'/%3E%3Cpath d='M12 18H7a2 2 0 0 1-2-2V9'/%3E%3Cpath d='m9 15 3 3-3 3'/%3E%3C/svg%3E",
    );
    pub const Menu: Icon = Icon(
        "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='M4 5h16'/%3E%3Cpath d='M4 12h16'/%3E%3Cpath d='M4 19h16'/%3E%3C/svg%3E",
    );
    pub const Minus: Icon = Icon(
        "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='M5 12h14'/%3E%3C/svg%3E",
    );
    pub const Plus: Icon = Icon(
        "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='M5 12h14'/%3E%3Cpath d='M12 5v14'/%3E%3C/svg%3E",
    );
    pub const X: Icon = Icon(
        "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='M18 6 6 18'/%3E%3Cpath d='m6 6 12 12'/%3E%3C/svg%3E",
    );
}

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
