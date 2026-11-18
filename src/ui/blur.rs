//! The privacy blur, as WA Web Plus has it: contact photos, names and
//! numbers, the chat list's previews and the open chat's messages, each
//! drawn out of focus until the pointer rests on it, so a glance at the
//! screen gives nothing away.
//!
//! egui has no filters, so a blur is drawn the way a lens smears: every shape
//! painted for the hidden content is repainted as twenty faint copies
//! spread evenly over a small disc. Text keeps its galley and only its opacity
//! changes, so colour emoji (painted over text by fastframe-emoji at its
//! opacity) blur with it.

use std::cell::Cell;

use egui::epaint::{ClippedShape, Shape};
use egui::layers::ShapeIdx;
use egui::{Color32, Rect, Vec2};

use crate::settings::PrivacyBlur;

thread_local! {
    /// What the window being drawn hides: set once per frame from Settings.
    static CHOICE: Cell<PrivacyBlur> = const { Cell::new(PrivacyBlur::NONE) };
}

/// Takes the Settings choice for this frame.
pub fn set(choice: PrivacyBlur) {
    CHOICE.with(|current| current.set(choice));
}

/// Whether contact and group photos are blurred.
pub fn photos() -> bool {
    CHOICE.with(Cell::get).photos
}

/// Whether names and phone numbers are blurred.
pub fn names() -> bool {
    CHOICE.with(Cell::get).names
}

/// Whether the latest message under each chat in the list is blurred.
pub fn previews() -> bool {
    CHOICE.with(Cell::get).previews
}

/// Whether message text and media in the open chat are blurred.
pub fn messages() -> bool {
    CHOICE.with(Cell::get).messages
}

/// How far the copies spread, in points: enough that no letter survives at
/// the sizes the interface uses.
pub const RADIUS: f32 = 6.0;
/// A photo's copies spread wider: faces stay recognisable longer than text.
pub const PHOTO_RADIUS: f32 = 8.0;

/// Where the next shape painted on `ui`'s layer will go, to pass to
/// [`since`] once the content is drawn.
pub fn mark(ui: &egui::Ui) -> usize {
    let layer = ui.layer_id();
    ui.ctx()
        .graphics_mut(|graphics| graphics.entry(layer).next_idx().0)
}

/// Blurs everything painted on `ui`'s layer since `start`, unless the
/// pointer rests on `reveal`, which shows it as it is.
pub fn since(ui: &egui::Ui, start: usize, reveal: Rect, radius: f32) {
    if ui.rect_contains_pointer(reveal) {
        return;
    }
    let layer = ui.layer_id();
    ui.ctx().graphics_mut(|graphics| {
        let list = graphics.entry(layer);
        let end = list.next_idx().0;
        for index in start..end {
            list.mutate_shape(ShapeIdx(index), |clipped: &mut ClippedShape| {
                let shape = std::mem::replace(&mut clipped.shape, Shape::Noop);
                clipped.shape = blurred(shape, radius);
            });
        }
    });
}

/// Paints with `paint` and blurs what it drew when `on`, unless the pointer
/// rests on `reveal`.
pub fn hiding<R>(
    ui: &mut egui::Ui,
    on: bool,
    reveal: Rect,
    radius: f32,
    paint: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let start = mark(ui);
    let result = paint(ui);
    if on {
        since(ui, start, reveal, radius);
    }
    result
}

/// How many copies make a blur.
const COPIES: usize = 20;

/// Where each copy goes, as fractions of the radius: a sunflower spiral,
/// which spreads them evenly over the whole disc. Copies on a ring alone
/// would leave the letters' outlines readable.
fn offsets() -> impl Iterator<Item = Vec2> {
    let golden = std::f32::consts::PI * (3.0 - 5.0_f32.sqrt());
    (0..COPIES).map(move |index| {
        let distance = ((index as f32 + 0.5) / COPIES as f32).sqrt();
        Vec2::angled(index as f32 * golden) * distance
    })
}

/// How opaque each copy is: together they cover a point about as fully as
/// the original did (1 - (1 - a)^20 ≈ 0.85).
const COPY_OPACITY: f32 = 0.09;

