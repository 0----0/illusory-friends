use crate::{
    assets::Assets, colors, AnimationComponent, CollisionComponent, Overworld, Position,
    SpriteComponent,
};
use hecs::{
    serialize::row::{try_serialize, DeserializeContext, SerializeContext},
    Entity, EntityBuilder, EntityRef, World,
};
use macroquad::prelude::*;
use serde::{ser::SerializeStruct, Deserialize, Serialize};
use std::cell::RefCell;

enum Tool {
    Select,
    Move,
    Spawn,
}

impl Default for Tool {
    fn default() -> Self {
        Self::Select
    }
}

fn rect_manual_input_ui(ui: &mut egui::Ui, rect: &mut Rect) -> egui::Response {
    ui.horizontal(|ui| {
        ui.add(egui::DragValue::new(&mut rect.x)) | ui.add(egui::DragValue::new(&mut rect.y))
    })
    .inner
        | ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut rect.w)) | ui.add(egui::DragValue::new(&mut rect.h))
        })
        .inner
}

fn vec2_manual_input_ui(ui: &mut egui::Ui, vec: &mut Vec2) -> egui::Response {
    ui.horizontal(|ui| {
        ui.add(egui::DragValue::new(&mut vec.x)) | ui.add(egui::DragValue::new(&mut vec.y))
    })
    .inner
}

fn collisions_ui(ui: &mut egui::Ui, entity: EntityRef) {
    if let Some(mut col) = entity.get_mut::<CollisionComponent>() {
        rect_manual_input_ui(ui, &mut col.bounds);
    }
}

fn position_ui(ui: &mut egui::Ui, entity: EntityRef) {
    if let Some(mut pos) = entity.get_mut::<Position>() {
        vec2_manual_input_ui(ui, &mut pos.0);
    }
}

fn sprite_ui(ui: &mut egui::Ui, entity: EntityRef) {
    if let Some(mut sprite) = entity.get_mut::<SpriteComponent>() {
        ui.label("Offset");
        vec2_manual_input_ui(ui, &mut sprite.offset);
        ui.checkbox(&mut sprite.centered, "Centered");
        if let Some(source) = &mut sprite.source {
            ui.label("Source:");
            rect_manual_input_ui(ui, source);
        } else {
            if ui.button("Add source").clicked() {
                sprite.source = Some(Default::default());
            }
        }
        ui.label("Layer:");
        ui.add(egui::DragValue::new(&mut sprite.layer));
    }
}

fn animation_ui(ui: &mut egui::Ui, entity: EntityRef) {
    if let Some(mut animation) = entity.get_mut::<AnimationComponent>() {
        ui.label("Offset:");
        vec2_manual_input_ui(ui, &mut animation.offset);
        ui.label("Frame:");
        ui.add(egui::DragValue::new(&mut animation.frame));
    }
}

impl Serialize for Overworld {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut context = OverworldSerializeContext;
        let mut state = serializer.serialize_struct("Overworld", 2)?;
        state.serialize_field("player", &self.player)?;
        state.serialize_field(
            "world",
            &SerializeWorld(RefCell::new((&mut context, &self.world))),
        )?;
        state.end()
    }
}

struct SerializeWorld<'a, C>(RefCell<(&'a mut C, &'a World)>)
where
    C: SerializeContext;

impl<'a, C> Serialize for SerializeWorld<'a, C>
where
    C: SerializeContext,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut this = self.0.borrow_mut();
        hecs::serialize::row::serialize(this.1, this.0, serializer)
    }
}

macro_rules! apply_component_ids {
    ($macro: ident) => {
        $macro! {
            Position : Position,
            Sprite : SpriteComponent,
            Collision : CollisionComponent,
            Animation : AnimationComponent,
        }
    };
}

fn duplicate_entity(entity: EntityRef, builder: &mut EntityBuilder) {
    macro_rules! duplicate_helper {
        ($($id:ident : $ty:ty,)*) => {
            $(if let Some(component) = entity.get::<$ty>() {
                builder.add((*component).clone());
            })*
        };
    }
    apply_component_ids!(duplicate_helper);
    // if let Some(component) = entity.get::<Position>() {
    //     builder.add((*component).clone());
    // }
}
struct OverworldDeserializeContext;

