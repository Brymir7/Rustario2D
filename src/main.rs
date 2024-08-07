use image_utils::load_and_convert_texture;
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
fn resolve_collision_general(object: &Object, velocity: &Vec2, other: &Object) -> (Vec2, Vec2) {
    let self_center = Vec2::new(
        object.pos.x + object.width as f32 / 2.0,
        object.pos.y + object.height as f32 / 2.0,
    );
    let other_center = Vec2::new(
        other.pos.x + other.width as f32 / 2.0,
        other.pos.y + other.height as f32 / 2.0,
    );
    let x_overlap =
        (object.width as f32 + other.width as f32) / 2.0 - (self_center.x - other_center.x).abs();
    let y_overlap =
        (object.height as f32 + other.height as f32) / 2.0 - (self_center.y - other_center.y).abs();

    let mut new_pos = object.pos;
    let mut new_velocity = *velocity;

    if x_overlap > 0.0 && y_overlap > 0.0 {
        let y_collision_threshold = 0.2;
        if y_overlap < object.height as f32 * y_collision_threshold {
            if self_center.y < other_center.y {
                new_pos.y -= y_overlap;
                new_velocity.y = 0.0;
            } else {
                new_pos.y += y_overlap;
                new_velocity.y = 0.0;
            }
        } else {
            if x_overlap < y_overlap {
                if self_center.x < other_center.x {
                    new_pos.x -= x_overlap;
                    new_velocity.x = 0.0;
                } else {
                    new_pos.x += x_overlap;
                    new_velocity.x = 0.0;
                }
            } else {
                if self_center.y < other_center.y {
                    new_pos.y -= y_overlap;
                    new_velocity.y = 0.0;
                } else {
                    new_pos.y += y_overlap;
                    new_velocity.y = 0.0;
                }
            }
        }
    }

    (new_pos, new_velocity)
}
trait CollisionHandler {
    fn resolve_collision(&self, object: &Object, velocity: &Vec2, other: &Object) -> (Vec2, Vec2);
}
struct DoNothingCollisionHandler;
impl CollisionHandler for DoNothingCollisionHandler {
    fn resolve_collision(&self, object: &Object, velocity: &Vec2, other: &Object) -> (Vec2, Vec2) {
        (
            Vec2::new(object.pos.x, object.pos.y),
            Vec2::new(velocity.x, velocity.y),
        )
    }
}
struct BlockCollisionHandler;
impl CollisionHandler for BlockCollisionHandler {
    fn resolve_collision(&self, object: &Object, velocity: &Vec2, other: &Object) -> (Vec2, Vec2) {
        let (new_pos, new_velocity) = resolve_collision_general(object, velocity, other);
        (new_pos, new_velocity)
    }
}
struct EnemyCollisionHandler;
impl CollisionHandler for EnemyCollisionHandler {
    fn resolve_collision(&self, object: &Object, velocity: &Vec2, other: &Object) -> (Vec2, Vec2) {
        let (new_pos, new_velocity) = resolve_collision_general(object, velocity, other);
        let new_velocity = Vec2::new(-new_velocity.x, new_velocity.y); // reverse direction, typical mario goomba | goomba collision
        (new_pos, new_velocity)
    }
}
struct EnemyBlockCollisionHandler;
impl CollisionHandler for EnemyBlockCollisionHandler {
    fn resolve_collision(&self, object: &Object, velocity: &Vec2, other: &Object) -> (Vec2, Vec2) {
        let (new_pos, new_velocity) = resolve_collision_general(object, velocity, other);
        if other.pos.y / 16.0 == object.pos.y / 16.0 {
            // if goomba is on the same level as block, reverse direction
            let new_pos = Vec2::new(new_pos.x - 2.0, new_pos.y); // move goomba back a bit, otherwise it will get stuck
            return (new_pos, Vec2::new(-velocity.x, velocity.y));
        }
        (new_pos, new_velocity)
    }
}
struct PlayerEnemyCollisionHandler;
impl CollisionHandler for PlayerEnemyCollisionHandler {
    fn resolve_collision(&self, object: &Object, velocity: &Vec2, other: &Object) -> (Vec2, Vec2) {
        let (new_pos, new_velocity) = resolve_collision_general(object, velocity, other);
        (new_pos, new_velocity)
    }
}
trait Updatable {
    fn mut_object(&mut self) -> &mut Object;
    fn mut_velocity(&mut self) -> &mut Vec2;
    fn object(&self) -> &Object;
    fn velocity(&self) -> &Vec2;

    fn set_grounded(&mut self, grounded: bool);
    fn animate(&mut self) -> &mut Animate;

    fn apply_gravity(&mut self) {
        self.mut_velocity().y += GRAVITY as f32 * PHYSICS_FRAME_TIME;
    }

