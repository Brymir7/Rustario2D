use macroquad::prelude::*;
use mario_config::mario_config::MARIO_WORLD_SIZE;
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

#[derive(Clone, PartialEq, Copy)]
enum BlockType {
    Block,
    MovementBlock,
    Background,
    PowerupBlock,
}

#[derive(PartialEq, Clone, Copy)]
enum EnemyType {
    Goomba,
    Koopa,
}

#[derive(PartialEq, Clone, Copy)]
enum ObjectType {
    Block(BlockType),
    Enemy(EnemyType),
    Player,
    PowerUp,
}

struct Object {
    x: u16,
    y: u16,
    height: u16,
    width: u16,
    max_speed: Option<u16>,
    sprite: Texture2D,
    object_type: ObjectType,
    velocity: (i16, i16),
}

impl Object {
    fn new(
        x: u16,
        y: u16,
        sprite: Texture2D,
        max_speed: Option<u16>,
        object_type: ObjectType,
    ) -> Object {
        Object {
            x,
            y,
            height: sprite.height() as u16,
            width: sprite.width() as u16,
            max_speed: max_speed,
            sprite,
            object_type,
            velocity: (0, 0),
        }
    }
    fn new_player(x: u16, y: u16, max_speed: u16, sprite: Texture2D) -> Object {
        Object {
            x,
            y,
            height: sprite.height() as u16,
            width: sprite.width() as u16,
            max_speed: Some(max_speed),
            sprite,
            object_type: ObjectType::Player,
            velocity: (0, 0),
        }
    }
    fn update(&mut self) {
        self.x = (self.x as i16 + self.velocity.0) as u16;
        self.y = (self.y as i16 + self.velocity.1) as u16;
    }

    fn draw(&self, camera_x: u16, camera_y: u16) {
        if self.x < camera_x || self.y < camera_y {
            return;
        }
        let x = self.x - camera_x;
        let y = self.y - camera_y;
        if x > MARIO_WORLD_SIZE.width || y > MARIO_WORLD_SIZE.height {
            return;
        }
        draw_texture(&self.sprite, x as f32, y as f32, WHITE);
    }
}

impl PartialEq for Object {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y && self.object_type == other.object_type
    }
}

struct Camera {
    x: u16,
    y: u16,
    width: u16,
    height: u16,
}

impl Camera {
    fn new(width: u16, height: u16) -> Camera {
        Camera {
            x: 0,
            y: 0,
            width,
            height,
        }
    }

    fn update(&mut self, player_x: u16, player_y: u16) {
        self.x = player_x.saturating_sub(self.width / 4);
        self.y = player_y.saturating_sub(self.height / 2);
    }
}

struct World {
    height: u16,
    width: u16,
    objects: Vec<Object>,
    camera: Camera,
}

impl World {
    fn new(height: u16, width: u16) -> World {
        World {
            height,
            width,
            objects: Vec::new(),
            camera: Camera::new(width, height),
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
                .get(tile.sprite_name.as_str())
                .cloned()
                .unwrap_or(ObjectType::Block(BlockType::Block)); // Default to Block(Wall)

            let object = Object::new(
                tile.start_x as u16,
                tile.start_y as u16,
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
        let player = Object::new_player(48, 176, 2, player_sprite);
        self.insert_player(player);
    }
    fn add_object(&mut self, object: Object) {
        self.objects.push(object);
    }
    fn insert_player(&mut self, player: Object) {
        self.objects.insert(0, player);
    }
    fn handle_input(&mut self) {
        if let Some(player) = self.objects.first_mut() {
            if is_key_down(KeyCode::A) {
                player.x = player.x.saturating_sub(1);
            } else if is_key_down(KeyCode::D) {
                player.x = player.x.saturating_add(1);
            }
        }
    }
    fn update(&mut self) {
        for object in &mut self.objects {
            object.update();
        }
        if let Some(player) = self.objects.first() {
            self.camera.update(player.x, player.y);
        }
    }

    fn draw(&self) {
        let (player, other_objects) = self.objects.split_at(1);
        for object in other_objects {
            object.draw(self.camera.x, self.camera.y);
        }
        if let Some(player) = player.first() {
            player.draw(self.camera.x, self.camera.y);
        }
    }
}

#[macroquad::main("Rustario Bros")]
async fn main() {
    preparation::main();
    println!("Finished preparing level data");

    let mut world = World::new(MARIO_WORLD_SIZE.height, MARIO_WORLD_SIZE.width);
    world.load_player().await;
    world.load_level().await;

    println!("Finished loading level");
    request_new_screen_size(world.width as f32, world.height as f32);

    loop {
        clear_background(BLACK);
        world.handle_input();
        world.update();
        world.draw();
        next_frame().await;
    }
}
