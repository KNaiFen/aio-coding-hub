//! Shared semantic palette with deterministic terminal capability fallbacks.

use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorCapability {
    None,
    Ansi16,
    Ansi256,
    TrueColor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Default,
    Muted,
    Accent,
    Info,
    Provider,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    capability: ColorCapability,
}

impl Palette {
    pub fn detected(enabled: bool) -> Self {
        #[cfg(test)]
        let capability = if enabled {
            ColorCapability::Ansi16
        } else {
            ColorCapability::None
        };

        #[cfg(not(test))]
        let capability = {
            let no_color = std::env::var_os("NO_COLOR").is_some();
            let color_term = std::env::var("COLORTERM").ok();
            let term = std::env::var("TERM").ok();
            capability_from_values(enabled, no_color, color_term.as_deref(), term.as_deref())
        };

        Self::new(capability)
    }

    pub const fn new(capability: ColorCapability) -> Self {
        Self { capability }
    }

    #[cfg(test)]
    pub const fn capability(self) -> ColorCapability {
        self.capability
    }

    pub fn style(self, tone: Tone) -> Style {
        let mut style = match self.color(tone) {
            Some(color) => Style::default().fg(color),
            None => Style::default(),
        };
        if tone == Tone::Muted {
            style = style.add_modifier(Modifier::DIM);
        }
        style
    }

    pub fn selected(self, style: Style) -> Style {
        match self.selection_background() {
            Some(color) => style.bg(color),
            None => style.add_modifier(Modifier::REVERSED),
        }
    }

    fn color(self, tone: Tone) -> Option<Color> {
        if tone == Tone::Default || self.capability == ColorCapability::None {
            return None;
        }
        Some(match (self.capability, tone) {
            (ColorCapability::TrueColor, Tone::Muted) => Color::Rgb(139, 148, 158),
            (ColorCapability::TrueColor, Tone::Accent) => Color::Rgb(105, 170, 179),
            (ColorCapability::TrueColor, Tone::Info) => Color::Rgb(125, 160, 196),
            (ColorCapability::TrueColor, Tone::Provider) => Color::Rgb(182, 137, 190),
            (ColorCapability::TrueColor, Tone::Success) => Color::Rgb(113, 174, 126),
            (ColorCapability::TrueColor, Tone::Warning) => Color::Rgb(205, 166, 91),
            (ColorCapability::TrueColor, Tone::Error) => Color::Rgb(207, 105, 111),
            (ColorCapability::Ansi256, Tone::Muted) => Color::Indexed(245),
            (ColorCapability::Ansi256, Tone::Accent) => Color::Indexed(109),
            (ColorCapability::Ansi256, Tone::Info) => Color::Indexed(110),
            (ColorCapability::Ansi256, Tone::Provider) => Color::Indexed(182),
            (ColorCapability::Ansi256, Tone::Success) => Color::Indexed(108),
            (ColorCapability::Ansi256, Tone::Warning) => Color::Indexed(179),
            (ColorCapability::Ansi256, Tone::Error) => Color::Indexed(174),
            (ColorCapability::Ansi16, Tone::Muted) => Color::DarkGray,
            (ColorCapability::Ansi16, Tone::Accent) => Color::Cyan,
            (ColorCapability::Ansi16, Tone::Info) => Color::Blue,
            (ColorCapability::Ansi16, Tone::Provider) => Color::Magenta,
            (ColorCapability::Ansi16, Tone::Success) => Color::Green,
            (ColorCapability::Ansi16, Tone::Warning) => Color::Yellow,
            (ColorCapability::Ansi16, Tone::Error) => Color::Red,
            (ColorCapability::None, _) | (_, Tone::Default) => return None,
        })
    }

    fn selection_background(self) -> Option<Color> {
        match self.capability {
            ColorCapability::None => None,
            ColorCapability::Ansi16 => Some(Color::DarkGray),
            ColorCapability::Ansi256 => Some(Color::Indexed(237)),
            ColorCapability::TrueColor => Some(Color::Rgb(53, 58, 65)),
        }
    }
}

fn capability_from_values(
    enabled: bool,
    no_color: bool,
    color_term: Option<&str>,
    term: Option<&str>,
) -> ColorCapability {
    if !enabled || no_color || term.is_some_and(|value| value.eq_ignore_ascii_case("dumb")) {
        return ColorCapability::None;
    }
    if color_term.is_some_and(|value| {
        let value = value.to_ascii_lowercase();
        value.contains("truecolor") || value.contains("24bit")
    }) {
        return ColorCapability::TrueColor;
    }
    if term.is_some_and(|value| value.to_ascii_lowercase().contains("256color")) {
        return ColorCapability::Ansi256;
    }
    ColorCapability::Ansi16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_detection_has_deterministic_fallbacks() {
        assert_eq!(
            capability_from_values(true, false, Some("truecolor"), Some("xterm-256color")),
            ColorCapability::TrueColor
        );
        assert_eq!(
            capability_from_values(true, false, None, Some("screen-256color")),
            ColorCapability::Ansi256
        );
        assert_eq!(
            capability_from_values(true, false, None, Some("xterm")),
            ColorCapability::Ansi16
        );
        assert_eq!(
            capability_from_values(true, false, None, None),
            ColorCapability::Ansi16
        );
        assert_eq!(
            capability_from_values(true, true, Some("truecolor"), Some("xterm")),
            ColorCapability::None
        );
        assert_eq!(
            capability_from_values(false, false, Some("truecolor"), Some("xterm")),
            ColorCapability::None
        );
    }

    #[test]
    fn semantic_colors_change_with_terminal_capability() {
        assert_eq!(
            Palette::new(ColorCapability::TrueColor)
                .style(Tone::Success)
                .fg,
            Some(Color::Rgb(113, 174, 126))
        );
        assert_eq!(
            Palette::new(ColorCapability::Ansi256)
                .style(Tone::Success)
                .fg,
            Some(Color::Indexed(108))
        );
        assert_eq!(
            Palette::new(ColorCapability::Ansi16)
                .style(Tone::Success)
                .fg,
            Some(Color::Green)
        );
        assert_eq!(
            Palette::new(ColorCapability::None).style(Tone::Success).fg,
            None
        );
        assert_eq!(
            Palette::new(ColorCapability::Ansi256).capability(),
            ColorCapability::Ansi256
        );
        assert!(Palette::new(ColorCapability::None)
            .selected(Style::default())
            .add_modifier
            .contains(Modifier::REVERSED));
    }
}