/// `shape` as faint copies spread over a disc of `radius` points.
pub fn blurred(shape: Shape, radius: f32) -> Shape {
    if matches!(shape, Shape::Noop | Shape::Callback(_)) {
        return shape;
    }
    Shape::Vec(
        offsets()
            .map(|offset| {
                let mut copy = shape.clone();
                fade(&mut copy, COPY_OPACITY);
                copy.translate(offset * radius);
                copy
            })
            .collect(),
    )
}

/// Multiplies a shape's opacity. Text keeps its galley, which the emoji
/// plugin knows it by, and changes only its opacity factor.
fn fade(shape: &mut Shape, opacity: f32) {
    match shape {
        Shape::Text(text) => text.opacity_factor *= opacity,
        Shape::Vec(shapes) => {
            for shape in shapes {
                fade(shape, opacity);
            }
        }
        other => egui::epaint::shape_transform::adjust_colors(other, move |color: &mut Color32| {
            *color = color.gamma_multiply(opacity);
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_blurred_shape_is_faint_copies_around_it() {
        let rect = Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(20.0, 20.0));
        let shape = Shape::rect_filled(rect, 0.0, Color32::WHITE);
        let Shape::Vec(copies) = blurred(shape, 4.0) else {
            panic!("copies");
        };
        assert_eq!(copies.len(), COPIES);
        let mut far = 0.0_f32;
        for copy in &copies {
            let Shape::Rect(copy) = copy else {
                panic!("a rectangle");
            };
            assert!(copy.fill.a() < 64, "{:?}", copy.fill);
            far = far.max((copy.rect.center() - rect.center()).length());
        }
        assert!(far > 3.8 && far <= 4.0, "{far}");
        // Together they cover about as much as the shape did.
        let covered = 1.0 - (1.0 - COPY_OPACITY).powi(COPIES as i32);
        assert!((covered - 0.85).abs() < 0.01, "{covered}");
    }

    #[test]
    fn content_painted_since_the_mark_is_blurred_until_hovered() {
        let ctx = egui::Context::default();
        for hovered in [false, true] {
            let pointer = if hovered {
                egui::pos2(15.0, 15.0)
            } else {
                egui::pos2(500.0, 500.0)
            };
            let mut shapes = Vec::new();
            for _ in 0..2 {
                let output = ctx.run_ui(
                    egui::RawInput {
                        events: vec![egui::Event::PointerMoved(pointer)],
                        ..Default::default()
                    },
                    |ui| {
                        let before = ui.painter().add(Shape::Noop);
                        let start = mark(ui);
                        let rect =
                            Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(20.0, 20.0));
                        ui.painter().rect_filled(rect, 0.0, Color32::RED);
                        since(ui, start, rect, RADIUS);
                        ui.painter()
                            .set(before, Shape::rect_filled(rect, 0.0, Color32::BLUE));
                    },
                );
                let mut output = output;
                output.textures_delta.clear();
                shapes = output.shapes;
            }
            let red: Vec<_> = shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    Shape::Vec(copies) => Some(copies.len()),
                    Shape::Rect(rect) if rect.fill == Color32::RED => Some(1),
                    _ => None,
                })
                .collect();
            assert_eq!(red, [if hovered { 1 } else { COPIES }], "hovered {hovered}");
            // What was painted before the mark is left alone.
            assert!(shapes.iter().any(
                |clipped| matches!(&clipped.shape, Shape::Rect(rect) if rect.fill == Color32::BLUE)
            ));
        }
    }

    #[test]
    fn the_choice_is_read_back() {
        set(PrivacyBlur {
            photos: true,
            names: false,
            previews: false,
            messages: true,
        });
        assert!(photos() && !names() && !previews() && messages());
        set(PrivacyBlur {
            previews: true,
            ..PrivacyBlur::NONE
        });
        assert!(previews() && !messages());
        set(PrivacyBlur::NONE);
        assert!(!photos() && !names() && !previews() && !messages());
    }
}