impl DeserializeContext for OverworldDeserializeContext {
    fn deserialize_entity<'de, M>(
        &mut self,
        mut map: M,
        entity: &mut hecs::EntityBuilder,
    ) -> Result<(), M::Error>
    where
        M: serde::de::MapAccess<'de>,
    {
        while let Some(key) = map.next_key()? {
            macro_rules! deserialize_helper {
                ($($id:ident : $ty:ty,)*) => {
                    match key {
                        $(ComponentId::$id => {
                            entity.add::<$ty>(map.next_value()?);
                        })*
                    }
                }
            }
            apply_component_ids!(deserialize_helper);
            // ComponentId::Position => entity.add::<Position>(map.next_value()?),
            // ComponentId::Sprite => todo!(),
            // ComponentId::Collision => todo!(),
            // ComponentId::Animation => todo!(),
        }
        Ok(())
    }
}

pub fn deserialize_world<'de, D>(deserializer: D) -> Result<World, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let mut context = OverworldDeserializeContext;
    hecs::serialize::row::deserialize(&mut context, deserializer)
}

#[derive(Serialize, Deserialize)]
enum ComponentId {
    Position,
    Sprite,
    Collision,
    Animation,
}

struct OverworldSerializeContext;

impl SerializeContext for OverworldSerializeContext {
    fn serialize_entity<S>(&mut self, entity: EntityRef<'_>, map: &mut S) -> Result<(), S::Error>
    where
        S: serde::ser::SerializeMap,
    {
        macro_rules! serialize_helper {
            ($($id:ident : $ty:ty,)*) => {
                $(try_serialize::<$ty, _, _>(&entity, &ComponentId::$id, map)?;)*
            }
        }
        apply_component_ids!(serialize_helper);
        // try_serialize::<Position, _, _>(&entity, &ComponentId::Position, map)?;
        // try_serialize::<SpriteComponent, _, _>(&entity, &ComponentId::Sprite, map)?;
        // try_serialize::<CollisionComponent, _, _>(&entity, &ComponentId::Collision, map)?;
        // try_serialize::<AnimationComponent, _, _>(&entity, &ComponentId::Animation, map)?;

        Ok(())
    }
}

#[derive(Default)]
pub struct OverworldEditor {
    tool: Tool,
    selected: Option<Entity>,
    drag: Option<(Entity, Vec2)>,
    show_collisions: bool,
}

impl OverworldEditor {
    fn save(&self, overworld: &Overworld) -> anyhow::Result<()> {
        // let mut output = Vec::with_capacity(128);
        // let mut serializer = serde_json::Serializer::pretty(&mut output);
        // hecs::serialize::row::serialize(
        //     &overworld.world,
        //     &mut OverworldSerializeContext {},
        //     &mut serializer,
        // )?;
        // println!(
        //     "{}",
        //     std::str::from_utf8(output.as_slice()).unwrap_or("UTF8 error")
        // );
        let file = std::fs::File::create("assets/overworld.json")?;
        serde_json::to_writer(file, overworld)?;
        // println!("{}", serde_json::to_string_pretty(overworld)?);
        Ok(())
    }

    pub async fn load(&self, overworld: &mut Overworld) -> anyhow::Result<()> {
        *overworld = serde_json::from_slice(&load_file("assets/overworld.json").await?)?;
        Ok(())
    }

    fn highlight_hovered(&self, assets: &Assets, overworld: &mut Overworld, camera: &Camera2D) {
        let cursor = camera.screen_to_world(Vec2::from(mouse_position()));

        if let Some((entity, _)) = overworld.query_cursor_pos(assets, cursor) {
            if let Ok((Position(pos), sprite)) = overworld
                .world
                .query_one_mut::<(&Position, &SpriteComponent)>(entity)
            {
                let bounds = sprite.bounds(assets).offset(*pos);
                draw_rectangle_lines(bounds.x, bounds.y, bounds.w, bounds.h, 1.0, colors::LIGHT);
            }
        }
    }

    fn highlight_selected(&self, assets: &Assets, overworld: &mut Overworld) {
        if let Some(entity) = self.selected {
            if let Ok((Position(pos), sprite)) = overworld
                .world
                .query_one_mut::<(&Position, &SpriteComponent)>(entity)
            {
                let bounds = sprite.bounds(assets).offset(*pos);
                draw_rectangle_lines(bounds.x, bounds.y, bounds.w, bounds.h, 1.0, colors::LIGHT);

                let crosshair_size = 10.0;
                draw_line(
                    pos.x,
                    pos.y - crosshair_size,
                    pos.x,
                    pos.y + crosshair_size,
                    1.0,
                    colors::BLUE,
                );
                draw_line(
                    pos.x - crosshair_size,
                    pos.y,
                    pos.x + crosshair_size,
                    pos.y,
                    1.0,
                    colors::BLUE,
                );
                draw_circle_lines(pos.x, pos.y, crosshair_size / 2.0, 1.0, colors::BLUE);
            }
        }
    }

