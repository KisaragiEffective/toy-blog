use std::fmt::Display;

pub trait ToAnsiForegroundColorSequence {
    fn to_foreground_color_ansi_sequence(&self) -> String;
}

impl <T: ToAnsiForegroundColorSequence> ToAnsiForegroundColorSequence for &T {
    fn to_foreground_color_ansi_sequence(&self) -> String {
        (*self).to_foreground_color_ansi_sequence()
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl ToAnsiForegroundColorSequence for Rgb {
    fn to_foreground_color_ansi_sequence(&self) -> String {
        let Rgb {r, g, b} = &self;
        format!("\x1b[38;2;{r};{g};{b}m")
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[repr(u8)]
#[allow(dead_code)]
pub enum BasicColor {
    Black = 0,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White
}

impl ToAnsiForegroundColorSequence for BasicColor {
    fn to_foreground_color_ansi_sequence(&self) -> String {
        format!("\x1b[3{c}m", c = *self as u8)
    }
}

#[inline]
pub const fn reset_sequence() -> &'static str {
    "\x1b[0m"
}

#[inline]
pub const fn bar_color() -> impl ToAnsiForegroundColorSequence {
    Rgb { r: 160, g: 160, b: 160 }
}

#[inline]
pub fn generate_temporary_foreground_color(x: &impl ToAnsiForegroundColorSequence, s: impl Display) -> String {
    format!(
        "{color}{s}{reset}",
        color = x.to_foreground_color_ansi_sequence(),
        reset = reset_sequence(),
    )
}