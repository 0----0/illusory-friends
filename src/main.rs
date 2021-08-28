#![feature(result_cloned)]
#![feature(option_get_or_insert_default)]

use std::cell::RefCell;
use std::rc::Rc;

use bmfont::CharPosition;
use colors::DARK;
use futures::executor::LocalSpawner;
use futures::task::LocalSpawnExt;
use futures::Future;
use hecs::{Entity, World};
use macroquad::prelude::*;

use serde::Deserialize;
use serde::Serialize;
use serde_with::serde_as;
use serde_with::{DeserializeAs, SerializeAs};

mod assets;
mod colors;
mod editor;
mod types;
mod ustr;

use ustr::*;

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
#[derive(Clone, Copy, Serialize, Deserialize, Default)]
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

#[derive(Clone, Copy, Serialize, Deserialize)]
enum InteractableType {
    Lamp,
    Ghost,
}

impl Default for InteractableType {
    fn default() -> Self {
        Self::Lamp
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, Default)]
struct Interactable {
    #[serde(with = "RectDef")]
    bounds: Rect,
    interaction: InteractableType,
    priority: i32,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
struct FollowComponent {
    target: Entity,
    max_distance: f32,
    speed: f32,
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
                animation: Ustr::from("Idle").unwrap(),
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

    fn follow(&mut self) {
        let mut adjustments = Vec::new();
        for (id, (Position(pos), follow)) in
            self.world.query::<(&Position, &FollowComponent)>().iter()
        {
            if let Ok(mut query) = self.world.query_one::<&Position>(follow.target) {
                if let Some(Position(target_pos)) = query.get() {
                    let x_diff = target_pos.x - pos.x;
                    let y_diff = target_pos.y - pos.y;
                    if x_diff.abs() + y_diff.abs() > follow.max_distance {
                        let adjustment = if x_diff.abs() > y_diff.abs() {
                            vec2(x_diff.abs().min(follow.speed).copysign(x_diff), 0.)
                        } else {
                            vec2(0., y_diff.abs().min(follow.speed).copysign(y_diff))
                        };

                        adjustments.push((id, adjustment));
                    }
                }
            }
        }
        for (id, adjustment) in adjustments {
            let query = self.world.query_one_mut::<&mut Position>(id).unwrap();
            query.0 += adjustment;
        }
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

    fn update(&mut self, assets: &Assets, events: &mut Vec<Event>, allow_input: bool) {
        if allow_input {
            if let Ok((Position(pos), sprite, animation)) = self.world.query_one_mut::<(
                &mut Position,
                &mut SpriteComponent,
                &mut AnimationComponent,
            )>(self.player)
            {
                if is_key_down(KeyCode::Up) {
                    animation.animation = ustr("Back");
                    sprite.flip_h = false;
                    pos.y -= 1.0;
                }
                if is_key_down(KeyCode::Down) {
                    animation.animation = ustr("Idle");
                    sprite.flip_h = false;
                    pos.y += 1.0;
                }
                if is_key_down(KeyCode::Left) {
                    animation.animation = ustr("Right");
                    sprite.flip_h = true;
                    pos.x -= 1.0;
                }
                if is_key_down(KeyCode::Right) {
                    animation.animation = ustr("Right");
                    sprite.flip_h = false;
                    pos.x += 1.0;
                }
            }
            self.follow();
        }
        self.resolve_penetrations(self.player);
        if allow_input {
            if is_key_pressed(KeyCode::Space) {
                self.interact(self.player, events);
            }
        }
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

    fn interact(&mut self, entity: Entity, events: &mut Vec<Event>) {
        let pos = match self.world.query_one_mut::<&Position>(entity) {
            Ok(Position(pos)) => *pos,
            Err(_) => return,
        };

        let mut interactables: Vec<_> = self
            .world
            .query_mut::<(&Position, &Interactable)>()
            .into_iter()
            .collect();
        interactables.sort_by_key(|(_id, (Position(..), Interactable { priority, .. }))| priority);
        for (
            id,
            (
                Position(interactable_pos),
                Interactable {
                    bounds,
                    interaction,
                    ..
                },
            ),
        ) in interactables.iter().rev()
        {
            if bounds.offset(*interactable_pos).contains(pos) {
                events.push(Event::Interaction {
                    entity: *id,
                    interaction: *interaction,
                });
                return;
            }
        }
    }

    fn draw_collisions(&self) {
        for (_id, (Position(pos), CollisionComponent { bounds })) in self
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

    fn draw_interactions(&self) {
        for (_id, (Position(pos), Interactable { bounds, .. })) in
            self.world.query::<(&Position, &Interactable)>().iter()
        {
            let rect = bounds.offset(*pos);
            draw_rectangle(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                color_u8!(251., 242., 54., 64.),
            );
        }
    }
}

enum WaitingFor {
    Confirm(futures::channel::oneshot::Sender<()>),
    Choice(futures::channel::oneshot::Sender<usize>),
    Auto(futures::channel::oneshot::Sender<()>),
    Nothing,
}

impl Default for WaitingFor {
    fn default() -> Self {
        Self::Nothing
    }
}

#[derive(Clone, Copy)]
enum PortraitOrientation {
    Left,
    Right,
}

#[derive(Default)]
struct Dialogue {
    shown: bool,
    current_text: String,
    current_progress: usize,
    waiting_for: WaitingFor,
    choices: Option<Vec<String>>,
    current_choice: usize,
    portrait: Option<(SpriteComponent, PortraitOrientation)>,
}

impl Dialogue {
    fn set_text(&mut self, text: String) {
        self.shown = true;
        self.current_text = text;
        self.current_progress = 0;
    }

    fn update(&mut self) {
        self.current_progress += 1;
        if let Some(choices) = &self.choices {
            if is_key_pressed(KeyCode::Up) {
                self.current_choice = match self.current_choice {
                    0 => choices.len() - 1,
                    _ => self.current_choice - 1,
                };
            }
            if is_key_pressed(KeyCode::Down) {
                self.current_choice = match self.current_choice {
                    c if c >= choices.len() - 1 => 0,
                    _ => self.current_choice + 1,
                };
            }
        }

        if self.current_progress >= self.current_text.len() {
            match std::mem::replace(&mut self.waiting_for, WaitingFor::Nothing) {
                WaitingFor::Auto(sender) => {
                    sender.send(()).unwrap();
                }
                other => {
                    self.waiting_for = other;
                }
            };
        }

        if is_key_pressed(KeyCode::Space) {
            match std::mem::replace(&mut self.waiting_for, WaitingFor::Nothing) {
                WaitingFor::Confirm(sender) => {
                    sender.send(()).unwrap();
                }
                WaitingFor::Choice(sender) => {
                    sender.send(self.current_choice).unwrap();
                    self.choices = None;
                }
                other => self.waiting_for = other,
            }
        }
    }

    fn draw(&self, assets: &Assets) {
        if self.shown {
            if let Some((portrait, orientation)) = &self.portrait {
                let base = match orientation {
                    PortraitOrientation::Left => (64., 128.),
                    PortraitOrientation::Right => (448., 128.),
                };
                draw_texture_ex(
                    *assets.get(&portrait.texture),
                    base.0,
                    base.1,
                    WHITE,
                    DrawTextureParams {
                        source: portrait.source,
                        ..Default::default()
                    },
                );
            }
            let num_chars = std::cmp::min(self.current_text.len(), self.current_progress);
            let ninebox = assets.get(&assets.get_texture("ninebox"));
            draw_nine_box(*ninebox, 32., 224., 576., 128.);
            draw_text_bmfont(
                assets,
                &self.current_text[0..num_chars],
                72.,
                264.,
                colors::LIGHT,
                Justify::Left,
            );
            if let Some(choices) = &self.choices {
                let mut x = 416.;
                let mut y = 112.;
                let mut width = 224.;
                let mut height = 128.;
                x -= 32.0;
                width += 32.0;
                match choices.len() {
                    3 => {
                        y -= 32.;
                        height += 32.;
                    }
                    _ => {}
                }
                draw_nine_box(*ninebox, x, y, width, height);
                for (i, c) in choices.iter().enumerate() {
                    let draw_text = |text: &str| {
                        draw_text_bmfont(
                            assets,
                            text,
                            x + width - 40.,
                            y + 40. + 30. * (i as f32),
                            colors::LIGHT,
                            Justify::Right,
                        );
                    };
                    if i == self.current_choice {
                        draw_text(&format!("> {}", c));
                    } else {
                        draw_text(c);
                    };
                }
            }
            // draw_text(
            //     &self.current_text[0..num_chars],
            //     100.,
            //     300.,
            //     50.,
            //     colors::LIGHT,
            // );
        }
    }
}

enum Justify {
    Left,
    Right,
}

fn draw_text_bmfont(assets: &Assets, text: &str, x: f32, y: f32, color: Color, justify: Justify) {
    let bmfont = &assets.font;
    let texture_id = assets.get_texture("font");
    let texture = assets.get(&texture_id);
    let char_positions = bmfont.parse(text).unwrap();
    let draw_char_position = |c: CharPosition, offset_x: f32| {
        draw_texture_ex(
            *texture,
            x + c.screen_rect.x as f32 + offset_x,
            y + c.screen_rect.y as f32,
            color,
            DrawTextureParams {
                source: Some(Rect {
                    x: c.page_rect.x as f32,
                    y: c.page_rect.y as f32,
                    w: c.page_rect.width as f32,
                    h: c.page_rect.height as f32,
                }),
                ..Default::default()
            },
        );
    };

    match justify {
        Justify::Left => {
            for c in char_positions {
                draw_char_position(c, 0.0)
            }
        }
        Justify::Right => {
            let char_positions: Vec<_> = char_positions.collect();
            let offset_x = char_positions
                .last()
                .map(|c| -c.screen_rect.max_x())
                .unwrap_or(0) as f32;
            for c in char_positions {
                draw_char_position(c, offset_x);
            }
        }
    }
}

fn draw_nine_box(texture: Texture2D, x: f32, y: f32, width: f32, height: f32) {
    let sprite_width = texture.width() / 3.0;
    let sprite_height = texture.height() / 3.0;
    let cw = std::cmp::max(2, (width / sprite_width).floor() as i32);
    let ch = std::cmp::max(2, (height / sprite_height).floor() as i32);
    for cx in 0..cw {
        for cy in 0..ch {
            let tx = match cx {
                0 => 0.,
                cx if cx == cw - 1 => 2.,
                _ => 1.,
            };
            let ty = match cy {
                0 => 0.,
                cy if cy == ch - 1 => 2.,
                _ => 1.,
            };
            let texture_rect = Rect {
                x: tx * sprite_width,
                y: ty * sprite_height,
                w: sprite_width,
                h: sprite_height,
            };
            draw_texture_ex(
                texture,
                x + (cx as f32) * sprite_width,
                y + (cy as f32) * sprite_height,
                WHITE,
                DrawTextureParams {
                    source: Some(texture_rect),
                    ..Default::default()
                },
            )
        }
    }
}

enum Event {
    Interaction {
        entity: Entity,
        interaction: InteractableType,
    },
}

#[derive(Clone, Copy)]
enum PlayerClass {
    Witch,
    Princess,
    Knight,
}

impl PlayerClass {
    fn str(&self) -> &str {
        match self {
            PlayerClass::Witch => "WITCH",
            PlayerClass::Princess => "PRINCESS",
            PlayerClass::Knight => "KNIGHT",
        }
    }
    fn ghost_title(&self) -> &str {
        match self {
            PlayerClass::Witch => "GREAT WITCH",
            PlayerClass::Princess => "YOUR HIGHNESS",
            PlayerClass::Knight => "MY LADY",
        }
    }
}
#[derive(Clone, Copy)]
enum GhostClass {
    Ghost,
    Goblin,
    Dwarf,
}

impl GhostClass {
    fn str(&self) -> &str {
        match self {
            GhostClass::Ghost => "GHOST",
            GhostClass::Goblin => "GOBLIN",
            GhostClass::Dwarf => "DWARF",
        }
    }

    fn affectation(&self) -> &str {
        match self {
            GhostClass::Ghost => "WOO OO",
            GhostClass::Goblin => "NYEHEHEH",
            GhostClass::Dwarf => "AYE",
        }
    }
}

#[derive(Default)]
struct Info {
    player_class: Option<PlayerClass>,
    ghost_class: Option<GhostClass>,
}

struct _Game {
    overworld: Overworld,
    camera: Camera2D,
    dialogue: Dialogue,
    info: Info,
}

#[derive(Clone)]
pub struct Game(Rc<RefCell<_Game>>);

impl Game {
    fn new(assets: &Assets) -> Self {
        Self(Rc::new(RefCell::new(_Game {
            overworld: Overworld::new(assets),
            camera: Camera2D::from_display_rect(Rect::new(0.0, 0.0, 640.0, 360.0)),
            dialogue: Default::default(),
            info: Default::default(),
        })))
    }

    fn update(&self, assets: &Assets, spawner: &LocalSpawner) {
        let mut this = self.0.borrow_mut();
        let mut events = Vec::new();
        let dialogue = this.dialogue.shown;
        this.overworld.update(assets, &mut events, !dialogue);
        if dialogue {
            this.dialogue.update();
        }
        for event in events {
            match event {
                Event::Interaction {
                    entity,
                    interaction,
                } => match interaction {
                    InteractableType::Lamp => spawner
                        .spawn_local(wrap_dialogue(lamp_dialogue_tree(self.clone())))
                        .unwrap(),
                    InteractableType::Ghost => {
                        if this.info.ghost_class.is_none() {
                            spawner
                                .spawn_local(wrap_dialogue(ghost_meeting(self.clone(), entity)))
                                .unwrap()
                        } else {
                            spawner
                                .spawn_local(wrap_dialogue(ghost_after(self.clone())))
                                .unwrap();
                        }
                    }
                },
            }
        }
    }

    fn draw(&self, assets: &Assets) {
        let this = self.0.borrow();
        set_camera(&this.camera);
        this.overworld.draw(assets);
        this.dialogue.draw(assets);
    }

    fn show_text<S>(&self, text: S) -> futures::channel::oneshot::Receiver<()>
    where
        S: Into<String>,
    {
        let mut this = self.0.borrow_mut();
        this.dialogue.set_text(text.into());
        let (s, r) = futures::channel::oneshot::channel();
        this.dialogue.waiting_for = WaitingFor::Confirm(s);
        r
    }

    fn show_text_auto<S>(&self, text: S) -> futures::channel::oneshot::Receiver<()>
    where
        S: Into<String>,
    {
        let mut this = self.0.borrow_mut();
        this.dialogue.set_text(text.into());
        let (s, r) = futures::channel::oneshot::channel();
        this.dialogue.waiting_for = WaitingFor::Auto(s);
        r
    }

    fn show_choice(
        &self,
        choices: impl IntoIterator<Item = impl Into<String>>,
    ) -> futures::channel::oneshot::Receiver<usize> {
        let mut this = self.0.borrow_mut();
        this.dialogue.choices = Some(choices.into_iter().map(Into::into).collect());
        this.dialogue.current_choice = 0;
        let (s, r) = futures::channel::oneshot::channel();
        this.dialogue.waiting_for = WaitingFor::Choice(s);
        r
    }

    fn show_portrait(&self, portrait: Option<(Portrait, PortraitOrientation)>) {
        let mut this = self.0.borrow_mut();
        this.dialogue.portrait = portrait.map(|(p, o)| {
            (
                match p {
                    Portrait::Maribelle => SpriteComponent {
                        texture: "maribelleportrait".into(),
                        ..Default::default()
                    },
                    Portrait::Ghost => SpriteComponent {
                        texture: "ghostportrait".into(),
                        ..Default::default()
                    },
                },
                o,
            )
        });
    }

    fn end_dialogue(&self) {
        let mut this = self.0.borrow_mut();
        this.dialogue.shown = false;
        this.dialogue.portrait = None;
        this.dialogue.choices = None;
        this.dialogue.current_choice = 0;
        this.dialogue.waiting_for = WaitingFor::Nothing;
    }

    // fn dialogue_mut(&self) -> RefMut<Dialogue> {
    //     RefMut::map(self.0.borrow_mut(), |this| &mut this.dialogue)
    // }
}

#[derive(Clone, Copy)]
enum Portrait {
    Maribelle,
    Ghost,
}

async fn lamp_dialogue_tree(game: Game) -> anyhow::Result<()> {
    let m = Some((Portrait::Maribelle, PortraitOrientation::Right));
    game.show_portrait(m);
    game.show_text("IT'S A LAMP.").await?;
    game.show_text("I WISH IT WERE A BIT BRIGHTER...").await?;
    game.end_dialogue();
    Ok(())
}

async fn ghost_customize_player_class(game: Game) -> anyhow::Result<()> {
    let m = Some((Portrait::Maribelle, PortraitOrientation::Right));
    let g = Some((Portrait::Ghost, PortraitOrientation::Left));
    let player_class_id = game
        .show_choice(["A WITCH", "A PRINCESS", "A KNIGHT"])
        .await?;
    let player_class = match player_class_id {
        0 => PlayerClass::Witch,
        1 => PlayerClass::Princess,
        _ => PlayerClass::Knight,
    };
    match player_class {
        PlayerClass::Witch => {
            game.show_portrait(m);
            game.show_text("I AM THE GREAT WITCH, MARIBELLE.\nYOU ARE A SERVANT I HAVE CONJURED.")
                .await?;
            game.show_portrait(g);
            game.show_text("WOW! YOU CREATED ME?\nYOUR MAGIC IS REALLY POWERFUL!")
                .await?;
        }
        PlayerClass::Princess => {
            game.show_portrait(m);
            game.show_text("I AM THE CROWN PRINCESS, MARIBELLE.\nYOU ARE MY LOYAL SUBJECT.")
                .await?;
            game.show_portrait(g);
            game.show_text("THE PRINCESS? WHAT AN HONOR!\nYOUR WISH IS MY COMMAND, HIGHNESS!")
                .await?;
        }
        PlayerClass::Knight => {
            game.show_portrait(m);
            game.show_text("I AM THE QUESTING KNIGHT, MARIBELLE.\nWOULD YOU LIKE TO BE MY SQUIRE?")
                .await?;
            game.show_portrait(g);
            game.show_text("OF COURSE!\nI ALWAYS WANTED TO GO QUESTING!")
                .await?;
        }
    }
    game.end_dialogue();
    game.0.borrow_mut().info.player_class = Some(player_class);
    Ok(())
}

async fn ghost_customize_ghost_class(game: Game) -> anyhow::Result<()> {
    let m = Some((Portrait::Maribelle, PortraitOrientation::Right));
    let g = Some((Portrait::Ghost, PortraitOrientation::Left));
    let player_class = game.0.borrow().info.player_class.unwrap();
    let ghost_class_id = game.show_choice(["A GHOST", "A GOBLIN", "A DWARF"]).await?;
    let ghost_class = match ghost_class_id {
        0 => GhostClass::Ghost,
        1 => GhostClass::Goblin,
        _ => GhostClass::Dwarf,
    };
    match ghost_class {
        GhostClass::Ghost => {
            game.show_portrait(m);
            game.show_text("YOU ARE A FORGOTTEN SPIRIT,\nBROUGHT BACK BY POWERFUL NECROMANCY.")
                .await?;
            match player_class {
                PlayerClass::Witch => {
                    game.show_portrait(g);
                    game.show_text("OOOOOO!\nTHANKS FOR BRINGING ME BACK!")
                        .await?;
                }
                _ => {
                    game.show_portrait(g);
                    game.show_text("OOOOOOO!\nSPOOKY!").await?;
                }
            }
        }
        GhostClass::Goblin => {
            game.show_portrait(m);
            game.show_text("YOU ARE A GOBLIN TROUBLEMAKER,\nTERRORIZING THE LAND.")
                .await?;
            match player_class {
                PlayerClass::Knight => {
                    game.show_portrait(g);
                    game.show_text("NYEHEHEH!  I HOPE THERE'S\nNO HARD FEELINGS, MS. KNIGHT!")
                        .await?;
                }
                _ => {
                    game.show_portrait(g);
                    game.show_text("NYEHEHEH!\nSOUNDS FUN!").await?;
                }
            }
        }
        GhostClass::Dwarf => {
            game.show_portrait(m);
            game.show_text("YOU ARE A DWARVEN WARRIOR,\nWISE AND STRONG.")
                .await?;
            match player_class {
                PlayerClass::Princess => {
                    game.show_portrait(g);
                    game.show_text("AYE, MY LIEGE!\nI ALWAYS WANTED TO SAY THAT!")
                        .await?;
                }
                _ => {
                    game.show_portrait(g);
                    game.show_text("AYE, THAT I AM!\nI HOPE I GET A COOL BEARD!")
                        .await?;
                }
            }
        }
    }
    game.0.borrow_mut().info.ghost_class = Some(ghost_class);
    game.end_dialogue();
    Ok(())
}

async fn ghost_after(game: Game) -> anyhow::Result<()> {
    let m = Some((Portrait::Maribelle, PortraitOrientation::Right));
    let g = Some((Portrait::Ghost, PortraitOrientation::Left));
    let player_class = game.0.borrow().info.player_class.unwrap();
    game.show_portrait(g);
    game.show_text_auto(format!(
        "ANYTHING YOU WANT TO TALK ABOUT,\n{}?",
        player_class.ghost_title()
    ))
    .await?;
    loop {
        let choice = game.show_choice(["NOTHING", "US", "FIREBOLT"]).await?;
        match choice {
            0 => {
                game.show_text(
                    "THAT'S OKAY.  LET ME KNOW\nIF THERE'S ANYTHING YOU WANT TO CHANGE!",
                )
                .await?;
                game.end_dialogue();
                return Ok(());
            }
            1 => {
                game.show_text_auto("WHICH OF US DO YOU WANT\nTO TALK ABOUT?")
                    .await?;
                let choice = game.show_choice(["NEVERMIND", "ME", "YOU"]).await?;
                match choice {
                    0 => {
                        game.show_text_auto("OH, OKAY!\nWAS THERE ANYTHING ELSE?")
                            .await?;
                        continue;
                    }
                    1 => {
                        game.show_portrait(m);
                        game.show_text("I WANT TO BE SOMEONE DIFFERENT.").await?;
                        game.show_portrait(g);
                        game.show_text_auto("OKAY THEN!\nWHO ARE YOU?").await?;
                        ghost_customize_player_class(game.clone()).await?;
                        game.show_portrait(g);
                        game.show_text_auto("IS THERE ANYTHING ELSE\nYOU WANT TO TALK ABOUT?")
                            .await?;
                        continue;
                    }
                    _ => {
                        game.show_portrait(m);
                        game.show_text("I WANT YOU TO BE SOMEONE ELSE.").await?;
                        game.show_portrait(g);
                        game.show_text("COOL!\nA CHANGE IS ALWAYS NICE.").await?;
                        game.show_text_auto("UMM... WHAT AM I, EXACTLY?").await?;
                        ghost_customize_ghost_class(game.clone()).await?;
                        game.show_portrait(g);
                        game.show_text_auto("IS THERE ANYTHING ELSE\nYOU WANT TO TALK ABOUT?")
                            .await?;
                        continue;
                    }
                }
            }
            _ => {
                firebolt_dialogue_tree(game.clone()).await?;
                game.show_portrait(g);
                game.show_text_auto("IS THERE ANYTHING ELSE\nYOU WANT TO TALK ABOUT?")
                    .await?;
                continue;
            }
        }
    }
}

async fn ghost_meeting(game: Game, ghost: Entity) -> anyhow::Result<()> {
    let m = Some((Portrait::Maribelle, PortraitOrientation::Right));
    let g = Some((Portrait::Ghost, PortraitOrientation::Left));
    game.show_portrait(g);
    game.show_text_auto("HI THERE!\nWHO ARE YOU?").await?;
    ghost_customize_player_class(game.clone()).await?;
    game.show_portrait(g);
    game.show_text_auto("COME TO THINK OF IT...\nWHAT AM I, EXACTLY?")
        .await?;
    ghost_customize_ghost_class(game.clone()).await?;

    let player_class = game.0.borrow().info.player_class.unwrap();
    let ghost_class = game.0.borrow().info.ghost_class.unwrap();
    game.show_portrait(g);
    game.show_text(format!(
        "WELL THEN, {} MARIBELLE,\nI'LL FOLLOW YOU! {}!",
        player_class.str(),
        ghost_class.affectation()
    ))
    .await?;
    game.end_dialogue();
    let player = game.0.borrow().overworld.player;
    game.0
        .borrow_mut()
        .overworld
        .world
        .insert_one(
            ghost,
            FollowComponent {
                target: player,
                max_distance: 64.0,
                speed: 1.0,
            },
        )
        .unwrap();

    Ok(())
}

async fn firebolt_dialogue_tree(game: Game) -> anyhow::Result<()> {
    let m = Some((Portrait::Maribelle, PortraitOrientation::Right));
    let g = Some((Portrait::Ghost, PortraitOrientation::Left));
    let player_class = game.0.borrow().info.player_class.unwrap();
    game.show_portrait(g);
    game.show_text_auto("WOW!  SO THIS SPELL IS CALLED FIREBOLT!\nHOW STRONG IS IT?")
        .await?;
    let (strength, cost) = loop {
        let strength = game
            .show_choice(["VERY STRONG", "IT'S OK", "IT'S WEAK"])
            .await?;
        match strength {
            0 => {
                game.show_portrait(m);
                game.show_text("IT'S SUPER STRONG.\nIT COULD PROBABLY KILL A DRAGON.")
                    .await?;
                game.show_portrait(g);
                game.show_text(format!(
                    "WOW! THAT'S SO COOL!\nYOU MUST BE A POWERFUL {}!",
                    player_class.str()
                ))
                .await?;
                game.show_text("SINCE IT'S SO STRONG,\nHOW MUCH MANA DOES IT COST?")
                    .await?;
            }
            1 => {
                game.show_portrait(m);
                game.show_text("IT'S NOTHING SPECIAL.\nAN EVERYDAY SPELL FOR ME.")
                    .await?;
                game.show_portrait(g);
                game.show_text("THAT'S NEAT!\nI BET YOU STUDIED HARD TO LEARN IT.")
                    .await?;
                game.show_text("SO SINCE IT'S AVERAGE STRENGTH,\nHOW MUCH MANA DOES IT COST?")
                    .await?;
            }
            _ => {
                game.show_portrait(m);
                game.show_text("IT'S SUPER WEAK.\nI'M STILL LEARNING BETTER SPELLS...")
                    .await?;
                game.show_portrait(g);
                game.show_text("AW, THAT'S OKAY.\nI BET YOU'LL GET STRONGER IN NO TIME!")
                    .await?;
                game.show_text("SO SINCE IT'S PRETTY WEAK,\nHOW MUCH MANA DOES IT COST?")
                    .await?;
            }
        }
        let cost = game
            .show_choice(["LOTS OF MANA", "NOT TOO MUCH", "BARELY ANY"])
            .await?;
        match cost {
            0 => {
                game.show_portrait(m);
                game.show_text("TONS.\nONLY THE MOST POWERFUL CAN WIELD IT.")
                    .await?;
                game.show_portrait(g);
                match strength {
                    0 => {
                        game.show_text("WHOA. THAT'S ONLY FITTING\nFOR SUCH A POWERFUL SPELL!")
                            .await?;
                    }
                    1 => {
                        game.show_text(format!(
                            "WOW. BEING A {} IS HARD...\nYOU'RE SO COOL!",
                            player_class.str()
                        ))
                        .await?;
                    }
                    _ => {
                        game.show_text("WOW, THAT MUCH?\nMAYBE THIS SPELL ISN'T SO GOOD...")
                            .await?;
                    }
                }
            }
            1 => {
                game.show_portrait(m);
                game.show_text("NOT TOO MUCH.\nI CAN HANDLE IT, EASY.")
                    .await?;
                game.show_portrait(g);
                match strength {
                    0 => {
                        game.show_text("SUCH AN EFFICIENT SPELL!\nYOU'RE SO SMART!")
                            .await?;
                    }
                    1 => {
                        game.show_text("THAT'S A GREAT SPELL TO START WITH.\nGOOD THINKING!")
                            .await?;
                    }
                    _ => {
                        game.show_text("IT SOUNDS HARD TO USE,\nBUT I BET YOU'LL DO GREAT!")
                            .await?;
                    }
                }
            }
            _ => {
                game.show_portrait(m);
                game.show_text("IT'S SUPER CHEAP.\nI CAN CAST IT ALL DAY.")
                    .await?;
                game.show_portrait(g);
                match strength {
                    0 => {
                        game.show_text("WOW... IS THAT THE STRONGEST SPELL?\nTHAT'S AMAZING! THIS'LL BE A BREEZE!").await?;
                    }
                    1 => {
                        game.show_text("THAT'S GREAT! WE CAN GO\nON A WHILE WITHOUT RESTING!")
                            .await?;
                    }
                    _ => {
                        game.show_text("THAT MAKES SENSE.\nIT'S GREAT TO HAVE OPTIONS!")
                            .await?;
                    }
                }
            }
        }
        let strength_str = match strength {
            0 => "A VERY STRONG",
            1 => "A GOOD",
            _ => "A WEAK",
        };
        let cost_str = match cost {
            0 => "A LOT OF",
            1 => "SOME",
            _ => "BARELY ANY",
        };
        game.show_text(format!(
            "SO FIREBOLT IS {} SPELL THAT\nCOSTS {} MANA. ARE YOU SURE?",
            strength_str, cost_str
        ))
        .await?;
        let confirm = game.show_choice(["YES", "ACTUALLY..."]).await?;
        match confirm {
            0 => {
                game.show_text("GREAT! REMEMBER,\nYOU CAN ALWAYS CHANGE YOUR MIND!")
                    .await?;
                break (strength, cost);
            }
            _ => {
                game.show_text("OH, WANNA GO OVER IT AGAIN?\nTHAT'S OKAY!")
                    .await?;
                game.show_text("SO THIS SPELL IS CALLED FIREBOLT!\nHOW STRONG IS IT?")
                    .await?;
                continue;
            }
        }
    };

    game.end_dialogue();

    Ok(())
}

async fn wrap_dialogue(dialogue: impl Future<Output = anyhow::Result<()>>) {
    match dialogue.await {
        Ok(()) => (),
        Err(e) => {
            debug!("{:?}", e);
            ()
        }
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut assets = Assets::new().await.unwrap();
    // let mut overworld = Overworld::new(&assets);
    // let camera = Camera2D::from_display_rect(Rect::new(0.0, 0.0, 640.0, 360.0));
    let game = Game::new(&assets);
    let mut editor = OverworldEditor::default();
    editor
        .load(&mut game.0.borrow_mut().overworld)
        .await
        .unwrap();
    let mut pool = futures::executor::LocalPool::new();
    let spawner = pool.spawner();
    // let mut dialogue = false;
    let mut editor_enabled = false;

    loop {
        clear_background(DARK);

        // set_camera(&camera);

        // overworld.update(&assets);
        // overworld.draw(&assets);
        game.update(&assets, &spawner);
        game.draw(&assets);
        // if !dialogue {
        //     spawner
        //         .spawn_local(wrap_dialogue(firebolt_dialogue_tree(game.clone())))
        //         .unwrap();
        //     dialogue = true;
        // }

        if is_key_down(KeyCode::LeftShift) {
            if is_key_pressed(KeyCode::I) {
                game.end_dialogue();
            }
            if is_key_pressed(KeyCode::J) {
                editor_enabled = !editor_enabled;
            }
            if is_key_pressed(KeyCode::R) {
                match assets.reload().await {
                    Ok(()) => {}
                    Err(e) => println!("Failed to reload assets: {:?}", e),
                };
            }
        }

        if editor_enabled {
            editor.update(&assets, &game).await;
        }

        pool.run_until_stalled();
        next_frame().await
    }
}
