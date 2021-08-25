use super::Asset;
use crate::types::Rect;
use async_trait::async_trait;
use macroquad::prelude::*;
use std::collections::HashMap;
use std::path::Path;

mod deserialize {
    use crate::types::Rect;
    use serde::Deserialize;

    #[derive(Deserialize)]
    #[serde(remote = "Rect")]
    struct RectDef {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    }

    #[derive(Deserialize)]
    struct Size {
        w: f32,
        h: f32,
    }

    impl Size {
        fn relativize(&self, rect: &Rect) -> Rect {
            Rect {
                x: rect.x / self.w,
                y: rect.y / self.h,
                w: rect.w / self.w,
                h: rect.h / self.h,
            }
        }
    }

    #[derive(Deserialize)]
    #[serde(rename_all(deserialize = "camelCase"))]
    struct Frame {
        #[serde(with = "RectDef")]
        frame: Rect,
        #[serde(with = "RectDef")]
        sprite_source_size: Rect,
        source_size: Size,
        duration: f32,
    }
    impl Frame {
        fn convert(&self, size: &Size) -> super::Frame {
            // let r_frame = size.relativize(&self.frame);
            // let r_source_size = size.relativize(&self.sprite_source_size);
            super::Frame {
                src: self.frame,
                offset: [self.sprite_source_size.x, self.sprite_source_size.y],
                source_size: [self.source_size.w, self.source_size.h],
            }
        }
    }

    #[derive(Deserialize)]
    struct FrameTag {
        name: String,
        from: usize,
        to: usize,
    }
    impl FrameTag {
        fn convert(&self, frames: &[Frame], fps: f32) -> (String, Vec<usize>) {
            let frame_per_ms = fps / 1000.0;
            let mut output = Vec::new();
            for f in self.from..self.to + 1 {
                let frame = &frames[f];
                for _ in 0..(frame.duration * frame_per_ms) as usize {
                    output.push(f);
                }
            }
            (self.name.to_owned(), output)
        }
    }

    #[derive(Deserialize)]
    #[serde(rename_all(deserialize = "camelCase"))]
    struct Meta {
        image: String,
        size: Size,
        frame_tags: Vec<FrameTag>,
    }

    #[derive(Deserialize)]
    pub struct SpriteSheet {
        frames: Vec<Frame>,
        meta: Meta,
    }
    impl SpriteSheet {
        pub(super) fn convert(&self) -> super::SpriteInfo {
            super::SpriteInfo {
                frames: self
                    .frames
                    .iter()
                    .map(|f| f.convert(&self.meta.size))
                    .collect(),
                animations: self
                    .meta
                    .frame_tags
                    .iter()
                    .map(|t| t.convert(&self.frames, 60.0))
                    .collect(),
            }
        }
        pub fn get_image_filename(&self) -> &str {
            &self.meta.image
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Frame {
    pub src: Rect,
    pub offset: [f32; 2],
    pub source_size: [f32; 2],
}

#[derive(Debug)]
struct SpriteInfo {
    frames: Vec<Frame>,
    animations: HashMap<String, Vec<usize>>,
}

pub struct AnimatedSprite {
    pub src: Texture2D,
    info: SpriteInfo,
}

impl AnimatedSprite {
    pub async fn from_file(
        filepath: &Path,
    ) -> std::result::Result<AnimatedSprite, SpritesheetImportError> {
        let path = filepath;

        let file = load_string(filepath.to_str().unwrap()).await?;
        let v: deserialize::SpriteSheet = serde_json::from_str(&file)?;

        // let image_path = path.parent().unwrap_or(path).canonicalize()?.join(v.get_image_filename());
        let image_path = path.parent().unwrap_or(path).join(v.get_image_filename());
        let image = load_texture(image_path.to_str().unwrap()).await?;
        image.set_filter(FilterMode::Nearest);
        let info = v.convert();
        Ok(AnimatedSprite {
            src: image,
            info: info,
        })
    }

    pub fn get_anim_frame(&self, anim: &str, frame: usize) -> &Frame {
        let frame_id = self
            .info
            .animations
            .get(anim)
            .and_then(|anim_data| anim_data.get(frame))
            .cloned()
            .unwrap_or(0);

        &self.info.frames[frame_id]
    }

    pub fn get_anim_length(&self, anim: &str) -> usize {
        self.info
            .animations
            .get(anim)
            .map(|anim_data| anim_data.len())
            .unwrap_or(0)
    }

    pub fn draw(&self, dest: cgmath::Point2<f32>, anim: &str, frame: usize, flip_h: bool) {
        let frame_info = self.get_anim_frame(anim, frame);
        let mut offset: cgmath::Vector2<f32> = frame_info.offset.into();
        // offset.x *= if flip_h { -1.0 } else { 1.0 };
        // offset -= cgmath::Vector2::new(self.info.size[0], self.info.size[1]) / 2.0;
        let actual_dest = dest + offset;
        draw_texture_ex(
            self.src,
            actual_dest.x,
            actual_dest.y,
            WHITE,
            DrawTextureParams {
                source: Some(frame_info.src.into()),
                flip_x: flip_h,
                ..Default::default()
            },
        );

        // let params = graphics::DrawParam::default()
        //     .src(frame_info.src)
        //     .dest(dest + offset)
        //     .scale([if flip_h { -1.0 } else { 1.0 }, 1.0]);
        // graphics::draw(ctx, &self.src, params)
    }
}

#[async_trait]
impl Asset for AnimatedSprite {
    async fn load(path: &Path) -> anyhow::Result<Self> {
        Ok(Self::from_file(path).await?)
    }
    fn delete(&self) {
        self.src.delete();
    }
}

use std::fmt;
#[derive(Debug)]
pub enum SpritesheetImportError {
    JSONError(serde_json::Error),
    FileError(FileError),
}

impl fmt::Display for SpritesheetImportError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SpritesheetImportError::JSONError(e) => write!(f, "Error loading JSON: {}", e),
            SpritesheetImportError::FileError(e) => write!(f, "Error loading file: {}", e),
        }
    }
}

impl std::error::Error for SpritesheetImportError {}

impl From<serde_json::Error> for SpritesheetImportError {
    fn from(err: serde_json::Error) -> SpritesheetImportError {
        SpritesheetImportError::JSONError(err)
    }
}

impl From<FileError> for SpritesheetImportError {
    fn from(v: FileError) -> Self {
        Self::FileError(v)
    }
}
