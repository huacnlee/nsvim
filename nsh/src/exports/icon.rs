//! `Icon`: a Lucide icon, drawn as a vector in the text colour around it.

use nsh_core::prelude::*;

use crate::ui::icons::{self, Lucide};

/// The icons of the interface. Each is a Lucide icon of the same name.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PropEnum)]
pub enum IconName {
    /// An arrow to the right: go on.
    ArrowRight,
    /// A check mark: an option that is on.
    Check,
    /// A chevron down: an open folder, a menu.
    ChevronDown,
    /// A chevron right: a closed folder, a step on.
    #[default]
    ChevronRight,
    /// An empty circle: a choice not made.
    Circle,
    /// A circle with a dot: the choice made.
    CircleDot,
    /// Two pages with marks: a diff.
    Diff,
    /// A dot: unsaved changes. Draw it with a wide line to fill it.
    Dot,
    /// A git branch.
    GitBranch,
    /// Two arrows between two points: changes to review.
    GitCompareArrows,
    /// A minus: less.
    Minus,
    /// A plus: more.
    Plus,
    /// An X: close.
    X,
}

impl IconName {
    fn icon(self) -> icons::Icon {
        match self {
            IconName::ArrowRight => Lucide::ArrowRight,
            IconName::Check => Lucide::Check,
            IconName::ChevronDown => Lucide::ChevronDown,
            IconName::ChevronRight => Lucide::ChevronRight,
            IconName::Circle => Lucide::Circle,
            IconName::CircleDot => Lucide::CircleDot,
            IconName::Diff => Lucide::Diff,
            IconName::Dot => Lucide::Dot,
            IconName::GitBranch => Lucide::GitBranch,
            IconName::GitCompareArrows => Lucide::GitCompareArrows,
            IconName::Minus => Lucide::Minus,
            IconName::Plus => Lucide::Plus,
            IconName::X => Lucide::X,
        }
    }
}

/// The props of [`Icon`].
#[derive(Debug, Clone, PartialEq, Props)]
pub struct IconProps {
    /// Which icon (required).
    pub name: Prop<IconName>,
    /// Its width and height in points (16 when left out).
    #[prop(default = 16.0)]
    pub size: Prop<f32>,
    /// The width of its lines, of 24 units (2 when left out). A wide line fills a small shape:
    /// `dot` at 9 is a solid dot.
    #[prop(default = 2.0)]
    pub stroke_width: Prop<f32>,
    /// What a screen reader says for it. None: the icon is decoration, and screen readers skip
    /// it, because the text or control beside it says what it means.
    pub label: Prop<Option<SharedString>>,
}

/// A Lucide icon in the text colour around it. Colour it with a text class
/// (`class="text-muted-foreground"`).
#[component]
pub fn Icon(props: IconProps) -> impl IntoElement {
    let (name, width) = (props.name, props.stroke_width);
    let src = move || {
        let icon = name.get().icon();
        let w = width.get();
        if w == 2.0 { icons::src(icon) } else { icons::src_with_stroke_width(icon, w) }
    };
    let size = props.size;
    let label = props.label;
    svg(src)
        .size(move || px(size.get()))
        .flex_shrink_0()
        .accessible(move || label.get().is_some())
        .accessibility_elements_hidden(move || label.get().is_none())
        .role(AccessibilityRole::Image)
        .label(move || label.get().unwrap_or_default())
}
