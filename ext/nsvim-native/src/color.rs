//! A colour as the themes write it: red, green, blue and alpha from 0 to 1.

/// A colour in sRGB with straight alpha.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const BLACK: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const WHITE: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
    pub const TRANSPARENT: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };

    /// This colour `t` of the way to `other`.
    pub fn mix(self, other: Color, t: f32) -> Color {
        let m = |a: f32, b: f32| a + (b - a) * t;
        Color { r: m(self.r, other.r), g: m(self.g, other.g), b: m(self.b, other.b), a: m(self.a, other.a) }
    }

    /// This colour with alpha `a`.
    pub fn alpha(self, a: f32) -> Color {
        Color { a, ..self }
    }

    /// `#rrggbb`, or `#rrggbbaa` when it is not opaque.
    pub fn to_hex(self) -> String {
        let c = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        if self.a >= 1.0 {
            format!("#{:02x}{:02x}{:02x}", c(self.r), c(self.g), c(self.b))
        } else {
            format!("#{:02x}{:02x}{:02x}{:02x}", c(self.r), c(self.g), c(self.b), c(self.a))
        }
    }
}

/// Reads `#rgb`, `#rrggbb` or `#rrggbbaa`.
pub fn hex(s: &str) -> Option<Color> {
    let h = s.trim().strip_prefix('#')?;
    let digit = |i: usize, n: usize| u8::from_str_radix(h.get(i..i + n)?, 16).ok();
    let (r, g, b, a) = match h.len() {
        3 => (digit(0, 1)? * 17, digit(1, 1)? * 17, digit(2, 1)? * 17, 255),
        6 => (digit(0, 2)?, digit(2, 2)?, digit(4, 2)?, 255),
        8 => (digit(0, 2)?, digit(2, 2)?, digit(4, 2)?, digit(6, 2)?),
        _ => return None,
    };
    let f = |v: u8| v as f32 / 255.0;
    Some(Color { r: f(r), g: f(g), b: f(b), a: f(a) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_reads_and_writes_back() {
        assert_eq!(hex("#6C9EFF").unwrap().to_hex(), "#6c9eff");
        assert_eq!(hex("#fff"), Some(Color::WHITE));
        assert_eq!(hex("#00000080").unwrap().to_hex(), "#00000080");
        assert_eq!(hex("6C9EFF"), None);
    }
}
