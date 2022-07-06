pub trait ToAnsiColorSequence {
    fn to_ansi_color_sequence(&self) -> String;
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl ToAnsiColorSequence for Rgb {
    fn to_ansi_color_sequence(&self) -> String {
        let Rgb {r, g, b} = &self;
        format!("\x1b[38;2;{r};{g};{b}m")
    }
}

pub fn ansi_foreground_full_colored(color: &impl ToAnsiColorSequence) -> String {
    color.to_ansi_color_sequence()
}

#[inline]
pub const fn ansi_reset_sequence() -> &'static str {
    "\x1b[0m"
}

#[inline]
pub const fn bar_color() -> impl ToAnsiColorSequence {
    Rgb { r: 160, g: 160, b: 160 }
}
