use macroquad::prelude::*;
use mario_config::mario_config::{GRAVITY, MARIO_SPRITE_BLOCK_SIZE, MARIO_WORLD_SIZE};
use preparation::Tile;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::thread::sleep;
use std::time::Duration;
pub mod image_utils;
pub mod mario_config;
pub mod preparation;
use lazy_static::lazy_static;
const DIRECTIONS: [(isize, isize); 4] = [(-1, 0), (0, -1), (0, 1), (1, 0)];
lazy_static! {
    static ref SPRITE_TYPE_MAPPING: HashMap<&'static str, ObjectType> = {
        let mut m = HashMap::new();
        m.insert("9.png", ObjectType::Block(BlockType::PowerupBlock));
        m.insert("10.png", ObjectType::Block(BlockType::Block));
        m.insert("11.png", ObjectType::Block(BlockType::Block));
        m.insert("12.png", ObjectType::Block(BlockType::Block));
        m.insert("13.png", ObjectType::Block(BlockType::Block));
        m.insert("14.png", ObjectType::Block(BlockType::Block));
        m.insert("15.png", ObjectType::Block(BlockType::Block));
        m.insert("16.png", ObjectType::Block(BlockType::Block));
        m.insert("17.png", ObjectType::Block(BlockType::Block));
        m.insert("19.png", ObjectType::Block(BlockType::Block));
        m.insert("20.png", ObjectType::Block(BlockType::Block));
        m.insert("21.png", ObjectType::Block(BlockType::Block));
        m.insert("25.png", ObjectType::Block(BlockType::Block));
        m.insert("31.png", ObjectType::Block(BlockType::Block));
        m
    };
}

#[derive(Clone, PartialEq, Copy, Debug)]
enum BlockType {
    Block,
    MovementBlock,
    Background,
    PowerupBlock,
}

#[derive(PartialEq, Clone, Copy, Debug)]
enum EnemyType {
    Goomba,
    Koopa,
}

#[derive(PartialEq, Clone, Copy, Debug)]
enum ObjectType {
    Block(BlockType),
    Enemy(EnemyType),
    Player,
    PowerUp,
}
#[derive(Clone)]
struct Object {
    pos: Vec2,
    height: usize,
    width: usize,
    max_speed: Option<i32>,
    sprite: Texture2D,
    object_type: ObjectType,
    velocity: Vec2,
}

impl Object {
    fn new(
        x: usize,
        y: usize,
        sprite: Texture2D,
        max_speed: Option<i32>,
        object_type: ObjectType,
    ) -> Object {
        Object {
            pos: Vec2::new(x as f32, y as f32),
            height: sprite.height() as usize,
            width: sprite.width() as usize,
            max_speed: max_speed,
            sprite,
            object_type,
            velocity: Vec2::new(0.0, 0.0),
        }
    }
    fn new_player(x: usize, y: usize, max_speed: i32, sprite: Texture2D) -> Object {
        Object {
            pos: Vec2::new(x as f32, y as f32),
            height: sprite.height() as usize,
            width: sprite.width() as usize,
            max_speed: Some(max_speed),
            sprite,
            object_type: ObjectType::Player,
            velocity: Vec2::new(0.0, 0.0),
        }
    }
    fn apply_gravity(&mut self) {
        self.velocity.y += GRAVITY as f32 * get_frame_time();
    }
    fn update(&mut self, surrounding_objects: Vec<Object>) {
        let block_below = surrounding_objects.iter().find(|obj| {
            obj.pos.y > self.pos.y && obj.object_type == ObjectType::Block(BlockType::Block)
        });
        if block_below.is_none() {
            self.apply_gravity();
        }
        self.velocity.x *= 0.98;
        self.pos = self.pos + self.velocity;
        for obj in &surrounding_objects {
            match obj.object_type {
                ObjectType::Block(BlockType::Block) => {
                    self.check_and_handle_collision(obj);
                }
                ObjectType::Block(BlockType::PowerupBlock) => {
                    self.check_and_handle_collision(obj);
                }
                _ => {}
            }
        }

        self.pos.x = self
            .pos
            .x
            .clamp(0.0, MARIO_WORLD_SIZE.width as f32 - self.width as f32);
        self.pos.y = self
            .pos
            .y
            .clamp(0.0, MARIO_WORLD_SIZE.height as f32 - self.height as f32);
    }

