use macroquad::prelude::*;
use mario_config::mario_config::{
    ACCELERATION, CAMERA_HEIGHT_IN_TILES, CAMERA_WIDTH, CAMERA_WIDTH_IN_TILES, GRAVITY,
    JUMP_STRENGTH, MARIO_SPRITE_BLOCK_SIZE, MARIO_WORLD_SIZE, MAX_VELOCITY_X,
    PHYSICS_FRAME_PER_SECOND, PHYSICS_FRAME_TIME, WORLD_HEIGHT_IN_TILES, WORLD_WIDTH_IN_TILES,
};
use preparation::LevelData;
use shader::shader::SPRITE_FRAGMENT_SHADER;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
pub mod image_utils;
pub mod mario_config;
pub mod preparation;
pub mod shader;
use lazy_static::lazy_static;
const DEFAULT_VERTEX_SHADER: &'static str = "#version 100
precision lowp float;

attribute vec3 position;
attribute vec2 texcoord;

varying vec2 uv;

void main() {
    gl_Position =  vec4(position, 1);
    uv = texcoord;
}
";
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
    static ref SPRITE_TYPE_MAPPING: HashMap<&'static u8, ObjectType> = {
        let mut m = HashMap::new();
        m.insert(&9, ObjectType::Block(BlockType::PowerupBlock));
        m.insert(&10, ObjectType::Block(BlockType::Block));
        m.insert(&11, ObjectType::Block(BlockType::Block));
        m.insert(&12, ObjectType::Block(BlockType::Block));
        m.insert(&13, ObjectType::Block(BlockType::Block));
        m.insert(&14, ObjectType::Block(BlockType::Block));
        m.insert(&15, ObjectType::Block(BlockType::Block));
        m.insert(&16, ObjectType::Block(BlockType::Block));
        m.insert(&17, ObjectType::Block(BlockType::Block));
        m.insert(&19, ObjectType::Block(BlockType::Block));
        m.insert(&20, ObjectType::Block(BlockType::Block));
        m.insert(&21, ObjectType::Block(BlockType::Block));
        m.insert(&25, ObjectType::Block(BlockType::Block));
        m.insert(&31, ObjectType::Block(BlockType::Block));
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
    object_type: ObjectType,
}

