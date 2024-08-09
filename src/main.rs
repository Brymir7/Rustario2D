use image_utils::load_and_convert_texture;
use macroquad::prelude::*;
use mario_config::mario_config::{
    ACCELERATION, GRAVITY, JUMP_STRENGTH, MARIO_SPRITE_BLOCK_SIZE, MARIO_WORLD_SIZE,
    MAX_VELOCITY_X, PHYSICS_FRAME_PER_SECOND, PHYSICS_FRAME_TIME, SCALE_IMAGE_FACTOR,
};
use preparation::LevelData;
use std::cell::Ref;
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
#[derive(Debug)]
enum CollisionType {
    PlayerWithBlock,
    PlayerKillEnemy,
    PlayerHitBy,
    PlayerWithPowerup,
    EnemyWithBlock,
    EnemyWithEnemy,
}
struct CollisionResponse {
    new_pos: Vec2,
    new_velocity: Vec2,
    collided: bool,
    collision_type: Option<CollisionType>,
}
fn resolve_collision_general(
    object: &Object,
    velocity: &Vec2,
    other: &Object,
) -> CollisionResponse {
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
    CollisionResponse {
        new_pos,
        new_velocity,
        collided: x_overlap > 0.0 && y_overlap > 0.0,
        collision_type: None,
    }
}

