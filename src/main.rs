#![feature(result_cloned)]

use macroquad::prelude::*;
use macroquad::ui::{
    hash, root_ui,
    widgets::{self, Group},
    Drag, Ui,
};
use hecs::{World, Entity};
use ustr::Ustr;

mod colors;
mod assets;
mod types;

use assets::Assets;
use assets::AnimatedSpriteId;

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

#[derive(Clone, Copy)]
struct Position(f32, f32);

struct AnimatedSpriteComponent {
    id: AnimatedSpriteId,
    flip_h: bool,
    offset: [f32; 2],
    animation: Ustr,
    frame: usize,
}

impl Default for AnimatedSpriteComponent {
    fn default() -> Self {
        Self {
            id: Default::default(),
            flip_h: false,
            offset: [0.0, 0.0],
            animation: Ustr::from("Idle"),
            frame: 0,
        }
    }
}

struct Overworld {
    world: World,
    player: Entity,
}

impl Overworld {
    fn new(assets: &Assets) -> Self {
        let mut world = World::new();
        world.spawn((
            Position(0.0f32, 0.0f32),
            *assets.char_concept.get()
        ));
        let player = world.spawn((
            Position(0.0f32, 0.0f32),
            AnimatedSpriteComponent {
                id: assets.char_sprite,
                ..Default::default()
            }
        ));
        // let lad = world.spawn(((123, 123), true));
        // let mut query = world.query_one_mut::<((&mut f32, &mut f32), &mut u32)>(player).unwrap();
        // let mut query = world.query_one_mut::<(&mut (i32, i32), &bool)>(lad).unwrap();
        Self {
            world,
            player
        }
    }

    fn draw(&self, assets: &Assets) {
        for (_id, (&Position(x, y), &texture)) in self.world.query::<(&Position, &Texture2D)>().iter() {
            draw_texture(*assets.char_concept.get(), x, y, WHITE);
        }

        for (_id, (&Position(x, y), sprite))
        in self.world.query::<(&Position, &AnimatedSpriteComponent)>().iter() {
            assets.get(&sprite.id).draw(
                [x+sprite.offset[0],y+sprite.offset[1]].into(),
                sprite.animation.as_str(),
                sprite.frame,
                sprite.flip_h
            );
        }
    }

    fn tick_animations(&mut self, assets: &Assets) {
        let query = self.world.query_mut::<&mut AnimatedSpriteComponent>();
        for (_id, sprite) in query {
            sprite.frame += 1;
            if sprite.frame >= assets.get(&assets.char_sprite).get_anim_length(sprite.animation.as_str()) {
                sprite.frame = 0;
            }
        }
    }

    fn update(&mut self, assets: &Assets) {
        let (Position(x, y), sprite) = self.world.query_one_mut::<(&mut Position, &mut AnimatedSpriteComponent)>(self.player).unwrap();
        if is_key_down(KeyCode::Up) {
            sprite.animation = "Back".into();
            sprite.flip_h = false;
            *y -= 1.0;
        }
        if is_key_down(KeyCode::Down) {
            sprite.animation = "Idle".into();
            sprite.flip_h = false;
            *y += 1.0;
        }
        if is_key_down(KeyCode::Left) {
            sprite.animation = "Right".into();
            sprite.flip_h = true;
            *x -= 1.0;
        }
        if is_key_down(KeyCode::Right) {
            sprite.animation = "Right".into();
            sprite.flip_h = false;
            *x += 1.0;
        }
        self.tick_animations(assets);
    }
}


#[macroquad::main(window_conf)]
async fn main() {

    // let mut world = World::new();
    // let texture = load_texture("assets/charconcept.png").await.unwrap();
    // world.spawn(((0.0f32, 0.0f32), texture));
    let mut assets = Assets::new().await.unwrap();
    let mut overworld = Overworld::new(&assets);

    loop {
        clear_background(colors::DARK);

        if is_key_pressed(KeyCode::R) {
            assets.reload().await;
        }

        set_camera(&Camera2D::from_display_rect(Rect::new(0.0, 0.0, 640.0, 360.0)));

        // set_camera(&Camera2D {
        //     zoom: vec2(1.0, 1.0),
        //     ..Default::default()
        // });

        // draw_rectangle(screen_width() / 2.0 - 60.0, 100.0, 120.0, 60.0, colors::LIGHT);
        // draw_texture(texture, 0.0, 0.0, WHITE);

        overworld.update(&assets);
        overworld.draw(&assets);
        // for (_id, (&(x, y), &texture)) in world.query::<(&(f32, f32), &Texture2D)>().iter() {
        //     draw_texture(texture, x, y, colors::LIGHT);
        // }

        set_default_camera();

        widgets::Window::new(hash!(), vec2(50., 50.), vec2(100., 100.))
            .label("Editor")
            .titlebar(true)
            .ui(&mut *root_ui(), |ui| {
                ui.label(None, "hi");
                if ui.button(None, "Push me") {
                    println!("pushed");
                }
            });
        
        egui_macroquad::ui(|egui_ctx| {
            egui::Window::new("hi!")
                .resizable(true)
                .show(egui_ctx, |ui| {
                    ui.label("Test");
                    if ui.button("Spawn new thing").clicked() {
                        for pos in overworld.world.query_one_mut::<&Position>(overworld.player).cloned() {
                            overworld.world.spawn((pos, assets.char_concept.get().clone()));
                        }
                    }
                });
        });

        egui_macroquad::draw();
        
        next_frame().await
    }
}