    fn apply_x_axis_friction(&mut self) {
        self.mut_velocity().x =
            (self.velocity().x.abs() - 2.0 * PHYSICS_FRAME_TIME) * self.velocity().x.signum();
    }
    fn update_animation(&mut self) {}
    fn get_collision_handler(&self, object_type: ObjectType) -> Box<dyn CollisionHandler>; // this could be a trait enum?

    fn update(&mut self, surrounding_objects: Vec<Object>) {
        let self_center_x = self.object().pos.x + self.object().width as f32 / 2.0;
        let block_below = surrounding_objects.iter().find(|obj| {
            obj.pos.y > self.object().pos.y
                && obj.pos.x < self_center_x
                && obj.pos.x + obj.width as f32 > self_center_x
                && matches!(
                    obj.object_type,
                    ObjectType::Block(BlockType::Block)
                        | ObjectType::Block(BlockType::PowerupBlock)
                )
        });

        if block_below.is_none() {
            self.apply_gravity();
            self.set_grounded(false);
        } else {
            self.set_grounded(true);
            self.apply_x_axis_friction();
        }
        let velocity = self.velocity().clone();
        self.mut_object().pos += velocity;

        for other in surrounding_objects.iter() {
            if other.object_type == ObjectType::Block(BlockType::Block)
                || other.object_type == ObjectType::Block(BlockType::PowerupBlock)
            {
                let collision_handler = self.get_collision_handler(other.object_type);
                let (new_pos, new_velocity) =
                    collision_handler.resolve_collision(self.object(), self.velocity(), other);
                self.mut_object().pos = new_pos;
                *self.mut_velocity() = new_velocity;
            }
        }
        self.update_animation();
        self.animate().update();
    }
}

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
    static ref MARIO_SPRITE_LOOKUP: [Texture2D; 6] = [
        load_and_convert_texture(include_bytes!("../sprites/Mario.png"), ImageFormat::Png),
        load_and_convert_texture(
            include_bytes!("../sprites/Mario_Run1.png"),
            ImageFormat::Png
        ),
        load_and_convert_texture(
            include_bytes!("../sprites/Mario_Run2.png"),
            ImageFormat::Png
        ),
        load_and_convert_texture(
            include_bytes!("../sprites/Mario_Jump1.png"),
            ImageFormat::Png
        ),
        load_and_convert_texture(
            include_bytes!("../sprites/Mario_Turn.png"),
            ImageFormat::Png
        ),
        load_and_convert_texture(
            include_bytes!("../sprites/Mario_Jump_HMomentum.png"),
            ImageFormat::Png
        ),
    ];
    static ref GOOMBA_SPRITE_LOOKUP: [Texture2D; 3] = [
        load_and_convert_texture(include_bytes!("../sprites/Goomba1.png"), ImageFormat::Png),
        load_and_convert_texture(include_bytes!("../sprites/Goomba2.png"), ImageFormat::Png),
        load_and_convert_texture(include_bytes!("../sprites/Goomba3.png"), ImageFormat::Png),
    ];
}

#[derive(Clone, PartialEq, Copy, Debug)]
enum BlockType {
    Block,

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
struct Animate {
    frames: Vec<Texture2D>,
    current_frame: usize,
    speed_factor: f32,
    time_to_change: f32,
    time_elapsed: f32,
}

impl Animate {
    fn new(speed_factor: f32) -> Self {
        assert!(speed_factor > 0.0);
        Animate {
            frames: Vec::new(),
            current_frame: 0,
            speed_factor,
            time_to_change: (PHYSICS_FRAME_TIME * 3.0) * speed_factor, // 10 frames per sprite
            time_elapsed: 0.0,
        }
    }

    fn change_animation(&mut self, new_frames: Vec<Texture2D>) {
        if new_frames != self.frames {
            self.frames = new_frames;
            self.current_frame = 0;
            self.time_elapsed = 0.0;
        }
    }

    fn update(&mut self) {
        if self.frames.len() > 1 {
            self.time_elapsed += PHYSICS_FRAME_TIME;
            if self.time_elapsed >= self.time_to_change {
                self.current_frame = (self.current_frame + 1) % self.frames.len();
                self.time_elapsed -= self.time_to_change;
            }
        }
    }
    fn scale_animation_speed(&mut self, factor: f32) {
        assert!(factor > 0.0);
        self.speed_factor = factor;
        self.time_to_change = (PHYSICS_FRAME_TIME * 3.0) * (1.0 / self.speed_factor);
    }
    fn current_frame(&self) -> Option<&Texture2D> {
        self.frames.get(self.current_frame)
    }
}

struct Player {
    object: Object,
    max_speed: i32,
    velocity: Vec2,
    is_grounded: bool,
    powerup_state: PlayerPowerupState,
    animate: Animate,
}
impl Updatable for Player {
    fn mut_object(&mut self) -> &mut Object {
        &mut self.object
    }