trait CollisionHandler {
    fn resolve_collision(
        &self,
        object: &Object,
        velocity: &Vec2,
        other: &Object,
    ) -> CollisionResponse;
}
struct DoNothingCollisionHandler;
impl CollisionHandler for DoNothingCollisionHandler {
    fn resolve_collision(&self, object: &Object, velocity: &Vec2, _: &Object) -> CollisionResponse {
        CollisionResponse {
            new_pos: object.pos,
            new_velocity: *velocity,
            collided: false,
            collision_type: None,
        }
    }
}
struct BlockCollisionHandler;
impl CollisionHandler for BlockCollisionHandler {
    fn resolve_collision(
        &self,
        object: &Object,
        velocity: &Vec2,
        other: &Object,
    ) -> CollisionResponse {
        return resolve_collision_general(object, velocity, other);
    }
}
struct EnemyCollisionHandler;
impl CollisionHandler for EnemyCollisionHandler {
    fn resolve_collision(
        &self,
        object: &Object,
        velocity: &Vec2,
        other: &Object,
    ) -> CollisionResponse {
        // let collision_response = resolve_collision_general(object, velocity, other);
        CollisionResponse {
            new_pos: Vec2::new(object.pos.x - velocity.x * 2.0, object.pos.y), // keep old position, such that other goomba also inverts
            new_velocity: Vec2::new(-velocity.x, velocity.y), // reverse direction, typical mario goomba | goomba collision
            collided: true,
            collision_type: Some(CollisionType::EnemyWithEnemy),
        }
    }
}
struct EnemyBlockCollisionHandler;
impl CollisionHandler for EnemyBlockCollisionHandler {
    fn resolve_collision(
        &self,
        object: &Object,
        velocity: &Vec2,
        other: &Object,
    ) -> CollisionResponse {
        let collision_response = resolve_collision_general(object, velocity, other);
        if other.pos.y / 16.0 == object.pos.y / 16.0 {
            // if goomba is on the same level as block, reverse direction
            let new_pos = Vec2::new(collision_response.new_pos.x, collision_response.new_pos.y); // move goomba back a bit, otherwise it will get stuck
            return CollisionResponse {
                new_pos,
                new_velocity: Vec2::new(-velocity.x, velocity.y),
                collided: collision_response.collided,
                collision_type: Some(CollisionType::EnemyWithBlock),
            };
        }
        collision_response
    }
}
struct PlayerEnemyCollisionHandler;
impl CollisionHandler for PlayerEnemyCollisionHandler {
    fn resolve_collision(
        &self,
        object: &Object,
        velocity: &Vec2,
        other: &Object,
    ) -> CollisionResponse {
        let collision_response = resolve_collision_general(object, velocity, other);
        if collision_response.collided {
            println!("Player collided with enemy");
            if object.pos.y < other.pos.y {
                return CollisionResponse {
                    new_pos: collision_response.new_pos,
                    new_velocity: collision_response.new_velocity,
                    collided: collision_response.collided,
                    collision_type: Some(CollisionType::PlayerKillEnemy),
                };
            } else {
                return CollisionResponse {
                    new_pos: collision_response.new_pos,
                    new_velocity: collision_response.new_velocity,
                    collided: collision_response.collided,
                    collision_type: Some(CollisionType::PlayerHitBy),
                };
            }
        }
        return collision_response;
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
    fn handle_world_border(&mut self, world_bounds: (usize, usize)) -> Option<GameEvent>;
    fn update(
        &mut self,
        surrounding_objects: Vec<Object>,
        world_bounds: (usize, usize),
    ) -> Option<Vec<GameEvent>> {
        let mut possible_game_events: Option<Vec<GameEvent>> = None;
        let self_center_x: f32 = self.object().pos.x + self.object().width as f32 / 2.0;
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
            let collision_handler = self.get_collision_handler(other.object_type);
            let collision_response =
                collision_handler.resolve_collision(self.object(), self.velocity(), other);

            if collision_response.collided {
                *self.mut_velocity() = collision_response.new_velocity;
                self.mut_object().pos = collision_response.new_pos;
            }

            if let Some(collision_type) = collision_response.collision_type {
                possible_game_events = Some(Vec::new());
                let game_event = match collision_type {
                    CollisionType::PlayerKillEnemy => GameEvent {
                        event: GameEventType::Kill,
                        triggered_by: other.clone(),
                        target: Some(self.object().clone()),
                    },
                    CollisionType::PlayerHitBy => GameEvent {
                        event: GameEventType::PlayerHit,
                        triggered_by: other.clone(),
                        target: Some(self.object().clone()),
                    },

                    _ => continue,
                };
                if let Some(mut events) = possible_game_events {
                    events.push(game_event);
                    possible_game_events = Some(events);
                } else {
                    possible_game_events = Some(vec![game_event]);
                }
            }
        }
        let game_event = self.handle_world_border(world_bounds);
        if let Some(game_event) = game_event {
            if let Some(mut events) = possible_game_events {
                events.push(game_event);
                possible_game_events = Some(events);
            } else {
                possible_game_events = Some(vec![game_event]);
            }
        }
        self.update_animation();
        self.animate().update();

        possible_game_events
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
    PowerupBlock,
}

#[derive(PartialEq, Clone, Copy, Debug)]
enum EnemyType {
    Goomba,
}

#[derive(PartialEq, Clone, Copy, Debug)]
enum ObjectType {
    Block(BlockType),
    Enemy(EnemyType),
    Player,
}

#[derive(Clone, Debug)]
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

enum PlayerPowerupState {
    Small,
}
#[derive(Clone)]
struct Animate {
    frames: Vec<Texture2D>,
    current_frames_sprite: usize,
    speed_factor: f32,
    time_to_change: f32,
    time_elapsed: f32,
}

impl Animate {
    fn new(speed_factor: f32) -> Self {
        assert!(speed_factor > 0.0);
        Animate {
            frames: Vec::new(),
            current_frames_sprite: 0,
            speed_factor,
            time_to_change: (PHYSICS_FRAME_TIME * 3.0) * speed_factor, // 10 frames per sprite
            time_elapsed: 0.0,
        }
    }

    fn change_animation_sprites(&mut self, new_frames: Vec<Texture2D>) {
        if new_frames != self.frames {
            self.frames = new_frames;
            self.current_frames_sprite = 0;
            self.time_elapsed = 0.0;
        }
    }

