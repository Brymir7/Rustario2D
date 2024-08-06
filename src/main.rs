use macroquad::prelude::*;
use mario_config::mario_config::{
    ACCELERATION, GRAVITY, JUMP_STRENGTH, MARIO_SPRITE_BLOCK_SIZE, MARIO_WORLD_SIZE,
    MAX_VELOCITY_X, PHYSICS_FRAME_PER_SECOND, PHYSICS_FRAME_TIME, SCALE_IMAGE_FACTOR,
};
use preparation::LevelData;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
pub mod image_utils;
pub mod mario_config;
pub mod preparation;
use lazy_static::lazy_static;

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
    static ref SPRITE_TYPE_MAPPING: HashMap<&'static usize, ObjectType> = {
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
    static ref MAP_MOVEMENT_STATE_TO_TEXTURE2D: HashMap<PlayerMovementState, Vec<Texture2D>> = {
        let mut m: HashMap<PlayerMovementState, Vec<Texture2D>> = HashMap::new();

        fn load_and_convert_texture(data: &[u8], format: ImageFormat) -> Texture2D {
            let texture = Texture2D::from_file_with_format(data, Some(format));
            let mut texture_data = texture.get_texture_data();
            image_utils::convert_white_to_transparent(&mut texture_data);
            texture.update(&texture_data);
            texture
        }

        m.insert(
            PlayerMovementState::Idle,
            vec![load_and_convert_texture(
                include_bytes!("../sprites/Mario.png"),
                ImageFormat::Png,
            )],
        );
        m.insert(
            PlayerMovementState::Running,
            vec![
                load_and_convert_texture(
                    include_bytes!("../sprites/Mario_Run1.png"),
                    ImageFormat::Png,
                ),
                load_and_convert_texture(
                    include_bytes!("../sprites/Mario_Run2.png"),
                    ImageFormat::Png,
                ),
            ],
        );
        m.insert(
            PlayerMovementState::Turning,
            vec![load_and_convert_texture(
                include_bytes!("../sprites/Mario_Turn.png"),
                ImageFormat::Png,
            )],
        );
        m.insert(
            PlayerMovementState::Jumping,
            vec![
                load_and_convert_texture(
                    include_bytes!("../sprites/Mario_Jump1.png"),
                    ImageFormat::Png,
                ),
                load_and_convert_texture(
                    include_bytes!("../sprites/Mario_Jump2.png"),
                    ImageFormat::Png,
                ),
            ],
        );
        m.insert(
            PlayerMovementState::RunningJump,
            vec![load_and_convert_texture(
                include_bytes!("../sprites/Mario_Jump_HMomentum.png"),
                ImageFormat::Png,
            )],
        );

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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum PlayerMovementState {
    Idle,
    Running,
    Turning,
    Jumping, // Jumping and falling
    RunningJump,
}

enum PlayerPowerupState {
    Small,
    Big,
}
struct Player {
    object: Object,
    max_speed: i32,
    velocity: Vec2,
    is_grounded: bool,
    powerup_state: PlayerPowerupState,
    movement_state: PlayerMovementState, // unnecessary, could just do it in the draw function
    animation_frame: u8,
}

impl Player {
    fn new(x: usize, y: usize, max_speed: i32) -> Player {
        Player {
            object: Object::new(x, y, ObjectType::Player),
            max_speed,
            velocity: Vec2::new(0.0, 0.0),
            is_grounded: false,
            powerup_state: PlayerPowerupState::Small,
            movement_state: PlayerMovementState::Idle,
            animation_frame: 0,
        }
    }

    fn apply_gravity(&mut self) {
        self.velocity.y += GRAVITY as f32 * PHYSICS_FRAME_TIME;
    }
    fn apply_x_axis_friction(&mut self) {
        self.velocity.x =
            (self.velocity.x.abs() - 2.0 * PHYSICS_FRAME_TIME) * self.velocity.x.signum();
    }
    fn update_player_movement_state(&mut self) {
        let previous_state = self.movement_state;

        if self.velocity.x.abs() > 0.1 {
            if self.velocity.y.abs() > 0.0 {
                self.movement_state = PlayerMovementState::RunningJump;
            } else {
                self.movement_state = PlayerMovementState::Running;
            }
        } else if self.velocity.y.abs() > 0.0 {
            self.movement_state = PlayerMovementState::Jumping;
        } else {
            self.movement_state = PlayerMovementState::Idle;
        }

        if self.movement_state != previous_state {
            self.animation_frame = 0;
        } else {
            self.animation_frame += 1;
        }
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
        self.update_player_movement_state();
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

    fn draw(&mut self, camera_x: usize, camera_y: usize) {
        // implement animation + draw state using another type (struct) and this implements it then in object
        let sprites_to_draw = MAP_MOVEMENT_STATE_TO_TEXTURE2D
            .get(&self.movement_state)
            .expect("Failed to get sprite to draw");
        if self.animation_frame >= sprites_to_draw.len() as u8 {
            self.animation_frame = 0;
        }
        draw_texture_ex(
            &sprites_to_draw[self.animation_frame as usize],
            self.object.pos.x - camera_x as f32,
            self.object.pos.y - camera_y as f32,
            WHITE,
            DrawTextureParams {
                dest_size: Some(Vec2::new(
                    self.object.width as f32 * SCALE_IMAGE_FACTOR,
                    self.object.height as f32 * SCALE_IMAGE_FACTOR,
                )),
                flip_x: self.velocity.x < -0.1,
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
    level_texture: Option<Texture2D>,
}

impl World {
    fn new(height: usize, width: usize) -> World {
        let objects = vec![vec![vec![]; (width / 16) as usize]; (height / 16) as usize];
        World {
            height,
            width,
            objects,
            player: Player::new(48, 176, 6),
            camera: Camera::new(600, height),
            game_state: GameState::Playing,
            level_texture: None,
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

        let tilesheet = load_texture("sprites/tilesheet.png")
            .await
            .expect("Failed to load tilesheet");
        let mut render_target_camera =
            Camera2D::from_display_rect(Rect::new(0., 0., self.width as f32, self.height as f32));

        let level_render_target = render_target(self.width as u32, self.height as u32);
        render_target_camera.render_target = Some(level_render_target);
        {
            set_camera(&render_target_camera);
            for (index, tile) in level_data.tiles.iter().enumerate() {
                let x = (index as u32 % (self.width / MARIO_SPRITE_BLOCK_SIZE as usize) as u32)
                    * MARIO_SPRITE_BLOCK_SIZE as u32;
                let y = (index as u32 / (self.width / MARIO_SPRITE_BLOCK_SIZE as usize) as u32)
                    * MARIO_SPRITE_BLOCK_SIZE as u32;

                let sprite_y = (*tile as u32 * MARIO_SPRITE_BLOCK_SIZE as u32) as f32;

                draw_texture_ex(
                    &tilesheet,
                    x as f32,
                    y as f32,
                    WHITE,
                    DrawTextureParams {
                        source: Some(Rect {
                            x: 0.0,
                            y: sprite_y,
                            w: MARIO_SPRITE_BLOCK_SIZE as f32,
                            h: MARIO_SPRITE_BLOCK_SIZE as f32,
                        }),
                        dest_size: Some(Vec2::new(
                            MARIO_SPRITE_BLOCK_SIZE as f32 * SCALE_IMAGE_FACTOR,
                            MARIO_SPRITE_BLOCK_SIZE as f32 * SCALE_IMAGE_FACTOR,
                        )),
                        ..Default::default()
                    },
                );
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
        set_default_camera();
        let render_texture = render_target_camera.render_target.unwrap().texture;
        self.level_texture = Some(render_texture);
    }

    async fn load_player(&mut self) {
        self.player = Player::new(48, 176, MAX_VELOCITY_X);
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
    fn draw(&mut self) {
        match self.game_state {
            GameState::GameOver => {
                draw_text("Game Over", 200.0, 200.0, 40.0, RED);
            }
            GameState::GameWon => {
                draw_text("You Won!", 200.0, 200.0, 40.0, GREEN);
            }
            _ => {
                if let Some(level_texture) = &self.level_texture {
                    draw_texture_ex(
                        level_texture,
                        0.0,
                        0.0,
                        WHITE,
                        DrawTextureParams {
                            source: Some(Rect::new(
                                self.camera.x as f32,
                                self.camera.y as f32,
                                self.camera.width as f32,
                                self.camera.height as f32,
                            )),
                            dest_size: Some(Vec2::new(
                                self.camera.width as f32 * SCALE_IMAGE_FACTOR,
                                self.camera.height as f32 * SCALE_IMAGE_FACTOR,
                            )),
                            flip_y: true,
                            ..Default::default()
                        },
                    );
                }
                self.player.draw(self.camera.x, self.camera.y);
            }
        }
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Rustario Bros".to_owned(),
        window_width: 600 * SCALE_IMAGE_FACTOR as i32,
        window_height: MARIO_WORLD_SIZE.height as i32 * SCALE_IMAGE_FACTOR as i32,
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
