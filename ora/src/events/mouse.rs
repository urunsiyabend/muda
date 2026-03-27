use super::types::{Modifiers, MouseButton, Point};
use crate::style::units::Rect;

/// Mouse move event
#[derive(Debug, Clone, Copy)]
pub struct MouseMoveEvent {
    pub position: Point,
    pub modifiers: Modifiers,
}

/// Mouse button down event
#[derive(Debug, Clone, Copy)]
pub struct MouseDownEvent {
    pub position: Point,
    pub button: MouseButton,
    pub modifiers: Modifiers,
    /// Number of rapid consecutive clicks (1 = single, 2 = double, etc.)
    pub click_count: u32,
}

/// Mouse button up event
#[derive(Debug, Clone, Copy)]
pub struct MouseUpEvent {
    pub position: Point,
    pub button: MouseButton,
    pub modifiers: Modifiers,
}

/// Mouse scroll/wheel event
#[derive(Debug, Clone, Copy)]
pub struct MouseScrollEvent {
    pub delta: Point,
    pub modifiers: Modifiers,
}

/// Opaque identifier for hitboxes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HitboxId(pub u64);

/// Hitbox for hit testing
#[derive(Debug, Clone, Copy)]
pub struct Hitbox {
    pub id: HitboxId,
    pub bounds: Rect,
    pub opaque: bool,
}

/// Test hitboxes in reverse order (last painted = topmost).
/// Returns the HitboxId of the topmost opaque hitbox containing the point.
pub fn hit_test(hitboxes: &[Hitbox], point: Point) -> Option<HitboxId> {
    for hitbox in hitboxes.iter().rev() {
        if !hitbox.opaque {
            continue;
        }
        if hitbox.bounds.contains_point(point) {
            return Some(hitbox.id);
        }
    }
    None
}
