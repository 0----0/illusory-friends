#![feature(result_cloned)]

use hecs::{Entity, World};
use macroquad::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use serde_with::serde_as;
use serde_with::{DeserializeAs, SerializeAs};
use ustr::Ustr;

mod assets;
mod colors;
mod editor;
mod types;

use assets::Assets;
use assets::{AnimatedSpriteId, TextureId};

use editor::{deserialize_world, OverworldEditor};

// fn main() {
//     println!("Hello, world!");
// }

fn window_conf() -> Conf {
    Conf {
        window_title: "safe".to_owned(),
        window_width: 1280,
        window_height: 720,
        window_resizable: false,
        ..Default::default()
    }
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "Rect")]
struct RectDef {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}
impl SerializeAs<Rect> for RectDef {
    fn serialize_as<S>(source: &Rect, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        RectDef::serialize(source, serializer)
    }
}
impl<'de> DeserializeAs<'de, Rect> for RectDef {
    fn deserialize_as<D>(deserializer: D) -> Result<Rect, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        RectDef::deserialize(deserializer)
    }
}

#[derive(Clone, Copy, Serialize, Deserialize)]
struct Position(Vec2);

#[serde_as]
#[derive(Clone, Copy, Serialize, Deserialize)]
struct SpriteComponent {
    texture: TextureId,
    #[serde_as(as = "Option<RectDef>")]
    source: Option<Rect>,
    offset: Vec2,
    centered: bool,
    flip_h: bool,
    layer: i32,
}

impl SpriteComponent {
    fn size(&self, assets: &Assets) -> Vec2 {
        self.source.as_ref().map(Rect::size).unwrap_or_else(|| {
            let tex = assets.get(&self.texture);
            Vec2::new(tex.width(), tex.height())
        })
    }

    fn offset(&self, assets: &Assets) -> Vec2 {
        self.offset
            + if self.centered {
                self.size(assets) * -0.5
            } else {
                vec2(0., 0.)
            }
    }

    fn bounds(&self, assets: &Assets) -> Rect {
        // self.source
        //     .unwrap_or(Rect {
        //         x: 0.,
        //         y: 0.,
        //         w: self.texture.width(),
        //         h: self.texture.height(),
        //     })
        //     .offset(self.offset)

        let size = self.size(assets);
        let offset = self.offset(assets);

        Rect {
            x: offset.x,
            y: offset.y,
            w: size.x,
            h: size.y,
        }
    }
}

#[derive(Clone, Copy, Serialize, Deserialize)]
struct AnimationComponent {
    id: AnimatedSpriteId,
    animation: Ustr,
    frame: usize,
    offset: Vec2,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
struct CollisionComponent {
    #[serde(with = "RectDef")]
    bounds: Rect,
}

#[derive(Deserialize)]
pub struct Overworld {
    #[serde(deserialize_with = "deserialize_world")]
    world: World,
    player: Entity,
}

impl Overworld {
    fn new(assets: &Assets) -> Self {
        let mut world = World::new();
        world.spawn((
            Position(vec2(0., 0.)),
            SpriteComponent {
                texture: assets.char_concept,
                source: None,
                offset: Default::default(),
                flip_h: false,
                layer: -1,
                centered: false,
            },
            CollisionComponent {
                bounds: Rect {
                    x: 64.0,
                    y: 64.0,
                    w: 128.0,
                    h: 128.0,
                },
            },
        ));
        world.spawn((
            Position(vec2(0., 0.)),
            SpriteComponent {
                texture: assets.get_texture("minewall"),
                source: None,
                offset: Default::default(),
                flip_h: false,
                layer: -1,
                centered: false,
            },
            CollisionComponent {
                bounds: Rect {
                    x: 22.,
                    y: 22.,
                    w: 211.,
                    h: 113.,
                },
            },
        ));
        world.spawn((
            Position(vec2(0., 0.)),
            SpriteComponent {
                texture: assets.get_texture("minefloor"),
                source: None,
                offset: Default::default(),
                flip_h: false,
                layer: -1,
                centered: true,
            },
        ));
        let player = world.spawn((
            Position(vec2(320., 180.)),
            SpriteComponent {
                texture: TextureId::AnimatedSpriteId(assets.char_sprite),
                source: None,
                offset: Default::default(),
                flip_h: false,
                layer: 0,
                centered: false,
            },
            AnimationComponent {
                id: assets.char_sprite,
                animation: Ustr::from("Idle"),
                frame: 0,
                offset: Default::default(),
            },
            CollisionComponent {
                bounds: Rect {
                    x: -8.,
                    y: 12.,
                    w: 16.,
                    h: 10.,
                },
            },
        ));
        Self { world, player }
    }

    fn resolve_penetrations(&mut self, entity: Entity) {
        if let Ok((&Position(pos), &CollisionComponent { bounds })) = self
            .world
            .query_one_mut::<(&Position, &CollisionComponent)>(entity)
        {
            let mut our_box = bounds.offset(pos);

            for (
                id,
                (
                    Position(other_pos),
                    CollisionComponent {
                        bounds: other_bounds,
                    },
                ),
            ) in self.world.query_mut::<(&Position, &CollisionComponent)>()
            {
                if id == entity {
                    continue;
                }
                let other_box = other_bounds.offset(*other_pos);
                if our_box.overlaps(&other_box) {
                    let leftwards_motion = other_box.left() - our_box.right();
                    let rightwards_motion = other_box.right() - our_box.left();
                    let upwards_motion = other_box.top() - our_box.bottom();
                    let downwards_motion = other_box.bottom() - our_box.top();
                    let abs_cmp = |x: &f32, y: &f32| x.abs().partial_cmp(&y.abs()).unwrap();
                    let min_horiz = std::cmp::min_by(leftwards_motion, rightwards_motion, abs_cmp);
                    let min_vert = std::cmp::min_by(upwards_motion, downwards_motion, abs_cmp);

                    match min_horiz.abs().partial_cmp(&min_vert.abs()).unwrap() {
                        std::cmp::Ordering::Less | std::cmp::Ordering::Equal => {
                            our_box.x += min_horiz;
                        }
                        std::cmp::Ordering::Greater => {
                            our_box.y += min_vert;
                        }
                    }
                }
            }

            self.world.query_one_mut::<&mut Position>(entity).unwrap().0 +=
                our_box.point() - bounds.point() - pos;
        }
    }