    fn mut_velocity(&mut self) -> &mut Vec2 {
        &mut self.velocity
    }

    fn object(&self) -> &Object {
        &self.object
    }

    fn velocity(&self) -> &Vec2 {
        &self.velocity
    }

    fn set_grounded(&mut self, grounded: bool) {
        self.is_grounded = grounded;
    }

    fn animate(&mut self) -> &mut Animate {
        &mut self.animate
    }
    fn get_collision_handler(&self, object_type: ObjectType) -> Box<dyn CollisionHandler> {
        match object_type {
            ObjectType::Block(_) => Box::new(BlockCollisionHandler),
            ObjectType::Enemy(EnemyType::Goomba) => Box::new(EnemyCollisionHandler),
            ObjectType::Player => Box::new(PlayerEnemyCollisionHandler),
            _ => panic!("No collision handler for object type: {:?}", object_type),
        }
    }
    fn update_animation(&mut self) {
        // Use velocity and keyboard input to determine the correct animation frames
        if self.velocity.y < 0.0 {
            if self.velocity.x.abs() > 2.5 && self.velocity.y.abs() > 1.5 {
                // Running Jump
                self.animate
                    .change_animation(vec![MARIO_SPRITE_LOOKUP[5].clone()]);
                return;
            } else {
                // Jumping
                self.animate
                    .change_animation(vec![MARIO_SPRITE_LOOKUP[3].clone()]);
                return;
            }
        } else if self.velocity.x.abs() > 0.1 {
            // Running
            if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
                if self.velocity.x < 0.0 {
                    // Turning
                    self.animate
                        .change_animation(vec![MARIO_SPRITE_LOOKUP[4].clone()]);
                    return;
                }
            } else if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
                if self.velocity.x > 0.0 {
                    // Turning
                    self.animate
                        .change_animation(vec![MARIO_SPRITE_LOOKUP[4].clone()]);
                    return;
                }
            }
            self.animate
                .change_animation(MARIO_SPRITE_LOOKUP[1..3].to_vec());
            self.animate
                .scale_animation_speed(self.velocity.x.abs() / self.max_speed as f32);
        } else {
            // Idle
            self.animate
                .change_animation(vec![MARIO_SPRITE_LOOKUP[0].clone()]);
        }
    }
}

impl Player {
    fn new(x: usize, y: usize, max_speed: i32) -> Player {
        let mut player = Player {
            object: Object::new(x, y, ObjectType::Player),
            max_speed,
            velocity: Vec2::new(0.0, 0.0),
            is_grounded: false,
            powerup_state: PlayerPowerupState::Small,
            animate: Animate::new(1.0),
        };
        player
            .animate
            .change_animation(vec![MARIO_SPRITE_LOOKUP[0].clone()]);
        player
    }

    fn update(&mut self, surrounding_objects: Vec<Object>) {
        Updatable::update(self, surrounding_objects);
    }

    fn add_horizontal_velocity(&mut self, velocity: f32) {
        self.velocity.x += velocity;
        self.velocity.x = self
            .velocity
            .x
            .clamp(-self.max_speed as f32, self.max_speed as f32);
    }

    fn jump(&mut self) {
        const VELOCITY: f32 = -JUMP_STRENGTH * PHYSICS_FRAME_TIME;
        if self.is_grounded {
            self.velocity.y = -3.0;
            self.is_grounded = false;
        }
        if self.velocity.y > 0.0 {
            // if falling by gravity dont allow for slow falling
            return;
        }
        self.velocity.y += VELOCITY;
    }

