pub mod rectangles;
pub mod text;
pub mod texture;

pub use rectangles::{RectInstance, RectangleRenderer};
pub use text::TextSystem;
pub use texture::{TextureCache, TextureId, TextureEntry, ImageSource};

use std::sync::atomic::{AtomicU32, Ordering};

/// Measured monospace character width (stored as f32 bits for atomic access).
/// Set once during GPU initialization; read from views for caret positioning.
static MEASURED_CHAR_WIDTH: AtomicU32 = AtomicU32::new(0);

/// Store a measured monospace character width for views to use.
pub fn set_measured_char_width(width: f32) {
    MEASURED_CHAR_WIDTH.store(width.to_bits(), Ordering::Relaxed);
}

/// Read the measured monospace character width.
/// Returns 8.4 as a reasonable default if not yet measured.
pub fn measured_char_width() -> f32 {
    let bits = MEASURED_CHAR_WIDTH.load(Ordering::Relaxed);
    if bits == 0 {
        8.4 // Reasonable fallback before measurement
    } else {
        f32::from_bits(bits)
    }
}