    fn draw(&self, assets: &Assets) {
        let mut query = self.world.query::<(&Position, &SpriteComponent)>();
        let mut drawables: Vec<_> = query.iter().collect();
        drawables.sort_by(
            |(_, (Position(pos1), sprite1)), (_, (Position(pos2), sprite2))| {
                sprite1
                    .layer
                    .cmp(&sprite2.layer)
                    .then(pos1.y.partial_cmp(&pos2.y).unwrap())
            },
        );
        for (_id, (&Position(pos), sprite)) in drawables {
            let offset = sprite.offset(assets);
            let true_x = pos.x + offset.x;
            let true_y = pos.y + offset.y;
            draw_texture_ex(
                *assets.get(&sprite.texture),
                true_x,
                true_y,
                WHITE,
                DrawTextureParams {
                    source: sprite.source,
                    flip_x: sprite.flip_h,
                    ..Default::default()
                },
            );
        }
    }

    fn tick_animations(&mut self, assets: &Assets) {
        for (_id, animation) in self.world.query_mut::<&mut AnimationComponent>() {
            animation.frame += 1;
            if animation.frame
                >= assets
                    .get(&assets.char_sprite)
                    .get_anim_length(animation.animation.as_str())
            {
                animation.frame = 0;
            }
        }

        for (_id, (sprite, animation)) in self
            .world
            .query_mut::<(&mut SpriteComponent, &AnimationComponent)>()
        {
            let frame_info = assets
                .get(&animation.id)
                .get_anim_frame(animation.animation.as_str(), animation.frame);
            sprite.offset.x = frame_info.offset[0] + animation.offset.x;
            sprite.offset.y = frame_info.offset[1] + animation.offset.y;
            if sprite.centered {
                sprite.offset.x += (frame_info.src.w - frame_info.source_size[0]) / 2.0;
                sprite.offset.y += (frame_info.src.h - frame_info.source_size[1]) / 2.0;
            }
            sprite.source = Some(frame_info.src.into());
        }
    }

    fn update(&mut self, assets: &Assets) {
        if let Ok((Position(pos), sprite, animation)) =
            self.world
                .query_one_mut::<(&mut Position, &mut SpriteComponent, &mut AnimationComponent)>(
                    self.player,
                )
        {
            if is_key_down(KeyCode::Up) {
                animation.animation = "Back".into();
                sprite.flip_h = false;
                pos.y -= 1.0;
            }
            if is_key_down(KeyCode::Down) {
                animation.animation = "Idle".into();
                sprite.flip_h = false;
                pos.y += 1.0;
            }
            if is_key_down(KeyCode::Left) {
                animation.animation = "Right".into();
                sprite.flip_h = true;
                pos.x -= 1.0;
            }
            if is_key_down(KeyCode::Right) {
                animation.animation = "Right".into();
                sprite.flip_h = false;
                pos.x += 1.0;
            }
        }
        self.resolve_penetrations(self.player);
        self.tick_animations(assets);
    }

    fn query_cursor_pos(&self, assets: &Assets, cursor: Vec2) -> Option<(Entity, Vec2)> {
        let mut query = self.world.query::<(&Position, &SpriteComponent)>();
        let mut drawables: Vec<_> = query.iter().collect();
        drawables.sort_by(
            |(_, (Position(pos1), sprite1)), (_, (Position(pos2), sprite2))| {
                sprite1
                    .layer
                    .cmp(&sprite2.layer)
                    .then(pos1.y.partial_cmp(&pos2.y).unwrap())
            },
        );
        for (id, (Position(pos), sprite)) in drawables.iter().rev() {
            let bounds = sprite.bounds(assets).offset(Vec2::new(pos.x, pos.y));
            if bounds.contains(cursor) {
                return Some((*id, *pos - cursor));
            }
        }

        None
    }

    fn draw_collisions(&self) {
        for (id, (Position(pos), CollisionComponent { bounds })) in self
            .world
            .query::<(&Position, &CollisionComponent)>()
            .iter()
        {
            let rect = bounds.offset(*pos);
            draw_rectangle(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                color_u8!(99., 155., 255., 64.),
            );
        }
    }
}

struct DialogueBranch {
    events: Vec<DialogueEvent>,
}

enum DialogueEvent {
    Text(String),
    Choice(Vec<(String, DialogueBranch)>),
}

impl DialogueBranch {
    fn new() -> Self {
        Self {
            events: Default::default(),
        }
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut assets = Assets::new().await.unwrap();
    let mut overworld = Overworld::new(&assets);
    let camera = Camera2D::from_display_rect(Rect::new(0.0, 0.0, 640.0, 360.0));
    let mut editor = OverworldEditor::default();
    editor.load(&mut overworld).await.unwrap();

    loop {
        clear_background(colors::DARK);

        if is_key_pressed(KeyCode::R) {
            match assets.reload().await {
                Ok(()) => {}
                Err(e) => println!("Failed to reload assets: {:?}", e),
            };
        }

        set_camera(&camera);

        overworld.update(&assets);
        overworld.draw(&assets);

        editor.update(&assets, &mut overworld, &camera).await;

        next_frame().await
    }
}
