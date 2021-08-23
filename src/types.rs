use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, Debug)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Default for Rect {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            w: 16.0,
            h: 16.0,
        }
    }
}

impl Rect {
    pub fn left(&self) -> f32 {
        self.x
    }

    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    pub fn top(&self) -> f32 {
        self.y
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }

    pub fn center(&self) -> mint::Point2<f32> {
        mint::Point2 {
            x: self.x + self.w / 2.0,
            y: self.y + self.h / 2.0,
        }
    }

    pub fn combine_with(self, other: Rect) -> Rect {
        let x = f32::min(self.x, other.x);
        let y = f32::min(self.y, other.y);
        let w = f32::max(self.right(), other.right()) - x;
        let h = f32::max(self.bottom(), other.bottom()) - y;
        Rect { x, y, w, h }
    }

    pub fn flip_h(&self, flipped: bool) -> Rect {
        Rect {
            x: if flipped { -self.right() } else { self.left() },
            y: self.y,
            w: self.w,
            h: self.h,
        }
    }

    pub fn offset(&self, x: f32, y: f32) -> Rect {
        Rect {
            x: self.x + x,
            y: self.y + y,
            w: self.w,
            h: self.h,
        }
    }

    pub fn overlaps(&self, other: &Rect) -> bool {
        self.left() <= other.right()
            && self.right() >= other.left()
            && self.top() <= other.bottom()
            && self.bottom() >= other.top()
    }

    pub fn scale(&self, scale: f32) -> Rect {
        Rect {
            x: self.x * scale,
            y: self.y * scale,
            w: self.w * scale,
            h: self.h * scale,
        }
    }

    pub fn align(&self) -> Rect {
        let left = self.left().round();
        let right = self.right().round();
        let top = self.top().round();
        let bottom = self.bottom().round();

        Rect {
            x: left,
            y: top,
            w: right - left,
            h: bottom - top,
        }
    }
    pub fn normalize(&self) -> Rect {
        let mut rect = *self;
        if rect.w < 0.0 {
            rect.x = rect.x + rect.w;
            rect.w = -rect.w;
        }
        if rect.h < 0.0 {
            rect.y = rect.y + rect.h;
            rect.h = -rect.h;
        }
        rect
    }
}

impl From<macroquad::math::Rect> for Rect {
    fn from(r: macroquad::math::Rect) -> Self {
        Self {
            x: r.x,
            y: r.y,
            w: r.w,
            h: r.h,
        }
    }
}

impl From<Rect> for macroquad::math::Rect {
    fn from(r: Rect) -> Self {
        Self {
            x: r.x,
            y: r.y,
            w: r.w,
            h: r.h,
        }
    }
}

// impl From<Rect> for egui::Rect {
//     fn from(r: Rect) -> Self {
//         Self {
//             min: egui::Pos2 {
//                 x: r.left(),
//                 y: r.top()
//             },
//             max: egui::Pos2 {
//                 x: r.right(),
//                 y: r.bottom()
//             }
//         }
//     }
// }