    pub async fn update(&mut self, assets: &Assets, overworld: &mut Overworld, camera: &Camera2D) {
        let mut should_load = false;
        egui_macroquad::ui(|egui_ctx| {
            egui::Window::new("hi!")
                .resizable(true)
                .show(egui_ctx, |ui| {
                    ui.label("Test");
                    if let Some(entity) = self.selected {
                        if ui.button("Delete").clicked() {
                            overworld.world.despawn(entity).unwrap();
                        }
                        if let Ok(entity_ref) = overworld.world.entity(entity) {
                            position_ui(ui, entity_ref);
                            sprite_ui(ui, entity_ref);
                            animation_ui(ui, entity_ref);
                            collisions_ui(ui, entity_ref);
                            if ui.button("Duplicate").clicked() {
                                let mut builder = EntityBuilder::new();
                                duplicate_entity(entity_ref, &mut builder);
                                overworld.world.spawn(builder.build());
                            }
                        }
                    }
                    if ui.button("Spawn new thing").clicked() {
                        for pos in overworld
                            .world
                            .query_one_mut::<&Position>(overworld.player)
                            .cloned()
                        {
                            overworld.world.spawn((
                                pos,
                                SpriteComponent {
                                    texture: assets.char_concept,
                                    source: None,
                                    offset: Default::default(),
                                    flip_h: false,
                                    layer: -1,
                                    centered: false,
                                },
                            ));
                        }
                    }

                    if ui.button("Save").clicked() {
                        self.save(overworld)
                            .unwrap_or_else(|e| println!("Failed to save: {}", e));
                    }

                    if ui.button("Load").clicked() {
                        should_load = true;
                    }
                });

            if !egui_ctx.wants_keyboard_input() {
                if is_key_pressed(KeyCode::Q) {
                    self.tool = Tool::Select;
                }
                if is_key_pressed(KeyCode::W) {
                    self.tool = Tool::Move;
                }
                if is_key_pressed(KeyCode::E) {
                    self.tool = Tool::Spawn;
                }
                if is_key_pressed(KeyCode::H) {
                    self.show_collisions = !self.show_collisions;
                }
            }

            if self.show_collisions {
                overworld.draw_collisions();
            }

            self.highlight_selected(assets, overworld);

            if !egui_ctx.wants_pointer_input() {
                let cursor = camera.screen_to_world(Vec2::from(mouse_position()));
                match self.tool {
                    Tool::Select => {
                        self.highlight_hovered(assets, overworld, camera);
                        if is_mouse_button_pressed(MouseButton::Left) {
                            self.selected = overworld
                                .query_cursor_pos(assets, cursor)
                                .map(|(entity, _)| entity);
                        }
                    }
                    Tool::Move => {
                        self.highlight_hovered(assets, overworld, camera);
                        if is_mouse_button_pressed(MouseButton::Left) {
                            self.drag = overworld.query_cursor_pos(assets, cursor);
                        }

                        if is_mouse_button_down(MouseButton::Left) {
                            if let Some((drag, offset)) = self.drag {
                                let cursor = camera.screen_to_world(Vec2::from(mouse_position()));
                                if let Ok(pos) =
                                    overworld.world.query_one_mut::<&mut Position>(drag)
                                {
                                    *pos = Position(Vec2::new(cursor.x, cursor.y) + offset);
                                }
                            }
                        }
                    }
                    Tool::Spawn => {
                        if is_mouse_button_pressed(MouseButton::Left) {
                            overworld.world.spawn((
                                Position(cursor),
                                SpriteComponent {
                                    texture: assets.char_concept,
                                    source: None,
                                    offset: Default::default(),
                                    flip_h: false,
                                    layer: -1,
                                    centered: false,
                                },
                            ));
                        }
                    }
                }
            }
        });

        set_default_camera();
        egui_macroquad::draw();
        if should_load {
            self.load(overworld)
                .await
                .unwrap_or_else(|e| println!("Failed to load: {}", e));
        }
    }
}
