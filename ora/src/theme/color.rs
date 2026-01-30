use crate::style::Color;

/// Gray scale palette - 11 steps from lightest to darkest
/// Designed for comfortable dark mode with soft neutrals
pub fn gray_50() -> Color {
    Color::rgb(0.98, 0.98, 0.98) // #FAFAFA
}

pub fn gray_100() -> Color {
    Color::rgb(0.96, 0.96, 0.96) // #F5F5F5
}

pub fn gray_200() -> Color {
    Color::rgb(0.93, 0.93, 0.93) // #EEEEEE
}

pub fn gray_300() -> Color {
    Color::rgb(0.88, 0.88, 0.88) // #E0E0E0
}

pub fn gray_400() -> Color {
    Color::rgb(0.74, 0.74, 0.74) // #BDBDBD
}

pub fn gray_500() -> Color {
    Color::rgb(0.62, 0.62, 0.62) // #9E9E9E
}

pub fn gray_600() -> Color {
    Color::rgb(0.46, 0.46, 0.46) // #757575
}

pub fn gray_700() -> Color {
    Color::rgb(0.38, 0.38, 0.38) // #616161
}

pub fn gray_800() -> Color {
    Color::rgb(0.26, 0.26, 0.26) // #424242
}

pub fn gray_900() -> Color {
    Color::rgb(0.13, 0.13, 0.13) // #212121
}

pub fn gray_950() -> Color {
    Color::rgb(0.07, 0.07, 0.07) // #121212
}

/// Accent blue palette for primary actions and highlights
pub fn blue_400() -> Color {
    Color::rgb(0.26, 0.67, 0.96) // #42ABF5 - lighter blue for dark mode hover
}

pub fn blue_500() -> Color {
    Color::rgb(0.13, 0.59, 0.95) // #2196F3 - primary blue
}

pub fn blue_600() -> Color {
    Color::rgb(0.12, 0.53, 0.82) // #1E88E5 - darker blue for active states
}

pub fn blue_700() -> Color {
    Color::rgb(0.10, 0.46, 0.71) // #1976D2 - even darker for light mode active
}

/// Error/destructive red palette
pub fn red_500() -> Color {
    Color::rgb(0.96, 0.26, 0.21) // #F44336
}

pub fn red_600() -> Color {
    Color::rgb(0.90, 0.22, 0.21) // #E53935
}

/// Success green palette
pub fn green_500() -> Color {
    Color::rgb(0.30, 0.69, 0.31) // #4CAF50
}

pub fn green_600() -> Color {
    Color::rgb(0.26, 0.63, 0.28) // #43A047
}

/// Enumeration of all available palette colors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaletteColor {
    Gray50,
    Gray100,
    Gray200,
    Gray300,
    Gray400,
    Gray500,
    Gray600,
    Gray700,
    Gray800,
    Gray900,
    Gray950,
    Blue400,
    Blue500,
    Blue600,
    Blue700,
    Red500,
    Red600,
    Green500,
    Green600,
}

/// Get a color from the palette by enum
pub fn get(palette: PaletteColor) -> Color {
    match palette {
        PaletteColor::Gray50 => gray_50(),
        PaletteColor::Gray100 => gray_100(),
        PaletteColor::Gray200 => gray_200(),
        PaletteColor::Gray300 => gray_300(),
        PaletteColor::Gray400 => gray_400(),
        PaletteColor::Gray500 => gray_500(),
        PaletteColor::Gray600 => gray_600(),
        PaletteColor::Gray700 => gray_700(),
        PaletteColor::Gray800 => gray_800(),
        PaletteColor::Gray900 => gray_900(),
        PaletteColor::Gray950 => gray_950(),
        PaletteColor::Blue400 => blue_400(),
        PaletteColor::Blue500 => blue_500(),
        PaletteColor::Blue600 => blue_600(),
        PaletteColor::Blue700 => blue_700(),
        PaletteColor::Red500 => red_500(),
        PaletteColor::Red600 => red_600(),
        PaletteColor::Green500 => green_500(),
        PaletteColor::Green600 => green_600(),
    }
}
