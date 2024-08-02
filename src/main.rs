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
const PHYSICS_FRAME_PER_SECOND: f32 = 60.0;
const PHYSICS_FRAME_TIME: f32 = 1.0 / PHYSICS_FRAME_PER_SECOND;
const DIRECTIONS: [(isize, isize); 8] = [
    (-1, 0),
    (0, -1),
    (0, 1),
    (1, 0),
    (1, 1),
    (-1, -1),
    (1, -1),
    (-1, 1),
];

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
    sprite: Texture2D,
    object_type: ObjectType,
}

impl Object {
    fn new(x: usize, y: usize, sprite: Texture2D, object_type: ObjectType) -> Object {
        Object {
            pos: Vec2::new(x as f32, y as f32),
            height: sprite.height() as usize,
            width: sprite.width() as usize,
            sprite,
            object_type,
        }
    }

    fn draw(&self, camera_x: usize, camera_y: usize) {
        if self.pos.x < camera_x as f32 - 32.0 || self.pos.y < camera_y as f32 {
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

struct Player {
    object: Object,
    max_speed: i32,
    velocity: Vec2,
    is_grounded: bool,
}

impl Player {
    fn new(x: usize, y: usize, max_speed: i32, sprite: Texture2D) -> Player {
        Player {
            object: Object::new(x, y, sprite, ObjectType::Player),
            max_speed,
            velocity: Vec2::new(0.0, 0.0),
            is_grounded: false,
        }
    }

    fn apply_gravity(&mut self) {
        self.velocity.y += GRAVITY as f32 * PHYSICS_FRAME_TIME;
    }
    fn apply_x_axis_friction(&mut self) {
        self.velocity.x =
            (self.velocity.x.abs() - 2.0 * PHYSICS_FRAME_TIME) * self.velocity.x.signum();
    }
    fn update(&mut self, surrounding_objects: Vec<Object>) {
        println!(
            "Surrounding object types {:?}",
            surrounding_objects
                .iter()
                .map(|obj| obj.object_type)
                .collect::<Vec<_>>()
        );
        let self_center_x = self.object.pos.x + self.object.width as f32 / 2.0;
        let block_below = surrounding_objects.iter().find(|obj| {
            obj.pos.y > self.object.pos.y
                && obj.pos.x < self_center_x
                && obj.pos.x + obj.width as f32 > self_center_x
                && matches!(
                    obj.object_type,
                    ObjectType::Block(BlockType::Block)
                        | ObjectType::Block(BlockType::PowerupBlock)
                        | ObjectType::Block(BlockType::MovementBlock)
                )
        });

        if block_below.is_none() {
            self.apply_gravity();
            self.is_grounded = false;
        } else {
            self.is_grounded = true;
            self.apply_x_axis_friction();
        }
        self.object.pos += self.velocity;
        for object in surrounding_objects.iter() {
            if object.object_type == ObjectType::Block(BlockType::Block)
                || object.object_type == ObjectType::Block(BlockType::PowerupBlock)
            {
                self.check_and_handle_collision(object);
            }
        }
    }
    fn check_and_handle_collision(&mut self, other: &Object) {
        let self_center = Vec2::new(
            self.object.pos.x + self.object.width as f32 / 2.0,
            self.object.pos.y + self.object.height as f32 / 2.0,
        );
        let other_center = Vec2::new(
            other.pos.x + other.width as f32 / 2.0,
            other.pos.y + other.height as f32 / 2.0,
        );
        let x_overlap = (self.object.width as f32 + other.width as f32) / 2.0
            - (self_center.x - other_center.x).abs();
        let y_overlap = (self.object.height as f32 + other.height as f32) / 2.0
            - (self_center.y - other_center.y).abs();

        if x_overlap > 0.0 && y_overlap > 0.0 {
            let y_collision_threshold = 0.2;
            if y_overlap < self.object.height as f32 * y_collision_threshold {
                if self_center.y < other_center.y {
                    self.object.pos.y -= y_overlap;
                    self.velocity.y = 0.0;
                } else {
                    self.object.pos.y += y_overlap;
                    self.velocity.y = 0.0;
                }
            } else {
                if x_overlap < y_overlap {
                    if self_center.x < other_center.x {
                        self.object.pos.x -= x_overlap;
                        self.velocity.x = 0.0;
                    } else {
                        self.object.pos.x += x_overlap;
                        self.velocity.x = 0.0;
                    }
                } else {
                    if self_center.y < other_center.y {
                        self.object.pos.y -= y_overlap;
                        self.velocity.y = 0.0;
                    } else {
                        self.object.pos.y += y_overlap;
                        self.velocity.y = 0.0;
                    }
                }
            }
        }
    }
    fn add_horizontal_velocity(&mut self, velocity: f32) {
        self.velocity.x += velocity;
        self.velocity.x = self
            .velocity
            .x
            .clamp(-self.max_speed as f32, self.max_speed as f32);
    }

    fn add_vertical_velocity(&mut self, velocity: f32) {
        if self.is_grounded {
            self.velocity.y = -3.0;
            self.is_grounded = false;
        }
        self.velocity.y += velocity;
    }

    fn draw(&self, camera_x: usize, camera_y: usize) {
        self.object.draw(camera_x, camera_y);
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

struct World {
    height: usize,
    width: usize,
    objects: Vec<Vec<Vec<Object>>>,
    player: Player,
    camera: Camera,
    game_over: bool,
}

impl World {
    fn new(height: usize, width: usize) -> World {
        let objects = vec![vec![vec![]; (width / 16) as usize]; (height / 16) as usize];
        World {
            height,
            width,
            objects,
            player: Player::new(48, 176, 25, Texture2D::empty()), // Temporary empty texture
            camera: Camera::new(600, height),
            game_over: false,
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
                .unwrap_or(ObjectType::Block(BlockType::Background));

            let object = Object::new(
                tile.start_x as usize,
                tile.start_y as usize,
                sprite,
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
        self.player = Player::new(48, 176, 40, player_sprite);
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
        const ACCELERATION: f32 = 3.0;
        const JUMP_STRENGTH: f32 = 12.0;
        if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
            self.player
                .add_horizontal_velocity(ACCELERATION * PHYSICS_FRAME_TIME);
        }
        if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
            self.player
                .add_horizontal_velocity(-ACCELERATION * PHYSICS_FRAME_TIME);
        }
        if is_key_down(KeyCode::Space) {
            self.player
                .add_vertical_velocity(-JUMP_STRENGTH * PHYSICS_FRAME_TIME);
        }
    }

    fn update(&mut self) {
        let surrounding_objects: Vec<_> = DIRECTIONS
            .iter()
            .filter_map(|(dy, dx)| {
                let new_y = (self.player.object.pos.y / 16.0).round() as isize + *dy;
                let new_x = (self.player.object.pos.x / 16.0).round() as isize + *dx;

                if new_y >= 0
                    && new_y < self.objects.len() as isize
                    && new_x >= 0
                    && new_x < self.objects[0].len() as isize
                {
                    Some(self.objects[new_y as usize][new_x as usize].clone())
                } else {
                    None
                }
            })
            .flatten()
            .collect();

        self.player.update(surrounding_objects);
        self.camera.update(
            self.player.object.pos.x as usize,
            self.player.object.pos.y as usize,
        );
        self.check_player_in_bounds_or_game_over();
    }
    fn check_player_in_bounds_or_game_over(&mut self) {
        if self.player.object.pos.y > self.height as f32 {
            self.game_over = true;
        }
        if self.player.object.pos.x > self.width as f32 {
            self.game_over = true;
        }
        if self.player.object.pos.x < 0.0 {
            self.player.object.pos.x = 0.0;
        }
    }
    fn draw(&self) {
        if self.game_over {
            clear_background(BLACK);
            draw_text("Game Over", 100.0, 100.0, 30.0, RED);
            return;
        }
        for row in self.objects.iter() {
            for cell in row.iter() {
                for object in cell.iter() {
                    object.draw(self.camera.x, self.camera.y);
                }
            }
        }
        self.player.draw(self.camera.x, self.camera.y);
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Rustario Bros".to_owned(),
        window_width: 600,
        window_height: 400,
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

    let mut elapsed_time = 0.0;
    let target_time_step = 1.0 / PHYSICS_FRAME_PER_SECOND;

    loop {
        clear_background(BLACK);
        elapsed_time += get_frame_time();
        while elapsed_time >= target_time_step {
            world.handle_input();
            world.update();
            elapsed_time -= target_time_step;
        }
        world.draw();
        next_frame().await;
    }
}