    fn check_and_handle_collision(&mut self, other: &Object) {
        let self_center = Vec2::new(
            self.pos.x + self.width as f32 / 2.0,
            self.pos.y + self.height as f32 / 2.0,
        );
        let other_center = Vec2::new(
            other.pos.x + other.width as f32 / 2.0,
            other.pos.y + other.height as f32 / 2.0,
        );
        let x_overlap =
            (self.width as f32 + other.width as f32) / 2.0 - (self_center.x - other_center.x).abs();
        let y_overlap = (self.height as f32 + other.height as f32) / 2.0
            - (self_center.y - other_center.y).abs();

        if x_overlap > 0.0 && y_overlap > 0.0 {
            let y_collision_threshold = 0.2;
            if y_overlap < self.height as f32 * y_collision_threshold {
                if self_center.y < other_center.y {
                    self.pos.y -= y_overlap;
                    self.velocity.y = 0.0;
                } else {
                    self.pos.y += y_overlap;
                    self.velocity.y = 0.0;
                }
            } else {
                if x_overlap < y_overlap {
                    if self_center.x < other_center.x {
                        self.pos.x -= x_overlap;
                        self.velocity.x = 0.0;
                    } else {
                        self.pos.x += x_overlap;
                        self.velocity.x = 0.0;
                    }
                } else {
                    if self_center.y < other_center.y {
                        self.pos.y -= y_overlap;
                        self.velocity.y = 0.0;
                    } else {
                        self.pos.y += y_overlap;
                        self.velocity.y = 0.0;
                    }
                }
            }
        }
    }

    fn add_horizontal_velocity(&mut self, velocity: f32) {
        self.velocity.x += velocity as f32;
        if let Some(max_speed) = self.max_speed {
            self.velocity.x = self.velocity.x.clamp(-max_speed as f32, max_speed as f32);
        }
    }
    fn draw(&self, camera_x: usize, camera_y: usize) {
        if self.pos.x < camera_x as f32 || self.pos.y < camera_y as f32 {
            return;
        }
        let x = self.pos.x - camera_x as f32;
        let y = self.pos.y - camera_y as f32;
        if x > MARIO_WORLD_SIZE.width as f32 || y > MARIO_WORLD_SIZE.height as f32 {
            return;
        }
        draw_texture(&self.sprite, x as f32, y as f32, WHITE);
    }
}

impl PartialEq for Object {
    fn eq(&self, other: &Self) -> bool {
        self.pos == other.pos && self.object_type == other.object_type
    }
}

struct Camera {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

impl Camera {
    fn new(width: usize, height: usize) -> Camera {
        Camera {
            x: 0,
            y: 0,
            width,
            height,
        }
    }

