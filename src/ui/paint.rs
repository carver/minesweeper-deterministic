//! Draws squares and the face button in the classic Minesweeper style,
//! scaled to any square size.

use eframe::egui::{
    Align2, Color32, FontId, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, Vec2, pos2, vec2,
};

use crate::game::{CellInfo, Look};

const FACE_GRAY: Color32 = Color32::from_rgb(0xc0, 0xc0, 0xc0);
const LIGHT: Color32 = Color32::WHITE;
const SHADOW: Color32 = Color32::from_rgb(0x80, 0x80, 0x80);
const OPEN: Color32 = Color32::from_rgb(0xbd, 0xbd, 0xbd);
const GRID: Color32 = Color32::from_rgb(0x7b, 0x7b, 0x7b);
const MINE: Color32 = Color32::BLACK;
const EXPLODED: Color32 = Color32::from_rgb(0xff, 0x00, 0x00);
const FLAG: Color32 = Color32::from_rgb(0xff, 0x00, 0x00);
const MISSED_TINT: Color32 = Color32::from_rgba_premultiplied(77, 0, 0, 77);
const MAGIC_TINT: Color32 = Color32::from_rgba_premultiplied(77, 77, 0, 77);

const NUMBER_COLOURS: [Color32; 8] = [
    Color32::from_rgb(0x00, 0x00, 0xff),
    Color32::from_rgb(0x00, 0x80, 0x00),
    Color32::from_rgb(0xff, 0x00, 0x00),
    Color32::from_rgb(0x00, 0x00, 0x80),
    Color32::from_rgb(0x80, 0x00, 0x00),
    Color32::from_rgb(0x00, 0x80, 0x80),
    Color32::from_rgb(0x00, 0x00, 0x00),
    Color32::from_rgb(0x80, 0x80, 0x80),
];

/// `pressed` shows a covered square pushed in, as while the mouse is held
/// on it.
pub fn cell(painter: &Painter, rect: Rect, info: CellInfo, pressed: bool) {
    match info.look {
        Look::Covered if pressed => open(painter, rect, OPEN),
        Look::Covered => raised(painter, rect),
        Look::Flag => {
            raised(painter, rect);
            flag(painter, rect);
        }
        Look::Number(n) => {
            open(painter, rect, OPEN);
            number(painter, rect, n);
        }
        Look::Mine => {
            open(painter, rect, OPEN);
            mine(painter, rect);
        }
        Look::Exploded => {
            open(painter, rect, EXPLODED);
            mine(painter, rect);
        }
        Look::WrongFlag => {
            open(painter, rect, OPEN);
            mine(painter, rect);
            cross(painter, rect);
        }
    }
    if info.missed {
        painter.rect_filled(rect, 0.0, MISSED_TINT);
    }
    if info.magic {
        painter.rect_filled(rect, 0.0, MAGIC_TINT);
    }
}

fn bevel(rect: Rect) -> f32 {
    (rect.width() / 8.0).max(1.0).round()
}

/// A bevelled, unopened square.
fn raised(painter: &Painter, rect: Rect) {
    bevelled(painter, rect, bevel(rect), LIGHT, SHADOW);
}

/// The recessed frame around the board. `border` is its thickness.
pub fn sunken_frame(painter: &Painter, rect: Rect, border: f32) {
    bevelled(painter, rect, (border / 3.0).round(), SHADOW, LIGHT);
}

fn bevelled(painter: &Painter, rect: Rect, b: f32, top_left: Color32, bottom_right: Color32) {
    painter.rect_filled(rect, 0.0, FACE_GRAY);
    let (min, max) = (rect.min, rect.max);
    painter.add(Shape::convex_polygon(
        vec![
            min,
            pos2(max.x, min.y),
            pos2(max.x - b, min.y + b),
            pos2(min.x + b, max.y - b),
            pos2(min.x, max.y),
        ],
        top_left,
        Stroke::NONE,
    ));
    painter.add(Shape::convex_polygon(
        vec![
            max,
            pos2(min.x, max.y),
            pos2(min.x + b, max.y - b),
            pos2(max.x - b, min.y + b),
            pos2(max.x, min.y),
        ],
        bottom_right,
        Stroke::NONE,
    ));
    painter.rect_filled(rect.shrink(b), 0.0, FACE_GRAY);
}

fn open(painter: &Painter, rect: Rect, fill: Color32) {
    painter.rect_filled(rect, 0.0, fill);
    let line = Stroke::new(1.0, GRID);
    painter.line_segment([rect.left_top(), rect.right_top()], line);
    painter.line_segment([rect.left_top(), rect.left_bottom()], line);
}

