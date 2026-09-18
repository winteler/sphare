use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumIter};

#[repr(i16)]
#[derive(Clone, Copy, Debug, Default, Display, EnumIter, Eq, Hash, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::Type))]
pub enum Color {
    #[default]
    None = -1,
    Blue = 0,
    Purple = 1,
    Pink = 2,
    Bordeaux = 3,
    Red = 4,
    Orange = 5,
    Yellow = 6,
    Green = 7,
    Cyan = 8,
    LightGray = 9,
    DarkGray = 10,
    Brown = 11,
    Black = 12,
    White = 13,
}

impl From<i16> for Color {
    fn from(category_color_val: i16) -> Self {
        match category_color_val {
            x if x == Color::Blue as i16 => Color::Blue,
            x if x == Color::Purple as i16 => Color::Purple,
            x if x == Color::Pink as i16 => Color::Pink,
            x if x == Color::Bordeaux as i16 => Color::Bordeaux,
            x if x == Color::Red as i16 => Color::Red,
            x if x == Color::Orange as i16 => Color::Orange,
            x if x == Color::Yellow as i16 => Color::Yellow,
            x if x == Color::Green as i16 => Color::Green,
            x if x == Color::Cyan as i16 => Color::Cyan,
            x if x == Color::LightGray as i16 => Color::LightGray,
            x if x == Color::DarkGray as i16 => Color::DarkGray,
            x if x == Color::Brown as i16 => Color::Brown,
            x if x == Color::Black as i16 => Color::Black,
            x if x == Color::White as i16 => Color::White,
            _ => Color::None,
        }
    }
}

impl Color {
    pub fn to_bg_class(&self) -> &'static str {
        match self {
            Color::None => "border border-base-content/20 font-semibold",
            Color::Blue => "bg-blue-600 text-white font-semibold",
            Color::Purple => "bg-purple-600 text-white font-semibold",
            Color::Pink => "bg-pink-400 text-black font-semibold",
            Color::Bordeaux => "bg-red-800 text-white font-semibold",
            Color::Red => "bg-red-600 text-black font-semibold",
            Color::Orange => "bg-orange-600 text-black font-semibold",
            Color::Yellow => "bg-yellow-600 text-black font-semibold",
            Color::Green => "bg-green-600 text-black font-semibold",
            Color::Cyan => "bg-cyan-600 text-white font-semibold",
            Color::LightGray => "bg-gray-400 text-black font-semibold",
            Color::DarkGray => "bg-gray-600 text-white font-semibold",
            Color::Brown => "bg-amber-800 text-white font-semibold",
            Color::Black => "bg-black text-white font-semibold",
            Color::White => "bg-white text-black font-semibold",
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::colors::Color;
    use strum::IntoEnumIterator;

    #[test]
    fn test_color_from_i16() {
        for color in Color::iter() {
            assert_eq!(Color::from(color as i16), color);
        }
        assert_eq!(Color::from(-2), Color::None);
        assert_eq!(Color::from(100), Color::None);
    }
}