    fn update(&mut self, player_x: usize, player_y: usize) {
        self.x = player_x.saturating_sub(self.width / 4);
        self.y = player_y.saturating_sub(self.height);
    }
}
#[derive(Debug, Clone, Copy)]
struct ArrayIndex {
    x: usize,
    y: usize,
}
struct World {
    height: usize,
    width: usize,
    objects: Vec<Vec<Vec<Object>>>, // in a single tile theres a background Object + potentially a player, powerup or enemy
    player_index: ArrayIndex,
    camera: Camera,
}

impl World {
    fn new(height: usize, width: usize) -> World {
        let objects = vec![vec![vec![]; (width / 16) as usize]; (height / 16) as usize];
        World {
            height,
            width,
            objects: objects,
            camera: Camera::new(600, height),
            player_index: ArrayIndex { x: 0, y: 0 },
        }
    }
    async fn load_level(&mut self) {
        let mut level_data_file =
            File::open("leveldata/level_data.json").expect("Failed to open level data file");
        let mut level_data_string = String::new();
        level_data_file
            .read_to_string(&mut level_data_string)
            .expect("Failed to read level data file");
        let mut sprites_cache: HashMap<String, Texture2D> = HashMap::new();

        let level_data: Vec<Tile> =
            serde_json::from_str(&level_data_string).expect("Failed to parse level data");

        for tile in level_data {
            let sprite = if let Some(cached_sprite) = sprites_cache.get(&tile.sprite_name) {
                cached_sprite.clone()
            } else {
                let sprite = load_texture(&tile.sprite_name)
                    .await
                    .expect("Failed to load sprite");
                sprites_cache.insert(tile.sprite_name.clone(), sprite.clone());
                sprite
            };
            let object_type = SPRITE_TYPE_MAPPING
                .get(tile.sprite_name.as_str().split("/").last().unwrap())
                .cloned()
                .unwrap_or(ObjectType::Block(BlockType::Background)); // Default to Block(Wall)

            let object = Object::new(
                tile.start_x as usize,
                tile.start_y as usize,
                sprite,
                None,
                object_type,
            );
            self.add_object(object);
        }
    }
    async fn load_player(&mut self) {
        let player_sprite = load_texture("sprites/Mario.png")
            .await
            .expect("Failed to load player sprite");
        let mut player_sprite = player_sprite.get_texture_data();
        image_utils::convert_white_to_transparent(&mut player_sprite);
        let player_sprite = Texture2D::from_image(&player_sprite);
        let player = Object::new_player(48, 160, 12, player_sprite);
        self.player_index = ArrayIndex {
            y: (160.0) as usize / 16,
            x: (48.0) as usize / 16,
        };
        self.add_object(player);
    }
    fn add_object(&mut self, object: Object) {
        let x = (object.pos.x / 16.0) as usize;
        let y = (object.pos.y / 16.0) as usize;
        if y > self.objects.len() - 1 || x > self.objects[y].len() - 1 {
            println!("Object out of bounds at x: {}, y: {}", x, y);
            return;
        }
        self.objects[y][x].push(object);
    }
    fn handle_input(&mut self) {
        if let Some(player) = self.objects[self.player_index.y][self.player_index.x]
            .iter_mut()
            .find(|obj| matches!(obj.object_type, ObjectType::Player))
        {
            if is_key_down(KeyCode::Right) {
                player.add_horizontal_velocity(5.0 * get_frame_time());
            }
            if is_key_down(KeyCode::Left) {
                player.add_horizontal_velocity(-5.0 * get_frame_time());
            }

            if is_key_pressed(KeyCode::Space) {
                player.velocity.y = -3.0;
            }
        }
    }
    fn update(&mut self) {
        let objects_clone = self.objects.clone();
        let old_player_idx = self.player_index;
        for row in self.objects.iter_mut() {
            for cell in row.iter_mut() {
                for object in cell.iter_mut() {
                    match object.object_type {
                        ObjectType::Player => {
                            let surrounding_objects: Vec<_> = DIRECTIONS
                                .iter()
                                .filter_map(|(dy, dx)| {
                                    let new_y = self.player_index.y as isize + *dy;
                                    let new_x = self.player_index.x as isize + *dx;
                                    if new_y >= 0
                                        && new_y < objects_clone.len() as isize
                                        && new_x >= 0
                                        && new_x < objects_clone[0].len() as isize
                                    {
                                        Some(&objects_clone[new_y as usize][new_x as usize])
                                    } else {
                                        None
                                    }
                                })
                                .flatten()
                                .cloned()
                                .collect();
                            object.update(surrounding_objects);
                            self.player_index.x = (object.pos.x / 16.0) as usize;
                            self.player_index.y = (object.pos.y / 16.0) as usize;
                        }
                        _ => {}
                    }
                }
            }
        }
        if old_player_idx.x != self.player_index.x || old_player_idx.y != self.player_index.y {
            let player_object = self.objects[old_player_idx.y][old_player_idx.x]
                .iter()
                .find(|obj| matches!(obj.object_type, ObjectType::Player))
                .cloned();

            if let Some(player) = player_object {
                self.objects[old_player_idx.y][old_player_idx.x]
                    .retain(|obj| !matches!(obj.object_type, ObjectType::Player));
                self.objects[self.player_index.y][self.player_index.x].push(player);
            }
        }
        self.camera.update(
            self.objects[self.player_index.y][self.player_index.x]
                .iter()
                .find(|obj| matches!(obj.object_type, ObjectType::Player))
                .unwrap()
                .pos
                .x as usize,
            self.objects[self.player_index.y][self.player_index.x]
                .iter()
                .find(|obj| matches!(obj.object_type, ObjectType::Player))
                .unwrap()
                .pos
                .y as usize,
        );
    }

    fn draw(&self) {
        for row in self.objects.iter() {
            for cell in row.iter() {
                for object in cell.iter() {
                    object.draw(self.camera.x, self.camera.y);
                }
            }
        }
        if let Some(player) = self.objects[self.player_index.y][self.player_index.x]
            .iter()
            .find(|obj| matches!(obj.object_type, ObjectType::Player))
        {
            player.draw(self.camera.x, self.camera.y);
        }
    }
}
fn window_conf() -> Conf {
    Conf {
        window_title: "Rustario Bros".to_owned(),
        window_width: 600,
        window_height: 400,
        // Set the desired fps here
        window_resizable: false,
        high_dpi: true,
        fullscreen: false,
        sample_count: 1,

        ..Default::default()
    }
}
#[macroquad::main(window_conf)]
async fn main() {
    preparation::main();
    println!("Finished preparing level data");

    let mut world = World::new(MARIO_WORLD_SIZE.height, MARIO_WORLD_SIZE.width);

    world.load_level().await;
    world.load_player().await;
    println!("Finished loading level");
    request_new_screen_size(600.0, world.height as f32);

    loop {
        clear_background(BLACK);
        world.handle_input();
        world.update();
        world.draw();
        next_frame().await;
    }
}