    fn draw(&self, camera_x: usize, camera_y: usize) {
        // TODO! draw using Animate
        if let Some(sprite_to_draw) = self.animate.current_frame() {
            draw_texture_ex(
                &sprite_to_draw,
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
}
struct Goomba {
    object: Object,
    max_speed: i32,
    velocity: Vec2,
    animate: Animate,
    is_grounded: bool,
}
impl Updatable for Goomba {
    fn mut_object(&mut self) -> &mut Object {
        &mut self.object
    }

    fn mut_velocity(&mut self) -> &mut Vec2 {
        &mut self.velocity
    }

    fn object(&self) -> &Object {
        &self.object
    }

    fn velocity(&self) -> &Vec2 {
        &self.velocity
    }

    fn set_grounded(&mut self, grounded: bool) {
        self.is_grounded = grounded;
    }

    fn animate(&mut self) -> &mut Animate {
        &mut self.animate
    }
    fn get_collision_handler(&self, object_type: ObjectType) -> Box<dyn CollisionHandler> {
        match object_type {
            ObjectType::Block(_) => Box::new(EnemyBlockCollisionHandler),
            ObjectType::Enemy(EnemyType::Goomba) => Box::new(EnemyCollisionHandler),
            ObjectType::Player => Box::new(DoNothingCollisionHandler), // Goomba does not interact with player, player will handle goomba collision
            _ => panic!("No collision handler for object type: {:?}", object_type),
        }
    }
    fn update_animation(&mut self) {
        if self.velocity.x.abs() > 0.1 {
            self.animate.change_animation(GOOMBA_SPRITE_LOOKUP.to_vec());
            self.animate
                .scale_animation_speed(self.velocity.x.abs() / self.max_speed as f32);
        } else {
            self.animate
                .change_animation(vec![GOOMBA_SPRITE_LOOKUP[0].clone()]);
        }
    }
}
impl Goomba {
    fn new(x: usize, y: usize, max_speed: i32) -> Goomba {
        let mut goomba = Goomba {
            object: Object::new(x, y, ObjectType::Enemy(EnemyType::Goomba)),
            max_speed,
            velocity: Vec2::new(1.0, 0.0),
            animate: Animate::new(1.0),
            is_grounded: false,
        };
        goomba
            .animate
            .change_animation(GOOMBA_SPRITE_LOOKUP.to_vec());
        goomba
    }
    fn update(&mut self, surrounding_objects: Vec<Object>) {
        self.velocity.x = 1.0 * self.velocity.x.signum(); // avoid friction atm;
        Updatable::update(self, surrounding_objects);
    }
    fn draw(&self, camera_x: usize, camera_y: usize) {
        // TODO! draw using Animate
        if let Some(sprite_to_draw) = self.animate.current_frame() {
            draw_texture_ex(
                &sprite_to_draw,
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
    enemies: Vec<Goomba>,
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
            enemies: Vec::new(),
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
        self.level_texture = Some(render_texture); // to draw in one call, while keeping compressed json instead of loading a .png
        self.add_object(Object::new(200, 176, ObjectType::Enemy(EnemyType::Goomba)));
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
        let pos = object.pos;
        match object.object_type {
            ObjectType::Enemy(EnemyType::Goomba) => {
                self.enemies
                    .push(Goomba::new(pos.x as usize, pos.y as usize, 2));
            }
            _ => {}
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
            self.player.jump();
        }
    }
    fn get_surrounding_objects(objects: &Vec<Vec<Vec<Object>>>, object: &Object) -> Vec<Object> {
        DIRECTIONS
            .iter()
            .filter_map(|(dy, dx)| {
                let new_y = (object.pos.y / 16.0).round() as isize + *dy;
                let new_x = (object.pos.x / 16.0).round() as isize + *dx;

                if new_y >= 0
                    && new_y < objects.len() as isize
                    && new_x >= 0
                    && new_x < objects[0].len() as isize
                {
                    Some(objects[new_y as usize][new_x as usize].clone())
                } else {
                    None
                }
            })
            .flatten()
            .collect()
    }
    fn update(&mut self) {
        for enemy in &mut self.enemies {
            let surrounding_objects = Self::get_surrounding_objects(&self.objects, &enemy.object);
            let old_x = (enemy.object.pos.x / 16.0).round() as usize;
            let old_y = (enemy.object.pos.y / 16.0).round() as usize;
            enemy.update(surrounding_objects);
            let new_x = (enemy.object.pos.x / 16.0).round() as usize;
            let new_y = (enemy.object.pos.y / 16.0).round() as usize;
            if old_x != new_x || old_y != new_y {
                self.objects[old_y][old_x].retain(|obj| match obj.object_type {
                    ObjectType::Enemy(_) => false,
                    _ => true,
                });
                assert!(
                    new_y < self.objects.len() && new_x < self.objects[new_y].len(),
                    "Enemy out of bounds"
                );
                self.objects[new_y][new_x].push(enemy.object.clone());
            }
        }

        let player_old_x = (self.player.object.pos.x / 16.0).round() as usize;
        let player_old_y = (self.player.object.pos.y / 16.0).round() as usize;
        self.objects[player_old_y][player_old_x]
            .retain(|obj| obj.object_type != ObjectType::Player);
        let player_surrounding_objects =
            Self::get_surrounding_objects(&self.objects, &self.player.object);
        self.player.update(player_surrounding_objects);
        let player_new_x = (self.player.object.pos.x / 16.0).round() as usize;
        let player_new_y = (self.player.object.pos.y / 16.0).round() as usize;
        assert!(
            player_new_y < self.objects.len() && player_new_x < self.objects[player_new_y].len(),
            "Player out of bounds",
        );
        self.objects[player_new_y][player_new_x].push(self.player.object.clone());

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
                for enemy in &self.enemies {
                    enemy.draw(self.camera.x, self.camera.y);
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

        draw_text(&format!("FPS: {}", get_fps()), 10.0, 10.0, 20.0, WHITE);
        next_frame().await;
    }
}
