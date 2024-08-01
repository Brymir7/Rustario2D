use macroquad::prelude::*;
use mario_config::mario_config::{GRAVITY, MARIO_WORLD_SIZE};
use preparation::Tile;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
pub mod image_utils;
pub mod mario_config;
pub mod preparation;
use lazy_static::lazy_static;
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
        m.insert("23.png", ObjectType::Block(BlockType::Block));
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
    fn update(&mut self) {
        self.velocity = self.velocity + Vec2::new(0.0, GRAVITY as f32 * get_frame_time());
        self.velocity.x *= 0.95;
        self.pos = self.pos + self.velocity;
        if self.pos.x < 0.0 {
            self.pos.x = 0.0;
        }
        if self.pos.x + self.width as f32 > MARIO_WORLD_SIZE.width as f32 {
            self.pos.x = MARIO_WORLD_SIZE.width as f32 - self.width as f32;
        }
        if self.pos.y < 0.0 {
            self.pos.y = 0.0;
        }
        if self.pos.y + self.height as f32 > 196.0 as f32 {
            self.pos.y = 196.0 - self.height as f32;
            self.velocity.y = 0.0;
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
#[derive(Debug)]
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
        let player = Object::new_player(48, 176, 12, player_sprite);
        self.player_index = ArrayIndex {
            y: 176 / 16,
            x: 48 / 16,
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
        let max_y_index = self.objects.len() - 1;
        let max_x_index = self.objects[0].len() - 1;
        let block_below_player_y_idx = self.player_index.y + 1;
        let block_below_player_x_idx = self.player_index.x;
        let objects_iter = self.objects.split_at_mut(block_below_player_y_idx);
        let blocks_below_player = objects_iter.1;
        let blocks_above = objects_iter.0;
        if let Some(player) = blocks_above[self.player_index.y][self.player_index.x]
            .iter_mut()
            .find(|obj| matches!(obj.object_type, ObjectType::Player))
        {
            if is_key_down(KeyCode::Right) {
                player.add_horizontal_velocity(3.0 * get_frame_time());
            }
            if is_key_down(KeyCode::Left) {
                player.add_horizontal_velocity(-3.0 * get_frame_time());
            }

            if is_key_pressed(KeyCode::Space) {
                if block_below_player_y_idx < max_y_index && block_below_player_x_idx < max_x_index
                {
                    let block_below_player = &mut blocks_below_player[0][block_below_player_x_idx];
                    if block_below_player.is_empty() {
                        return;
                    }
                    let block_below_player = &mut block_below_player[0];
                    println!("{:?}", block_below_player.object_type);
                    if matches!(
                        block_below_player.object_type,
                        ObjectType::Block(BlockType::Block)
                            | ObjectType::Block(BlockType::PowerupBlock)
                            | ObjectType::Block(BlockType::MovementBlock)
                    ) {
                        player.velocity.y = -5.0;
                    }
                }
            }
        }
    }
    fn update(&mut self) {
        let prev_x = self.player_index.x;
        let prev_y = self.player_index.y;
        let new_x;
        let new_y;
        {
            let player = self.objects[prev_y][prev_x]
                .iter_mut()
                .find(|obj| matches!(obj.object_type, ObjectType::Player))
                .expect("Player not found");
            player.update();
            new_x = (player.pos.x / 16.0) as usize;
            new_y = (player.pos.y / 16.0) as usize;
        }
        if new_x != prev_x || new_y != prev_y {
            if new_y < self.objects.len() && new_x < self.objects[new_y].len() {
                let block_intersect = self.objects[new_y][new_x].iter().any(|obj| {
                    matches!(
                        obj.object_type,
                        ObjectType::Block(BlockType::Block)
                            | ObjectType::Block(BlockType::MovementBlock)
                            | ObjectType::Block(BlockType::PowerupBlock)
                    )
                });

                if !block_intersect {
                    let player_pos = self.objects[prev_y][prev_x]
                        .iter()
                        .position(|obj| matches!(obj.object_type, ObjectType::Player))
                        .expect("Player not found");
                    let player = self.objects[prev_y][prev_x].remove(player_pos);
                    self.objects[new_y][new_x].push(player);
                    self.player_index = ArrayIndex { x: new_x, y: new_y };
                } else {
                    let player = self.objects[prev_y][prev_x]
                        .iter_mut()
                        .find(|obj| matches!(obj.object_type, ObjectType::Player))
                        .expect("Player not found");
                    player.pos = Vec2::new(prev_x as f32 * 16.0, prev_y as f32 * 16.0);
                }
            }
        }
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

#[macroquad::main("Rustario Bros")]
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