fn number(painter: &Painter, rect: Rect, n: u8) {
    let Some(&colour) = n.checked_sub(1).and_then(|i| NUMBER_COLOURS.get(usize::from(i))) else {
        return;
    };
    let font = FontId::monospace(rect.height() * 0.75);
    let centre = rect.center() + vec2(0.0, rect.height() * 0.03);
    // Drawn twice, one pixel apart, for the heavy classic digits.
    for dx in [0.0, 1.0] {
        painter.text(
            centre + vec2(dx, 0.0),
            Align2::CENTER_CENTER,
            n,
            font.clone(),
            colour,
        );
    }
}

fn mine(painter: &Painter, rect: Rect) {
    let c = rect.center();
    let r = rect.width() * 0.28;
    let spike = Stroke::new((rect.width() / 12.0).max(1.0), MINE);
    for (dx, dy) in [(1.0, 0.0), (0.0, 1.0), (0.7, 0.7), (0.7, -0.7)] {
        let d = vec2(dx, dy) * r * 1.35;
        painter.line_segment([c - d, c + d], spike);
    }
    painter.circle_filled(c, r, MINE);
    painter.circle_filled(c - Vec2::splat(r * 0.35), r * 0.25, LIGHT);
}

fn flag(painter: &Painter, rect: Rect) {
    let s = rect.width();
    let at = |x: f32, y: f32| rect.min + vec2(x * s, y * s);
    painter.add(Shape::convex_polygon(
        vec![at(0.6, 0.14), at(0.6, 0.56), at(0.18, 0.35)],
        FLAG,
        Stroke::NONE,
    ));
    painter.line_segment(
        [at(0.6, 0.14), at(0.6, 0.72)],
        Stroke::new((s / 12.0).max(1.0), MINE),
    );
    painter.rect_filled(Rect::from_min_max(at(0.42, 0.68), at(0.72, 0.76)), 0.0, MINE);
    painter.rect_filled(Rect::from_min_max(at(0.30, 0.74), at(0.82, 0.82)), 0.0, MINE);
}

fn cross(painter: &Painter, rect: Rect) {
    let r = rect.shrink(rect.width() * 0.18);
    let stroke = Stroke::new((rect.width() / 10.0).max(1.0), FLAG);
    painter.line_segment([r.left_top(), r.right_bottom()], stroke);
    painter.line_segment([r.right_top(), r.left_bottom()], stroke);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mood {
    Happy,
    /// The mouse is held on the board.
    Tense,
    Dead,
    Won,
}

pub fn face(painter: &Painter, rect: Rect, mood: Mood, pressed: bool) {
    let b = bevel(rect).min(3.0);
    if pressed {
        painter.rect_filled(rect, 0.0, FACE_GRAY);
        painter.rect_stroke(rect, 0.0, Stroke::new(1.0, SHADOW), StrokeKind::Inside);
    } else {
        bevelled(painter, rect, b, LIGHT, SHADOW);
    }
    let shift = if pressed { vec2(1.0, 1.0) } else { Vec2::ZERO };
    let c = rect.center() + shift;
    let r = rect.width() * 0.36;
    let ink = Stroke::new((r / 9.0).max(1.0), MINE);
    painter.circle(c, r, Color32::YELLOW, ink);

    let eye = |dx: f32| c + vec2(dx * r, -0.28 * r);
    match mood {
        Mood::Dead => {
            for dx in [-0.38, 0.38] {
                let e = eye(dx);
                let d = r * 0.14;
                painter.line_segment([e - vec2(d, d), e + vec2(d, d)], ink);
                painter.line_segment([e + vec2(-d, d), e + vec2(d, -d)], ink);
            }
        }
        Mood::Won => {
            let lens = |dx: f32| Rect::from_center_size(eye(dx), vec2(r * 0.5, r * 0.3));
            painter.rect_filled(lens(-0.33), 2.0, MINE);
            painter.rect_filled(lens(0.33), 2.0, MINE);
            painter.line_segment([eye(-0.9), eye(0.9)], ink);
        }
        Mood::Happy | Mood::Tense => {
            painter.circle_filled(eye(-0.38), r * 0.1, MINE);
            painter.circle_filled(eye(0.38), r * 0.1, MINE);
        }
    }
    let mouth_y = c.y + 0.4 * r;
    match mood {
        Mood::Tense => {
            painter.circle_stroke(pos2(c.x, mouth_y), r * 0.17, ink);
        }
        Mood::Dead => arc(painter, pos2(c.x, mouth_y + 0.25 * r), r * 0.4, false, ink),
        Mood::Happy | Mood::Won => arc(painter, pos2(c.x, mouth_y - 0.25 * r), r * 0.45, true, ink),
    }
}

/// A half-circle mouth, curving down for a smile or up for a frown.
fn arc(painter: &Painter, centre: Pos2, radius: f32, smile: bool, stroke: Stroke) {
    let sign = if smile { 1.0 } else { -1.0 };
    let points = (0..=12)
        .map(|i| {
            let t = std::f32::consts::PI * (0.15 + 0.7 * i as f32 / 12.0);
            centre + vec2(-t.cos() * radius, sign * t.sin() * radius * 0.7)
        })
        .collect();
    painter.line(points, stroke);
}