impl Object {
    fn new(x: usize, y: usize, object_type: ObjectType) -> Object {
        Object {
            pos: Vec2::new(x as f32, y as f32),
            height: MARIO_SPRITE_BLOCK_SIZE,
            width: MARIO_SPRITE_BLOCK_SIZE,
            object_type,
        }
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
    sprite: Texture2D,
}

impl Player {
    fn new(x: usize, y: usize, max_speed: i32, sprite: Texture2D) -> Player {
        Player {
            object: Object::new(x, y, ObjectType::Player),
            max_speed,
            velocity: Vec2::new(0.0, 0.0),
            is_grounded: false,
            sprite,
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
        if self.velocity.y > 0.0 {
            // if falling by gravity dont allow for slow falling
            return;
        }
        self.velocity.y += velocity;
    }

    fn draw(&self, camera_x: usize, camera_y: usize) {
        draw_texture_ex(
            &self.sprite,
            self.object.pos.x - camera_x as f32,
            self.object.pos.y - camera_y as f32,
            WHITE,
            DrawTextureParams {
                dest_size: Some(Vec2::new(
                    self.object.width as f32,
                    self.object.height as f32,
                )),
                ..Default::default()
            },
        );
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
enum GameState {
    Playing,
    GameWon,
    GameOver,
}
struct World {
    height: usize,
    width: usize,
    objects: Vec<Vec<Vec<Object>>>,
    player: Player,
    camera: Camera,
    game_state: GameState,
    index_buffer: Option<Vec<u8>>,
    material: Option<Material>,
}

impl World {
    fn new(height: usize, width: usize) -> World {
        let objects = vec![vec![vec![]; (width / 16) as usize]; (height / 16) as usize];
        World {
            height,
            width,
            objects,
            player: Player::new(48, 176, 6, Texture2D::empty()), // Temporary empty texture
            camera: Camera::new(600, height),
            game_state: GameState::Playing,
            index_buffer: None,
            material: None,
        }
    }

    async fn load_level(&mut self) {
        let mut level_data_file =
            File::open("leveldata/level_data.json").expect("Failed to open level data file");
        let mut level_data_string = String::new();
        level_data_file
            .read_to_string(&mut level_data_string)
            .expect("Failed to read level data file");

        let level_data: LevelData =
            serde_json::from_str(&level_data_string).expect("Failed to parse level data");

        let mut index_buffer = vec![0; (WORLD_WIDTH_IN_TILES * WORLD_HEIGHT_IN_TILES) as usize];

        {
            for (index, tile) in level_data.tiles.iter().enumerate() {
                let x =
                    (index as u32 % (WORLD_WIDTH_IN_TILES) as u32) * MARIO_SPRITE_BLOCK_SIZE as u32;
                let y =
                    (index as u32 / (WORLD_WIDTH_IN_TILES) as u32) * MARIO_SPRITE_BLOCK_SIZE as u32;
                index_buffer[index] = *tile;

                self.add_object(Object::new(
                    x as usize,
                    y as usize,
                    SPRITE_TYPE_MAPPING
                        .get(&tile)
                        .unwrap_or_else(|| &ObjectType::Block(BlockType::Background))
                        .clone(),
                ));
            }
        }
        self.index_buffer = Some(index_buffer);
    }

    async fn load_player(&mut self) {
        let player_sprite = load_texture("sprites/Mario.png")
            .await
            .expect("Failed to load player sprite");
        let mut player_sprite = player_sprite.get_texture_data();
        image_utils::convert_white_to_transparent(&mut player_sprite);
        let player_sprite = Texture2D::from_image(&player_sprite);
        self.player = Player::new(48, 176, MAX_VELOCITY_X, player_sprite);
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
    fn get_viewport_adjusted_index_rgba_buffer(&self) -> Vec<u8> {
        let mut result =
            Vec::with_capacity((CAMERA_WIDTH_IN_TILES * CAMERA_HEIGHT_IN_TILES / 4) as usize);

        let index_buffer = self
            .index_buffer
            .as_ref()
            .expect("Index buffer not initialized");
        let start_x = self.camera.x / MARIO_SPRITE_BLOCK_SIZE;
        let start_y = self.camera.y / MARIO_SPRITE_BLOCK_SIZE;
        let end_x = start_x + CAMERA_WIDTH_IN_TILES;
        let end_y = start_y + CAMERA_HEIGHT_IN_TILES;

        for y in (start_y..end_y).step_by(2) {
            for x in (start_x..end_x).step_by(2) {
                let index_top_left = (y * WORLD_WIDTH_IN_TILES + x) as usize;
                let index_top_right = index_top_left + 1;
                let index_bottom_left = index_top_left + WORLD_WIDTH_IN_TILES;
                let index_bottom_right = index_bottom_left + 1;
                result.push(index_buffer[index_top_left]);
                result.push(if x + 1 < end_x {
                    index_buffer[index_top_right]
                } else {
                    0
                });
                result.push(if y + 1 < end_y {
                    index_buffer[index_bottom_left]
                } else {
                    0
                });
                result.push(if x + 1 < end_x && y + 1 < end_y {
                    index_buffer[index_bottom_right]
                } else {
                    0
                });
            }
        }

        result
    }
    async fn init_shader(&mut self) {
        let fragment_shader = SPRITE_FRAGMENT_SHADER;
        let tilesheet = load_texture("sprites/tilesheet.png")
            .await
            .expect("Failed to load tilesheet");
        let tilesheet_width_height = (tilesheet.width(), tilesheet.height());
        let material = load_material(
            ShaderSource::Glsl {
                vertex: DEFAULT_VERTEX_SHADER,
                fragment: fragment_shader,
            },
            MaterialParams {
                uniforms: vec![
                    ("canvasSize".to_string(), UniformType::Float2),
                    ("spriteSheetSize".to_string(), UniformType::Float2),
                    ("spriteSize".to_string(), UniformType::Float1),
                ],
                textures: vec![("indexTexture".to_string()), ("spriteSheet".to_string())],
                ..Default::default()
            },
        )
        .expect("Failed to load material");
        material.set_texture("spriteSheet", tilesheet);

        material.set_uniform(
            "canvasSize",
            vec2(self.camera.width as f32, self.camera.height as f32),
        );

        material.set_uniform(
            "spriteSheetSize",
            vec2(
                tilesheet_width_height.0 / 16.0,
                tilesheet_width_height.1 / 16.0,
            ),
        );
        material.set_uniform("spriteSize", 16.0);

        self.material = Some(material);
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
            self.game_state = GameState::GameOver;
        }
        if self.player.object.pos.x > self.width as f32 {
            self.game_state = GameState::GameWon;
        }
        if self.player.object.pos.x < 0.0 {
            self.player.object.pos.x = 0.0;
        }
    }
    fn draw(&self) {
        match self.game_state {
            GameState::GameOver => {
                draw_text("Game Over", 200.0, 200.0, 20.0, RED);
            }
            GameState::GameWon => {
                draw_text("You Won!", 200.0, 200.0, 20.0, GREEN);
            }
            GameState::Playing => {
                if let Some(material) = &self.material {
                    let packed_buffer = self.get_viewport_adjusted_index_rgba_buffer();
                    material.set_texture(
                        "indexTexture",
                        Texture2D::from_rgba8(packed_buffer.len() as u16 / 4, 1, &packed_buffer),
                    );
                    gl_use_material(material);
                    draw_texture_ex(
                        &Texture2D::from_rgba8(packed_buffer.len() as u16 / 4, 1, &packed_buffer),
                        0.0,
                        0.0,
                        WHITE,
                        DrawTextureParams {
                            ..Default::default()
                        },
                    );
                }
                gl_use_default_material();
                self.player.draw(self.camera.x, self.camera.y);
            }
        }
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
    world.init_shader().await;
    println!("Finished loading level");
    request_new_screen_size(CAMERA_WIDTH as f32, world.height as f32);

    let mut elapsed_time = 0.0;
    let target_time_step = 1.0 / PHYSICS_FRAME_PER_SECOND;

    loop {
        clear_background(BLACK);
        let fps = get_fps();
        elapsed_time += get_frame_time();

        while elapsed_time >= target_time_step {
            world.handle_input();
            world.update();
            elapsed_time -= target_time_step;
        }

        world.draw();

        draw_text(&format!("FPS: {}", fps), 15.0, 15.0, 20.0, RED);

        next_frame().await;
    }
}
