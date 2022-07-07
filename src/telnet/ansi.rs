use std::fmt::Display;

pub trait ToAnsiForegroundColorSequence {
    fn to_foreground_color_ansi_sequence(&self) -> String;
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