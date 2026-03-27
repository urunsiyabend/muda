use crate::style::Color;

/// IDE-oriented slate palette tuned for dark chrome and bright editor text.
pub fn gray_50() -> Color {
    Color::rgb(0.97, 0.98, 0.99) // #F8FAFC
}

pub fn gray_100() -> Color {
    Color::rgb(0.93, 0.95, 0.98) // #EDF2F9
}

pub fn gray_200() -> Color {
    Color::rgb(0.86, 0.89, 0.94) // #DBE3EF
}

pub fn gray_300() -> Color {
    Color::rgb(0.74, 0.78, 0.86) // #BCC7DB
}

pub fn gray_400() -> Color {
    Color::rgb(0.58, 0.63, 0.74) // #94A1BC
}

pub fn gray_500() -> Color {
    Color::rgb(0.43, 0.48, 0.58) // #6E7A94
}

pub fn gray_600() -> Color {
    Color::rgb(0.31, 0.35, 0.44) // #50596F
}

pub fn gray_700() -> Color {
    Color::rgb(0.18, 0.21, 0.28) // #2E3548
}

pub fn gray_800() -> Color {
    Color::rgb(0.13, 0.15, 0.21) // #202634
}

pub fn gray_900() -> Color {
    Color::rgb(0.09, 0.10, 0.15) // #171A26
}

pub fn gray_950() -> Color {
    Color::rgb(0.06, 0.07, 0.10) // #10121A
}

/// Accent blues inspired by modern editor chrome and focus states.
pub fn blue_400() -> Color {
    Color::rgb(0.47, 0.69, 1.0) // #78B0FF
}

pub fn blue_500() -> Color {
    Color::rgb(0.32, 0.56, 1.0) // #528FFF
}

pub fn blue_600() -> Color {
    Color::rgb(0.23, 0.46, 0.91) // #3A75E8
}

pub fn blue_700() -> Color {
    Color::rgb(0.17, 0.37, 0.77) // #2C5FC4
}

pub fn red_500() -> Color {
    Color::rgb(0.97, 0.42, 0.42) // #F76B6B
}

pub fn red_600() -> Color {
    Color::rgb(0.85, 0.28, 0.32) // #D94951
}

pub fn green_500() -> Color {
    Color::rgb(0.38, 0.80, 0.56) // #61CC8F
}

pub fn green_600() -> Color {
    Color::rgb(0.24, 0.67, 0.45) // #3DAB73
}

pub fn amber_500() -> Color {
    Color::rgb(0.97, 0.75, 0.34) // #F7BF57
}

pub fn amber_600() -> Color {
    Color::rgb(0.91, 0.63, 0.19) // #E8A132
}

pub fn purple_300() -> Color {
    Color::rgb(0.86, 0.73, 0.98) // #DBBAFA
}

pub fn purple_400() -> Color {
    Color::rgb(0.78, 0.60, 0.95) // #C799F2
}

pub fn purple_500() -> Color {
    Color::rgb(0.63, 0.44, 0.85) // #A171D8
}

pub fn cyan_400() -> Color {
    Color::rgb(0.49, 0.83, 0.93) // #7DD4ED
}

pub fn cyan_500() -> Color {
    Color::rgb(0.24, 0.66, 0.80) // #3EA8CC
}

pub fn orange_400() -> Color {
    Color::rgb(0.97, 0.72, 0.43) // #F8B86D
}

pub fn orange_500() -> Color {
    Color::rgb(0.90, 0.54, 0.24) // #E68A3D
}

pub fn yellow_300() -> Color {
    Color::rgb(1.0, 0.90, 0.57) // #FFE591
}

pub fn yellow_400() -> Color {
    Color::rgb(0.99, 0.82, 0.43) // #FDD16D
}

pub fn yellow_500() -> Color {
    Color::rgb(0.90, 0.69, 0.26) // #E5B043
}

pub fn blue_200() -> Color {
    Color::rgb(0.71, 0.84, 1.0) // #B5D6FF
}

pub fn blue_900() -> Color {
    Color::rgb(0.13, 0.22, 0.39) // #213864
}

pub fn green_400() -> Color {
    Color::rgb(0.55, 0.87, 0.51) // #8CDD82
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
    Blue200,
    Blue400,
    Blue500,
    Blue600,
    Blue700,
    Blue900,
    Red500,
    Red600,
    Green400,
    Green500,
    Green600,
    Amber500,
    Amber600,
    Purple300,
    Purple400,
    Purple500,
    Cyan400,
    Cyan500,
    Orange400,
    Orange500,
    Yellow300,
    Yellow400,
    Yellow500,
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
        PaletteColor::Blue200 => blue_200(),
        PaletteColor::Blue400 => blue_400(),
        PaletteColor::Blue500 => blue_500(),
        PaletteColor::Blue600 => blue_600(),
        PaletteColor::Blue700 => blue_700(),
        PaletteColor::Blue900 => blue_900(),
        PaletteColor::Red500 => red_500(),
        PaletteColor::Red600 => red_600(),
        PaletteColor::Green400 => green_400(),
        PaletteColor::Green500 => green_500(),
        PaletteColor::Green600 => green_600(),
        PaletteColor::Amber500 => amber_500(),
        PaletteColor::Amber600 => amber_600(),
        PaletteColor::Purple300 => purple_300(),
        PaletteColor::Purple400 => purple_400(),
        PaletteColor::Purple500 => purple_500(),
        PaletteColor::Cyan400 => cyan_400(),
        PaletteColor::Cyan500 => cyan_500(),
        PaletteColor::Orange400 => orange_400(),
        PaletteColor::Orange500 => orange_500(),
        PaletteColor::Yellow300 => yellow_300(),
        PaletteColor::Yellow400 => yellow_400(),
        PaletteColor::Yellow500 => yellow_500(),
    }
}