    fn update(&mut self) {
        if self.frames.len() > 1 {
            self.time_elapsed += PHYSICS_FRAME_TIME;
            if self.time_elapsed >= self.time_to_change {
                self.current_frames_sprite = (self.current_frames_sprite + 1) % self.frames.len();
                self.time_elapsed -= self.time_to_change;
            }
        }
    }
    fn scale_animation_speed(&mut self, factor: f32) {
        assert!(factor > 0.0);
        self.speed_factor = factor;
        self.time_to_change = (PHYSICS_FRAME_TIME * 3.0) * (1.0 / self.speed_factor);
    }
    fn current_frames_sprite(&self) -> Option<&Texture2D> {
        self.frames.get(self.current_frames_sprite)
    }
}

struct Player {
    object: Object,
    max_speed: i32,
    velocity: Vec2,
    is_grounded: bool,
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
            ObjectType::Enemy(EnemyType::Goomba) => Box::new(PlayerEnemyCollisionHandler),
            _ => panic!("No collision handler for object type: {:?}", object_type),
        }
    }
    fn handle_world_border(&mut self, world_bounds: (usize, usize)) -> Option<GameEvent> {
        if self.object.pos.x < 0.0 {
            self.object.pos.x = 0.0;
            self.velocity.x = 0.0;
        }
        if self.object.pos.x + self.object.width as f32 > world_bounds.0 as f32 {
            return Some(GameEvent {
                event: GameEventType::GameWon,
                triggered_by: self.object.clone(),
                target: None,
            });
        }
        if self.object.pos.y > world_bounds.1 as f32 {
            return Some(GameEvent {
                event: GameEventType::GameOver,
                triggered_by: self.object.clone(),
                target: None,
            });
        }
        None
    }
    fn update_animation(&mut self) {
        // Use velocity and keyboard input to determine the correct animation frames
        if self.velocity.y.abs() != 0.0 {
            if self.velocity.x.abs() > 2.5 {
                // Running Jump
                self.animate
                    .change_animation_sprites(vec![MARIO_SPRITE_LOOKUP[5].clone()]);
                return;
            } else {
                // Jumping
                self.animate
                    .change_animation_sprites(vec![MARIO_SPRITE_LOOKUP[3].clone()]);
                return;
            }
        } else if self.velocity.x.abs() > 0.1 {
            // Running
            if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
                if self.velocity.x < 0.0 {
                    // Turning
                    self.animate
                        .change_animation_sprites(vec![MARIO_SPRITE_LOOKUP[4].clone()]);
                    return;
                }
            } else if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
                if self.velocity.x > 0.0 {
                    // Turning
                    self.animate
                        .change_animation_sprites(vec![MARIO_SPRITE_LOOKUP[4].clone()]);
                    return;
                }
            }
            self.animate
                .change_animation_sprites(MARIO_SPRITE_LOOKUP[1..3].to_vec());
            self.animate
                .scale_animation_speed(self.velocity.x.abs() / self.max_speed as f32);
        } else {
            // Idle
            self.animate
                .change_animation_sprites(vec![MARIO_SPRITE_LOOKUP[0].clone()]);
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
            animate: Animate::new(1.0),
        };
        player
            .animate
            .change_animation_sprites(vec![MARIO_SPRITE_LOOKUP[0].clone()]);
        player
    }

    fn update(
        &mut self,
        surrounding_objects: Vec<Object>,
        world_bounds: (usize, usize),
    ) -> Option<Vec<GameEvent>> {
        return Updatable::update(self, surrounding_objects, world_bounds);
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
        if let Some(sprite_to_draw) = self.animate.current_frames_sprite() {
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
#[derive(Clone)]
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
    fn handle_world_border(&mut self, world_bounds: (usize, usize)) -> Option<GameEvent> {
        if self.object.pos.x < 0.0 {
            self.object.pos.x = 0.0;
            self.velocity.x = 0.0;
        }
        if self.object.pos.x + self.object.width as f32 > world_bounds.0 as f32 {
            self.object.pos.x = world_bounds.0 as f32 - self.object.width as f32;
            self.velocity.x = 0.0;
        }
        if self.object.pos.y > world_bounds.1 as f32 {
            return Some(GameEvent {
                event: GameEventType::Kill,
                triggered_by: self.object.clone(),
                target: None,
            });
        }
        None
    }
    fn get_collision_handler(&self, object_type: ObjectType) -> Box<dyn CollisionHandler> {
        match object_type {
            ObjectType::Block(_) => Box::new(EnemyBlockCollisionHandler),
            ObjectType::Enemy(EnemyType::Goomba) => Box::new(EnemyCollisionHandler),
            ObjectType::Player => Box::new(DoNothingCollisionHandler), // Goomba does not interact with player, player will handle goomba collision
        }
    }
    fn update_animation(&mut self) {
        if self.velocity.x.abs() > 0.1 {
            self.animate
                .change_animation_sprites(GOOMBA_SPRITE_LOOKUP.to_vec());
            self.animate
                .scale_animation_speed(self.velocity.x.abs() / self.max_speed as f32);
        } else {
            self.animate
                .change_animation_sprites(vec![GOOMBA_SPRITE_LOOKUP[0].clone()]);
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
            .change_animation_sprites(GOOMBA_SPRITE_LOOKUP.to_vec());
        goomba
    }
    fn update(
        &mut self,
        surrounding_objects: Vec<Object>,
        world_bounds: (usize, usize),
    ) -> Option<Vec<GameEvent>> {
        self.velocity.x = 1.0 * self.velocity.x.signum(); // avoid friction atm;
        return Updatable::update(self, surrounding_objects, world_bounds);
    }
    fn draw(&self, camera_x: usize, camera_y: usize) {
        // TODO! draw using Animate
        if let Some(sprite_to_draw) = self.animate.current_frames_sprite() {
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
            draw_rectangle_lines(
                self.object.pos.x - camera_x as f32,
                self.object.pos.y - camera_y as f32,
                self.object.width as f32 * SCALE_IMAGE_FACTOR,
                self.object.height as f32 * SCALE_IMAGE_FACTOR,
                1.0,
                RED,
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
#[derive(Debug)]
enum GameEventType {
    GameWon,
    GameOver,
    Kill,
    PlayerHit,
}
#[derive(Debug)]
struct GameEvent {
    event: GameEventType,
    triggered_by: Object,
    target: Option<Object>,
}
enum GameState {
    Playing,
    GameWon,
    GameOver,
}
#[derive(Clone)]
enum ObjectReference {
    Block(Object),
    Enemy(usize), // Index into the self.enemies vector
    Player,
    None,
}
struct World {
    height: usize,
    width: usize,
    objects: Vec<Vec<ObjectReference>>,
    player: Player,
    enemies: Vec<Goomba>,
    camera: Camera,
    game_state: GameState,
    level_texture: Option<Texture2D>,
}

impl World {
    fn new(height: usize, width: usize) -> World {
        let objects =
            vec![vec![ObjectReference::None; width / MARIO_SPRITE_BLOCK_SIZE as usize]; height];
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
                if let Some(object_type) = SPRITE_TYPE_MAPPING.get(&tile) {
                    self.add_object(Object::new(x as usize, y as usize, object_type.clone()));
                }
            }
        }
        set_default_camera();
        let render_texture = render_target_camera.render_target.unwrap().texture;
        self.level_texture = Some(render_texture); // to draw in one call, while keeping compressed json instead of loading a .png
    }

    async fn load_player(&mut self) {
        self.player = Player::new(48, 176, MAX_VELOCITY_X);
    }
    fn spawn_enemies(&mut self) {
        self.add_object(Object::new(160, 176, ObjectType::Enemy(EnemyType::Goomba)));
        self.add_object(Object::new(224, 176, ObjectType::Enemy(EnemyType::Goomba)));
        self.add_object(Object::new(640, 176, ObjectType::Enemy(EnemyType::Goomba)));
        self.add_object(Object::new(776, 176, ObjectType::Enemy(EnemyType::Goomba)));
        self.add_object(Object::new(876, 176, ObjectType::Enemy(EnemyType::Goomba)));
        self.add_object(Object::new(2648, 176, ObjectType::Enemy(EnemyType::Goomba)));
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
        if let ObjectReference::None = self.objects[y][x] {
            self.objects[y][x] = match object.object_type {
                ObjectType::Block(_) => ObjectReference::Block(object),
                ObjectType::Enemy(_) => ObjectReference::Enemy(self.enemies.len() - 1),
                ObjectType::Player => ObjectReference::Player,
            };
        } else {
            panic!("Tried to add object where: Object already exists");
        }
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
    fn get_surrounding_objects(
        objects: &Vec<Vec<ObjectReference>>,
        enemies: &Vec<Goomba>,
        object: &Object,
    ) -> Vec<Object> {
        let surrounding_objects_refs = DIRECTIONS
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
            .collect::<Vec<ObjectReference>>();
        surrounding_objects_refs
            .iter()
            .filter_map(|reference| match reference {
                ObjectReference::Block(object) => Some(object.clone()),
                ObjectReference::Enemy(index) => {
                    if *index > enemies.len() - 1 {
                        return None;
                    }
                    let enemy = enemies[*index].clone();
                    Some(enemy.object)
                }
                ObjectReference::Player => None,
                ObjectReference::None => None,
            })
            .collect::<Vec<Object>>()
    }
    fn handle_game_event(&mut self, game_event: GameEvent) {
        println!("Game event: {:?}", game_event.event);
        match game_event.event {
            GameEventType::GameWon => {
                self.game_state = GameState::GameWon;
            }
            GameEventType::GameOver => {
                self.game_state = GameState::GameOver;
            }
            GameEventType::Kill => {
                let obj_idx_x = (game_event.triggered_by.pos.x / 16.0).round() as usize;
                let obj_idx_y = (game_event.triggered_by.pos.y / 16.0).round() as usize;
                self.enemies
                    .retain(|enemy| enemy.object != game_event.triggered_by);
                self.objects[obj_idx_y][obj_idx_x] = ObjectReference::None;
            }
            GameEventType::PlayerHit => {}
        }
    }

    fn update(&mut self) {
        let mut game_events = Vec::new();
        for i in 0..self.enemies.len() {
            let (before, after) = self.enemies.split_at_mut(i);
            let (enemy, after) = &mut after.split_at_mut(1);

            let other_enemies: Vec<Goomba> = before
                .iter_mut()
                .chain(after.iter_mut().skip(1))
                .map(|e| (*e).clone())
                .collect();
            let enemy = &mut enemy[0];
            let surrounding_objects =
                Self::get_surrounding_objects(&self.objects, &other_enemies, &enemy.object);

            let old_x = (enemy.object.pos.x / 16.0).round() as usize;
            let old_y = (enemy.object.pos.y / 16.0).round() as usize;

            let game_event = enemy.update(surrounding_objects, (self.width, self.height));
            game_events.push(game_event);

            let new_x = (enemy.object.pos.x / 16.0).round() as usize;
            let new_y = (enemy.object.pos.y / 16.0).round() as usize;

            if old_x == new_x && old_y == new_y {
                continue;
            }
            self.objects[old_y][old_x] = ObjectReference::None;
            self.objects[new_y][new_x] = ObjectReference::Enemy(i);
        }

        let player_old_x = (self.player.object.pos.x / 16.0).round() as usize;
        let player_old_y = (self.player.object.pos.y / 16.0).round() as usize;
        self.objects[player_old_y][player_old_x] = ObjectReference::None;
        let player_surrounding_objects: Vec<Object> =
            Self::get_surrounding_objects(&self.objects, &self.enemies, &self.player.object);

        let game_event = self
            .player
            .update(player_surrounding_objects, (self.width, self.height));
        game_events.push(game_event);
        let player_new_x = (self.player.object.pos.x / 16.0).round() as usize;
        let player_new_y = (self.player.object.pos.y / 16.0).round() as usize;
        assert!(
            player_new_y < self.objects.len() && player_new_x < self.objects[player_new_y].len(),
            "Player out of bounds",
        );
        self.objects[player_new_y][player_new_x] = ObjectReference::Player;

        self.camera.update(
            self.player.object.pos.x as usize,
            self.player.object.pos.y as usize,
        );

        for game_event_queue in game_events {
            if let Some(game_event_q) = game_event_queue {
                for game_event in game_event_q {
                    self.handle_game_event(game_event);
                }
            }
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
    world.spawn_enemies();
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
