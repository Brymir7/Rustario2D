use animation::animation::{FrameType, PlayAnimation, PlayAnimationBuilder};
use image_utils::load_and_convert_texture;
use macroquad::audio::{load_sound, play_sound, PlaySoundParams, Sound};
use macroquad::prelude::*;
use mario_config::mario_config::{
    ACCELERATION, GRAVITY, JUMP_STRENGTH, MARIO_NON_MUSIC_VOLUME, MARIO_SPRITE_BLOCK_SIZE, MARIO_WORLD_SIZE, MAX_VELOCITY_X, PHYSICS_FRAME_PER_SECOND, PHYSICS_FRAME_TIME, SCALE_IMAGE_FACTOR, SOUND_VOLUME
};
use preparation::LevelData;
use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::any::Any;
use std::usize;

pub mod image_utils;
pub mod mario_config;
pub mod animation;  
pub mod preparation;
use lazy_static::lazy_static;

lazy_static! {
    static ref SPRITE_ID_TO_TYPE: HashMap<&'static u8, ObjectType> = { // potentially rewrite as array lookup
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
    static ref SPRITE_ID_TO_TEXTURE2D: HashMap<u8, Texture2D> = { // potentially rewrite as array lookup
        let mut m  = HashMap::new();
        let tilesheet = Image::from_file_with_format(
            include_bytes!("../sprites/tilesheet.png"),
            Some(ImageFormat::Png),
        ).expect("Failed to load tilesheet.png");

        let amount_of_tiles = tilesheet.height() / MARIO_SPRITE_BLOCK_SIZE;
        assert!(amount_of_tiles < 256);
        for i in 0..amount_of_tiles {
            let mut tile_image = Image::gen_image_color(16, 16, Color::new(0.0, 0.0, 0.0, 0.0));
            for y in 0..16 {
                for x in 0..16 {
                    let color = tilesheet.get_pixel(x, y + (MARIO_SPRITE_BLOCK_SIZE*i) as u32);
                    tile_image.set_pixel(x, y, color);
                }
            }
            let tile_texture = Texture2D::from_image(&tile_image);
            tile_texture.set_filter(FilterMode::Nearest);
            m.insert(i.try_into().expect("Tilesheet has unexpected size"), tile_texture);
        }
        return m;
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
    static ref POWERUP_SPRITE_LOOKUP: [Texture2D; 1] = [load_and_convert_texture(
        include_bytes!("../sprites/Mushroom.png"),
        ImageFormat::Png
    ),];


}
#[allow(dead_code)]
enum DrawPortion {
    Top(f32),
    Bottom(f32), 
    Left(f32),
    Right(f32),
}

struct WorldBounds {
    min_x: usize,
    max_x: usize,
    max_y: usize,
}
#[derive(Debug)]
enum CollisionType {
    PlayerWithBlock,
    PlayerKillEnemy,
    PlayerHitBy,
    PlayerWithPowerupBlock,
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
fn get_collision_response(
    object: &Object,
    velocity: &Vec2,
    other: &SurroundingObject,
) -> CollisionResponse { 
    let (other, relative_direction_to_object) = (&other.object, other.relative_direction);
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
        match relative_direction_to_object {
            (0, -1) | (0, 1) => {
                new_pos.x -= x_overlap * relative_direction_to_object.1 as f32;
                if velocity.x.signum() == relative_direction_to_object.1 as f32 {
                    new_velocity.x = 0.0;
                }
            },
            (-1, 0) | (1, 0) => {
                new_pos.y -= y_overlap * relative_direction_to_object.0 as f32;
                new_velocity.y = 0.0;
            },
            _ => {
                if x_overlap < y_overlap  {
                    new_pos.x -= x_overlap * relative_direction_to_object.1 as f32;
                    if velocity.x.signum() == relative_direction_to_object.1 as f32 {
                        new_velocity.x = 0.0;
                    }
                } else {
                    new_pos.y -= y_overlap * relative_direction_to_object.0 as f32;
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
enum SpawnAnimation {
    PowerUp,
}
struct SpawningObject {
    object: Box<dyn Updatable>,
    animation_progress: f32,
    animation_finish: f32,
    spawn_animation: SpawnAnimation, // only for spawning, animate is used for alive objects
    draw_offset: Vec2,
}


impl SpawningObject {
    fn new(object: impl Updatable) -> Self {
        match object.object().object_type {
            ObjectType::Powerup => {
                SpawningObject {
                    object: Box::new(object),
                    animation_progress: 0.0,
                    animation_finish: MARIO_SPRITE_BLOCK_SIZE as f32,
                    spawn_animation: SpawnAnimation::PowerUp,
                    draw_offset: Vec2::new(0.0 , MARIO_SPRITE_BLOCK_SIZE as f32) // draw it where the block is then move up
                }
            }
            _ => panic!("Spawning object with animation not implemented for object type: {:?}", object.object().object_type),
        }
    }

    fn update(&mut self) -> bool { 
        match self.spawn_animation {
            SpawnAnimation::PowerUp => {
                let move_by = 0.5;
                self.animation_progress += move_by;
                if self.animation_progress >= self.animation_finish {
                    return true;
                } else {
                    self.draw_offset.y -= move_by;
                    return false;
                }
            }
        }
    }
    fn draw(& self, camera_x: usize, camera_y: usize) {
        match self.spawn_animation {
            SpawnAnimation::PowerUp => {
                let object = self.object.object();
                self.object.animate().draw(
                    &(object.pos + self.draw_offset),
                    object.width,
                    object.height,
                    &self.object.velocity(),
                    camera_x,
                    camera_y,
                    Some(DrawPortion::Top(self.animation_progress / self.animation_finish)),
                );
            }

        }
    
    }
}

trait CollisionHandler {
    fn resolve_collision(
        &self,
        object: &Object,
        velocity: &Vec2,
        other: &SurroundingObject,
    ) -> CollisionResponse;
}
struct DoNothingCollisionHandler;
impl CollisionHandler for DoNothingCollisionHandler {
    fn resolve_collision(&self, object: &Object, velocity: &Vec2, _: &SurroundingObject) -> CollisionResponse {
        CollisionResponse {
            new_pos: object.pos,
            new_velocity: *velocity,
            collided: false,
            collision_type: None,
        }
    }
}
struct PowerupCollisionHandler;
impl CollisionHandler for PowerupCollisionHandler {
    fn resolve_collision(
        &self,
        object: &Object,
        velocity: &Vec2,
        other: &SurroundingObject,
    ) -> CollisionResponse {
        let collision_response = get_collision_response(object, velocity, other);

        if collision_response.collided {
            return CollisionResponse {
                new_pos: object.pos,
                new_velocity: *velocity,
                collided: collision_response.collided,
                collision_type: Some(CollisionType::PlayerWithPowerup),
            };
        }
        collision_response
    }
}
struct BlockCollisionHandler;
impl CollisionHandler for BlockCollisionHandler {
    fn resolve_collision(
        &self,
        object: &Object,
        velocity: &Vec2,
        other: &SurroundingObject,
    ) -> CollisionResponse {
        let collision_response = get_collision_response(object, velocity, other);
        match other.object.object_type {
            ObjectType::Block(BlockType::Block) => {
                if collision_response.collided {
                    return CollisionResponse {
                        new_pos: collision_response.new_pos,
                        new_velocity: collision_response.new_velocity,
                        collided: collision_response.collided,
                        collision_type: Some(CollisionType::PlayerWithBlock),
                    };
                }
            }
            ObjectType::Block(BlockType::PowerupBlock) => {
                if collision_response.collided {

                    return CollisionResponse {
                        new_pos: collision_response.new_pos,
                        new_velocity: collision_response.new_velocity,
                        collided: collision_response.collided,
                        collision_type: {
                            if other.relative_direction == (-1, 0) && velocity.y < 0.0 && object.object_type == ObjectType::Player && object.pos.y > other.object.pos.y { 

                                Some(CollisionType::PlayerWithPowerupBlock)
                            } else {
                                Some(CollisionType::PlayerWithBlock)
                            }
                        },
                    };
                }
            }
            _ => {}
        }
        collision_response
    }
}
struct EnemyCollisionHandler;
impl CollisionHandler for EnemyCollisionHandler {
    fn resolve_collision(
        &self,
        object: &Object,
        velocity: &Vec2,
        other: &SurroundingObject,
    ) -> CollisionResponse {
        let collision_response = get_collision_response(object, velocity, other);
        let new_velo = Vec2::new(-velocity.x, velocity.y);
        let new_pos = Vec2::new(object.pos.x, object.pos.y);

        CollisionResponse {
            new_pos: new_pos,       // move goomba back a bit, otherwise it will get stuck
            new_velocity: new_velo, // reverse direction, typical mario goomba | goomba collision
            collided: collision_response.collided,
            collision_type: match collision_response.collided {
                true => Some(CollisionType::EnemyWithEnemy),
                false => None,
            },
        }
    }
}
struct EnemyBlockCollisionHandler;
impl CollisionHandler for EnemyBlockCollisionHandler {
    fn resolve_collision(
        &self,
        object: &Object,
        velocity: &Vec2,
        other: &SurroundingObject,
    ) -> CollisionResponse {
        let collision_response = get_collision_response(object, velocity, other);
        if other.object.pos.y / MARIO_SPRITE_BLOCK_SIZE as f32 == object.pos.y / MARIO_SPRITE_BLOCK_SIZE as f32 {
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
        other: &SurroundingObject,
    ) -> CollisionResponse {
        let collision_response = get_collision_response(object, velocity, other);
        if collision_response.collided {
            if (object.pos.y + object.height as f32) < (other.object.pos.y + other.object.height as f32) {

                return CollisionResponse {
                    new_pos: collision_response.new_pos,
                    new_velocity: Vec2::new(velocity.x, -3.0), // bounce up
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
trait Updatable: 'static{
    fn as_any(&self) -> &dyn Any;
    fn mut_object(&mut self) -> &mut Object;
    fn mut_velocity(&mut self) -> &mut Vec2;
    fn object(&self) -> &Object;
    fn velocity(&self) -> &Vec2;

    fn set_grounded(&mut self, grounded: bool);
    fn animate(& self) -> & Animate;
    fn mut_animate(&mut self) -> &mut Animate;
    fn apply_gravity(&mut self) {
        self.mut_velocity().y += GRAVITY as f32 * PHYSICS_FRAME_TIME;
    }

    fn apply_x_axis_friction(&mut self, grounded: bool) {
        if !grounded {
            self.mut_velocity().x =
                (self.velocity().x.abs() - 1.0 * PHYSICS_FRAME_TIME) * self.velocity().x.signum();
        } else {
            self.mut_velocity().x =
                (self.velocity().x.abs() - 2.0 * PHYSICS_FRAME_TIME) * self.velocity().x.signum();
        }

    }
    fn update_animation(&mut self) {}
    fn get_collision_handler(&self, object_type: ObjectType) -> Box<dyn CollisionHandler>; // this could be a trait enum?
    fn handle_world_border(&mut self, world_bounds: WorldBounds) -> Option<GameEvent>;
    fn update(
        &mut self,
        surrounding_objects: &Vec<SurroundingObject>,
        world_bounds: WorldBounds,
    ) -> Vec<GameEvent> {
        let self_center_x: f32 = self.object().pos.x + self.object().width as f32 / 2.0;
        let block_below = surrounding_objects
            .iter()
            .find(|obj| {
                obj.relative_direction == (1, 0)
                    && obj.object.pos.x < self_center_x
                    && obj.object.pos.x + obj.object.width as f32 > self_center_x
            });

        if block_below.is_none() {
            self.apply_gravity();
            self.set_grounded(false);
            self.apply_x_axis_friction(false);
        } else {

            self.set_grounded(true);
            self.apply_x_axis_friction(true);
        }

        let velocity = self.velocity().clone();
        self.mut_object().pos += velocity;
        let mut game_events = Vec::new();
        for other in surrounding_objects {
            let collision_handler = self.get_collision_handler(other.object.object_type);
            let collision_response =
                collision_handler.resolve_collision(self.object(), self.velocity(), other);

            match collision_response.collision_type {
                Some(ref collision_type) => {
                    let game_event = self.create_game_event(collision_type, &other.object);
                    if let Some(event) = game_event {
                        game_events.push(event);
                    }
                }
                None => {}
            }
            if collision_response.collided {
                self.update_position_and_velocity(&collision_response);
            }
        }
        let game_event = self.handle_world_border(world_bounds);
        if let Some(event) = game_event {
            game_events.push(event);
        }
        self.update_animation();
        self.mut_animate().update();

        game_events
    }

    fn create_game_event(
        &self,
        collision_type: &CollisionType,
        other: &Object,
    ) -> Option<GameEvent> {
        match collision_type {
            CollisionType::PlayerKillEnemy => Some(GameEvent {
                event: GameEventType::Kill,
                triggered_by: self.object().clone(),
                target: Some(other.clone()),
            }),
            CollisionType::PlayerHitBy => Some(GameEvent {
                event: GameEventType::PlayerHit,
                triggered_by: other.clone(),
                target: Some(self.object().clone()),
            }),
            CollisionType::PlayerWithBlock=> Some(GameEvent {
                event: GameEventType::PlayerHitBlock,
                triggered_by: self.object().clone(),
                target: Some(other.clone()),
            }),
            CollisionType::PlayerWithPowerupBlock => Some(GameEvent {
                event: GameEventType::PlayerHitPowerupBlock,
                triggered_by: self.object().clone(),
                target: Some(other.clone()),
            }),
            CollisionType::EnemyWithBlock => None,
            CollisionType::EnemyWithEnemy => {
                // Goomba collision with goomba
                Some(GameEvent {
                    event: GameEventType::EnemyCollEnemy,
                    triggered_by: self.object().clone(),
                    target: Some(other.clone()),
                })
            }
            CollisionType::PlayerWithPowerup => Some(GameEvent {
                event: GameEventType::PlayerPowerUp,
                triggered_by: self.object().clone(),
                target: Some(other.clone()),
            }),

        }
    }
    fn update_position_and_velocity(&mut self, collision_response: &CollisionResponse) {
        *self.mut_velocity() = collision_response.new_velocity;
        self.mut_object().pos = collision_response.new_pos;
    }
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
    Powerup,
    Player,
}
struct SurroundingObject {
    object: Object,
    relative_direction: (isize, isize),
}
impl SurroundingObject {
    fn new(object: Object, relative_direction: (isize, isize)) -> SurroundingObject {
        assert!(relative_direction.0.abs() <= 1 && relative_direction.1.abs() <= 1);
        assert!(relative_direction != (0, 0));
        SurroundingObject {
            object,
            relative_direction,
        }
    }
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

enum PlayerState {
    Dead,
    Small,
    Big,
}
#[derive(Clone)]
struct Animate {
    frames: Vec<Texture2D>,
    animation: Option<PlayAnimation>,
    current_frame_index: usize,
    speed_factor: f32,
    time_to_change: f32,
    time_elapsed: f32,
}

impl Animate {
    fn new(speed_factor: f32) -> Self {
        assert!(speed_factor > 0.0);
        Animate {
            frames: Vec::new(),
            animation: None,
            current_frame_index: 0,
            speed_factor,
            time_to_change: (PHYSICS_FRAME_TIME * 5.0) / speed_factor,
            time_elapsed: 0.0,
        }
    }

    fn change_animation_sprites(&mut self, new_frames: Vec<Texture2D>) {
        if new_frames != self.frames {
            self.frames = new_frames;
            self.current_frame_index = 0;
            self.time_elapsed = 0.0;
        }
    }

    fn play_animation(&mut self, animation: PlayAnimation) {

        self.animation = Some(animation);
        self.current_frame_index = 0;
        self.time_elapsed = 0.0;

    }

    fn update(&mut self) {
        self.time_elapsed += PHYSICS_FRAME_TIME;
        if !(self.time_elapsed >= self.time_to_change)  {
            return;
        }
        if let Some(animation) = &mut self.animation {
            if let Some(mut loop_for) = animation.loop_for {
                if self.time_elapsed >= loop_for {
                    self.reset_animation();
                    return;
                }
                animation.next_frame();
                loop_for -= self.time_elapsed;
                animation.loop_for = Some(loop_for);
            }
            else if !animation.next_frame() {
                self.reset_animation();
            }
        } else if self.frames.len() > 1 {
            self.current_frame_index = (self.current_frame_index + 1) % self.frames.len();
        }
        self.time_elapsed -= self.time_to_change;
    }

    fn scale_animation_speed(&mut self, factor: f32) {
        assert!(factor > 0.0);
        self.speed_factor = factor;
        self.time_to_change = (PHYSICS_FRAME_TIME * 5.0) / self.speed_factor;
    }

    fn current_texture_frame(&self) -> Option<&Texture2D> {
        if let Some(animation) = &self.animation {
            animation.texture_frames.get(self.current_frame_index)
        } else {
            self.frames.get(self.current_frame_index)
        }
    }
    fn reset_animation(&mut self) {
        self.animation = None;
        self.current_frame_index = 0;
        self.time_elapsed = 0.0;
    }
    fn draw(&self, pos: &Vec2, width: usize, height: usize, velocity: &Vec2, camera_x: usize, camera_y: usize, draw_portion: Option<DrawPortion>) {
        if let Some(sprite_to_draw) = self.current_texture_frame() {
            let mut src_rect = Rect::new(0.0, 0.0, sprite_to_draw.width(), sprite_to_draw.height());
            let mut dest_size = Vec2::new(
                (width * SCALE_IMAGE_FACTOR) as f32,
                (height * SCALE_IMAGE_FACTOR) as f32
            );
            let mut pos_offset = Vec2::ZERO;

            if let Some(animation) = &self.animation {
                match &animation.frame_type {
                    Some(FrameType::Height(frames)) => {
                        dest_size.y = frames[animation.frame_index] as f32 * SCALE_IMAGE_FACTOR as f32;
                    }
                    Some(FrameType::Width(frames)) => {
                        dest_size.x = frames[animation.frame_index] as f32 * SCALE_IMAGE_FACTOR as f32;
                    }
                    Some(FrameType::PosOffset(frames)) => {
                        pos_offset = frames[animation.frame_index];
                    }
                    None => {}
                }
            }

            if let Some(portion) = draw_portion {
                match portion {
                    DrawPortion::Top(percentage) => {
                        let clamped_percentage = percentage.clamp(0.0, 1.0);
                        src_rect.h *= clamped_percentage;
                        dest_size.y *= clamped_percentage;
                    },
                    DrawPortion::Bottom(percentage) => {
                        let clamped_percentage = percentage.clamp(0.0, 1.0);
                        src_rect.y += src_rect.h * (1.0 - clamped_percentage);
                        src_rect.h *= clamped_percentage;
                        dest_size.y *= clamped_percentage;
                    },
                    DrawPortion::Left(percentage) => {
                        let clamped_percentage = percentage.clamp(0.0, 1.0);
                        src_rect.w *= clamped_percentage;
                        dest_size.x *= clamped_percentage;
                    },
                    DrawPortion::Right(percentage) => {
                        let clamped_percentage = percentage.clamp(0.0, 1.0);
                        src_rect.x += src_rect.w * (1.0 - clamped_percentage);
                        src_rect.w *= clamped_percentage;
                        dest_size.x *= clamped_percentage;
                    },
                }
            }

            draw_texture_ex(
                sprite_to_draw,
                (pos.x + pos_offset.x - camera_x as f32) * SCALE_IMAGE_FACTOR as f32,
                (pos.y + pos_offset.y - camera_y as f32) * SCALE_IMAGE_FACTOR as f32,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(dest_size),
                    source: Some(src_rect),
                    flip_x: velocity.x < -0.1,
                    ..Default::default()
                },
            );
        }
    }
}

struct Player {
    object: Object,
    max_speed: f32,
    velocity: Vec2,
    is_grounded: bool,
    power_state: PlayerState,
    animate: Animate,
}
impl Updatable for Player {
    fn as_any(&self) -> &dyn Any {
        self
    }
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

    fn animate(& self) -> & Animate {
        & self.animate
    }
    fn mut_animate(&mut self) -> &mut Animate {
        &mut self.animate
    }
    fn get_collision_handler(&self, object_type: ObjectType) -> Box<dyn CollisionHandler> {
        match object_type {
            ObjectType::Block(_) => Box::new(BlockCollisionHandler),
            ObjectType::Enemy(EnemyType::Goomba) => Box::new(PlayerEnemyCollisionHandler),
            ObjectType::Powerup => Box::new(PowerupCollisionHandler),
            _ => panic!("No collision handler for object type: {:?}", object_type),
        }
    }
    fn handle_world_border(&mut self, world_bounds: WorldBounds) -> Option<GameEvent> {
        if self.object.pos.x < world_bounds.min_x as f32 {
            self.object.pos.x = world_bounds.min_x as f32;
            self.velocity.x = 0.0;
        }
        if self.object.pos.x + self.object.width as f32 > world_bounds.max_x as f32 {
            return Some(GameEvent {
                event: GameEventType::GameWon,
                triggered_by: self.object.clone(),
                target: None,
            });
        }
        if self.object.pos.y > world_bounds.max_y as f32 {
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
    fn new(x: usize, y: usize, max_speed: f32) -> Player {
        let mut player = Player {
            object: Object::new(x, y, ObjectType::Player),
            max_speed,
            velocity: Vec2::new(0.0, 0.0),
            is_grounded: false,
            power_state: PlayerState::Small,
            animate: Animate::new(1.0),
        };
        player
            .animate
            .change_animation_sprites(vec![MARIO_SPRITE_LOOKUP[0].clone()]);
        player
    }
    fn power_up(&mut self) {
        match self.power_state {
            PlayerState::Small => {
                self.power_state = PlayerState::Big;
                let new_height = self.object.height * 2;
                let animation = PlayAnimationBuilder::new(vec![self.animate.frames[self.animate.current_frame_index].clone()])
                    .loop_for(0.5)
                    .height_frames(vec![self.object.height, new_height])
                    .build();
                self.object.height = new_height;
                self.animate.scale_animation_speed(0.8);
                self.animate.play_animation(animation);
            }
            _ => {}
        }
    }
    fn power_down(&mut self) {
        match self.power_state {
            PlayerState::Small => {
                self.power_state = PlayerState::Dead;
            }
            PlayerState::Big => {
                self.power_state = PlayerState::Small;
                self.object.height = MARIO_SPRITE_BLOCK_SIZE;
            }
            _ => {}
        }
    }
    fn update(
        &mut self,
        surrounding_objects: &Vec<SurroundingObject>,
        world_bounds: WorldBounds,
    ) -> Vec<GameEvent> {

        return Updatable::update(self, surrounding_objects, world_bounds);
    }

    fn add_horizontal_velocity(&mut self, velocity: f32) {
        self.velocity.x += velocity;
        self.velocity.x = self
            .velocity
            .x
            .clamp(-self.max_speed as f32, self.max_speed as f32);
    }

    fn jump(&mut self, sound: &Sound) {
        const VELOCITY: f32 = -JUMP_STRENGTH * PHYSICS_FRAME_TIME;
        if self.is_grounded {
            play_sound(
                sound,
                PlaySoundParams {
                    volume: MARIO_NON_MUSIC_VOLUME * SOUND_VOLUME,
                    looped: false,
                },
            );
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
        self.animate.draw(
            &self.object.pos,
            self.object.width,
            self.object.height,
            &self.velocity,
            camera_x,
            camera_y,
            None,
        )
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
fn as_any(&self) -> &dyn Any {
        self
    }
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

    fn animate(& self) -> & Animate {
        & self.animate
    }
    fn mut_animate(&mut self) -> &mut Animate {
        &mut self.animate
    }
    fn handle_world_border(&mut self, world_bounds: WorldBounds) -> Option<GameEvent> {
        if self.object.pos.x < 0.0 {
            self.object.pos.x = 0.0;
            self.velocity.x = 0.0;
        }
        if self.object.pos.x + self.object.width as f32 > world_bounds.max_x as f32 {
            self.object.pos.x = world_bounds.max_x as f32 - self.object.width as f32;
            self.velocity.x = 0.0;
        }
        if self.object.pos.y > world_bounds.max_y as f32 {
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
            ObjectType::Enemy(_) => Box::new(EnemyCollisionHandler),
            ObjectType::Player => Box::new(DoNothingCollisionHandler), // Goomba does not interact with player, player will handle goomba collision
            ObjectType::Powerup => Box::new(EnemyCollisionHandler),
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
        surrounding_objects: &Vec<SurroundingObject>,
        world_bounds: WorldBounds,
    ) -> Vec<GameEvent> {
        self.velocity.x = 1.0 * self.velocity.x.signum(); // avoid friction atm;
        return Updatable::update(self, surrounding_objects, world_bounds);
    }
    fn draw(&self, camera_x: usize, camera_y: usize) {
        self.animate.draw(
            &self.object.pos,
            self.object.width,
            self.object.height,
            &self.velocity,
            camera_x,
            camera_y,
            None,
        )
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
        let new_x = player_x.saturating_sub(self.width / 4);
        if new_x >= self.x {
            self.x = new_x;
            self.x = self.x.clamp(0, MARIO_WORLD_SIZE.width - self.width);
        }    
        self.y = player_y.saturating_sub(self.height);
    }
}
#[derive(Debug, Clone)]
enum GameEventType {
    GameWon,
    GameOver,
    Kill,
    PlayerHit,
    PlayerPowerUp,
    PlayerHitBlock,
    PlayerHitPowerupBlock,
    EnemyCollEnemy,
}
#[derive(Debug, Clone)]
struct GameEvent {
    event: GameEventType,
    triggered_by: Object,
    target: Option<Object>,
}
#[derive(PartialEq)]
enum GameState {
    Playing,
    GameWon,
    GameOver,
    Frozen(f32),
}
#[derive(Clone, Debug)]
enum ObjectReference {
    Block(usize),
    Enemy(usize), // Index into the self.enemies vector
    Player,
    Powerup(usize),
    None,
}


#[derive(Clone)]
struct PowerUp {
    object: Object,
    velocity: Vec2,
    animate: Animate,
}
impl Updatable for PowerUp {
    fn as_any(&self) -> &dyn Any {
        self
    }
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

    fn set_grounded(&mut self, _: bool) {}

    fn animate(& self) -> & Animate {
        & self.animate
    }
    fn mut_animate(&mut self) -> &mut Animate {
        &mut self.animate
    }
    fn get_collision_handler(&self, other: ObjectType) -> Box<dyn CollisionHandler> {
        match other {
            ObjectType::Block(_) => Box::new(EnemyBlockCollisionHandler), // powerup behaves like enemy
            ObjectType::Enemy(_) => Box::new(EnemyCollisionHandler),
            _ => Box::new(DoNothingCollisionHandler),
        }
    }

    fn handle_world_border(&mut self, world_bounds: WorldBounds) -> Option<GameEvent> {
        if self.object.pos.x < 0.0 {
            self.object.pos.x = 0.0;
            self.velocity.x = 0.0;
        }
        if self.object.pos.x + self.object.width as f32 > world_bounds.max_x as f32 {
            self.object.pos.x = world_bounds.max_x as f32 - self.object.width as f32;
            self.velocity.x = 0.0;
        }
        if self.object.pos.y > world_bounds.max_y as f32 {
            return Some(GameEvent {
                event: GameEventType::Kill,
                triggered_by: self.object.clone(),
                target: None,
            });
        }
        None
    }
    fn update_animation(&mut self) {
        self.animate.update();
    }
}

impl PowerUp {
    fn new(x: usize, y: usize) -> PowerUp {
        let mut powerup = PowerUp {
            object: Object::new(x, y, ObjectType::Powerup),
            velocity: Vec2::new(1.0, 0.0),
            animate: Animate::new(1.0),
        };
        powerup
            .animate
            .change_animation_sprites(POWERUP_SPRITE_LOOKUP.to_vec());
        powerup
    }
    fn update(
        &mut self,
        surrounding_objects: &Vec<SurroundingObject>,
        world_bounds: WorldBounds,
    ) -> Vec<GameEvent> {
        self.velocity.x = 1.0 * self.velocity.x.signum(); // avoid friction atm;
        return Updatable::update(self, surrounding_objects, world_bounds);
    }
    fn draw(&self, camera_x: usize, camera_y: usize) {
        self.animate.draw(
            &self.object.pos,
            self.object.width,
            self.object.height,
            &self.velocity,
            camera_x,
            camera_y,
            None,
        )
    }
}
#[derive(Clone)]
struct Block {
    object: Object,
    texture_id: u8,
    animate: Animate,
}
impl Block {
    fn new_block(x: usize, y: usize, texture_id: u8) -> Block {
        let mut block = Block {
            object: Object::new(x, y, ObjectType::Block(BlockType::Block)),
            animate: Animate::new(1.0),
            texture_id
        };
        block
            .animate
            .change_animation_sprites(vec![SPRITE_ID_TO_TEXTURE2D.get(&texture_id).expect("Invalid texture ID for Block").clone()]);
        block
    }
    fn new_powerup_block(x: usize, y: usize, texture_id: u8) -> Block {
        let mut block = Block {
            object: Object::new(x, y, ObjectType::Block(BlockType::PowerupBlock)),
            animate: Animate::new(1.0),
            texture_id: texture_id
        };
        block
            .animate
            .change_animation_sprites(vec![SPRITE_ID_TO_TEXTURE2D.get(&block.texture_id).expect("Invalid texture ID for Block").clone()]);
        block
    }
    fn transform_into_regular_block(&mut self) {
        self.object.object_type = ObjectType::Block(BlockType::Block);
        self.
        animate
        .change_animation_sprites(vec![SPRITE_ID_TO_TEXTURE2D.get(&10).expect("Invalid texture ID for Block").clone()]);
    }
    fn update(&mut self) {
        self.animate.update();
  
    }
    fn draw(&self, camera_x: usize, camera_y: usize) {
        self.animate.draw(
            &self.object.pos,
            self.object.width,
            self.object.height,
            &Vec2::new(0.0, 0.0),
            camera_x,
            camera_y,
            None,
        )
    }
}
struct World {
    height: usize,
    width: usize,
    objects: Vec<Vec<ObjectReference>>,
    player: Player,
    enemies: Vec<Goomba>,
    powerups: Vec<PowerUp>,
    blocks: Vec<Block>,
    spawning_objects: Vec<SpawningObject>,
    camera: Camera,
    game_state: GameState,
    level_texture: Option<Texture2D>,

    sounds: Option<(Sound, Sound, Sound)>,

}

impl World {
    fn new(height: usize, width: usize) -> World {
        let objects =
            vec![vec![ObjectReference::None; width / MARIO_SPRITE_BLOCK_SIZE as usize]; height];
        World {
            height,
            width,
            objects,
            player: Player::new(48, 176, MAX_VELOCITY_X),
            enemies: Vec::new(),
            powerups: Vec::new(),
            blocks : Vec::new(),
            spawning_objects: Vec::new(),
            camera: Camera::new(600, height),
            game_state: GameState::Playing,
            level_texture: None,


            sounds: None,
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


                if let None = SPRITE_ID_TO_TYPE.get(&tile) { // only draw non Blocks
                    let tile_texture = SPRITE_ID_TO_TEXTURE2D.get(&tile).expect("Couldn't find sprite id in SPRITE_ID_TO_TEXTURE");
                    draw_texture_ex( 
                        &tile_texture,
                        x as f32,
                        y as f32,
                        WHITE,
                        DrawTextureParams::default()
                    );
                }
                else if let Some(object_type) = SPRITE_ID_TO_TYPE.get(&tile) {
                    draw_texture_ex( // draw background behind any Block
                        &tilesheet,
                        x as f32,
                        y as f32,
                        WHITE,
                        DrawTextureParams {
                            source: Some(Rect {
                                x: 0.0,
                                y: 0.0,
                                w: MARIO_SPRITE_BLOCK_SIZE as f32,
                                h: MARIO_SPRITE_BLOCK_SIZE as f32,
                            }),
                            ..Default::default()
                        },
                    );
                    self.add_block(Object::new(x as usize, y as usize, object_type.clone()), *tile);
                }
            }
        }
        draw_text("It's time to save Peach", self.width as f32- 210.0 , self.height as f32 / 2.0 - 25.0, 20.0, WHITE);
        draw_text("Go! ->", self.width as f32- 55.0 , self.height as f32 / 2.0, 20.0, WHITE); 

        set_default_camera();

        let render_texture = render_target_camera.render_target.unwrap().texture;
        self.level_texture = Some(render_texture); // to draw in one call, while keeping compressed json instead of loading a .png

    }

    async fn load_sounds(&mut self){
        let jump_sound = load_sound("sounds/mario_jump.wav")
            .await
            .expect("Failed to load jump sound");
        let overworld_sound = load_sound("sounds/overworld.wav")
            .await
            .expect("Failed to load overworld sound");
        let powerup_sound = load_sound("sounds/powerup.wav")
            .await
            .expect("Failed to load powerup sound");
        self.sounds = Some((
            jump_sound.clone(),
            overworld_sound.clone(),
            powerup_sound.clone(),
        ));
        play_sound(
            &overworld_sound,
            PlaySoundParams {
                looped: true,
                volume: SOUND_VOLUME,
            },
        );
    }
    async fn load_player(&mut self) {
        self.player = Player::new(48, 176, MAX_VELOCITY_X);
    }
    fn load_enemies(&mut self) {
        self.add_object(Object::new(160, 176, ObjectType::Enemy(EnemyType::Goomba)));
        self.add_object(Object::new(224, 176, ObjectType::Enemy(EnemyType::Goomba)));
        self.add_object(Object::new(640, 176, ObjectType::Enemy(EnemyType::Goomba)));
        self.add_object(Object::new(776, 176, ObjectType::Enemy(EnemyType::Goomba)));
        self.add_object(Object::new(876, 176, ObjectType::Enemy(EnemyType::Goomba)));
        self.add_object(Object::new(2648, 176, ObjectType::Enemy(EnemyType::Goomba)));
    }
    fn spawn_powerup(&mut self, object: Object) {
        match object.object_type {
            ObjectType::Powerup => {
                let powerup = PowerUp::new(object.pos.x as usize, object.pos.y as usize);
                self.spawning_objects.push(SpawningObject::new(powerup));
            }
            _ => panic!("Can only spawn powerups with animation"),
        }
    }
    fn add_object(&mut self, object: Object) {
        let x =  (object.pos.x / MARIO_SPRITE_BLOCK_SIZE as f32).round() as usize;
        let y =  (object.pos.y / MARIO_SPRITE_BLOCK_SIZE as f32).round() as usize;
        if y > self.objects.len() - 1 || x > self.objects[y].len() - 1 {
            return;
        }
        let pos = object.pos;
        match object.object_type {
            ObjectType::Enemy(EnemyType::Goomba) => {
                self.enemies
                    .push(Goomba::new(pos.x as usize, pos.y as usize, 2));
            }
            ObjectType::Powerup => {
                self.powerups
                    .push(PowerUp::new(pos.x as usize, pos.y as usize));
            }
            _ => {}
        }
        if let ObjectReference::None = self.objects[y][x] {
            self.objects[y][x] = match object.object_type {
                ObjectType::Enemy(_) => ObjectReference::Enemy(self.enemies.len() - 1),
                ObjectType::Player => ObjectReference::Player,
                ObjectType::Powerup => ObjectReference::Powerup(self.powerups.len()),
                _ => panic!("Trying to add block as regular object!")
            };
        } else {
            println!("Adding object at x {} {}", x*MARIO_SPRITE_BLOCK_SIZE, y*MARIO_SPRITE_BLOCK_SIZE);
            panic!("Tried to add object where: Object already exists");
        }
    }
    fn add_block(&mut self, object: Object, texture_id: u8) {
        assert!(match object.object_type {
            ObjectType::Block(_) => {true},
             _ => {false}
            });
        let x = (object.pos.x / MARIO_SPRITE_BLOCK_SIZE as f32).round() as usize;
        let y = (object.pos.y / MARIO_SPRITE_BLOCK_SIZE as f32).round() as usize;
        if y > self.objects.len() - 1 || x > self.objects[y].len() - 1 {
                return;
            }
        let pos = object.pos;
        match object.object_type {
            ObjectType::Block(BlockType::Block) => {
                self.blocks.push(Block::new_block(pos.x as usize, pos.y as usize, texture_id))
            }
            ObjectType::Block(BlockType::PowerupBlock) => {
                self.blocks.push(Block::new_powerup_block(pos.x as usize, pos.y as usize, texture_id))
            }
            _ => {}
        }
        if let ObjectReference::None = self.objects[y][x] {
            self.objects[y][x] = ObjectReference::Block(self.blocks.len() - 1);
        }
        else {
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
            self.player.jump(
                &self
                    .sounds
                    .as_ref()
                    .expect("Initialize sounds before handling input!")
                    .0,
            );
        }
    }
    fn get_surrounding_objects(
        objects: &Vec<Vec<ObjectReference>>,
        enemies: &Vec<Goomba>,
        powerups: &Vec<PowerUp>,
        blocks: &Vec<Block>,
        object: &Object,
        radius: usize,
    ) -> Vec<SurroundingObject> {

        let directions: Vec<(isize, isize)> = (-(radius as isize)..=radius as isize)
        .flat_map(|dy| (-(radius as isize)..=radius as isize).map(move |dx| (dy, dx)))
        .filter(|&(dy, dx)| dy != 0 || dx != 0) // Exclude the (0, 0) direction (current object position)
        .collect();

        directions
            .iter()
            .filter_map(|(dy, dx)| {
                let new_x = (object.pos.x / MARIO_SPRITE_BLOCK_SIZE as f32).round() as isize + *dx;
                let new_y = (object.pos.y / MARIO_SPRITE_BLOCK_SIZE as f32).round() as isize + *dy;
                if new_y >= 0
                    && new_y < objects.len() as isize
                    && new_x >= 0
                    && new_x < objects[0].len() as isize
                {
                    let reference = objects[new_y as usize][new_x as usize].clone();
                    let relative_direction = (dy.signum(), dx.signum());
                    Some((reference, relative_direction))
                } else {
                    None
                }
            })
            .filter_map(|(reference, relative_direction)| match reference {
                ObjectReference::Block(index) => {
                    if blocks.len() <= index {
                        return None;
                    } 
                    Some(SurroundingObject::new(
                        blocks[index].object.clone(),
                    
                    relative_direction,
                ))},
                ObjectReference::Enemy(index) => {
                    if enemies.len() <= index {
                        return None;
                    }
                    let enemy = &enemies[index];
                    Some( SurroundingObject::new(
                        enemy.object.clone(),
                        relative_direction,
                    ))
                }
                ObjectReference::Player => None,
                ObjectReference::Powerup(powerup_index) => {
                    if powerups.len() <= powerup_index {
                        return None;
                    }
                    let powerup = &powerups[powerup_index];
                    Some(SurroundingObject::new(
                        powerup.object.clone(),
                        relative_direction,
                    ))
                }
                ObjectReference::None => None,
            })
            .collect()
    }
    fn get_the_objects_reference(&self, object: &Object) -> Option<ObjectReference> {
        let obj_idx_x: usize = (object.pos.x / MARIO_SPRITE_BLOCK_SIZE as f32).round() as usize;
        let obj_idx_y = (object.pos.y / MARIO_SPRITE_BLOCK_SIZE as f32).round() as usize;
        if obj_idx_y < self.objects.len() - 1 && obj_idx_x < self.objects[obj_idx_y].len() -1 {
            return Some(
                self.objects[obj_idx_y][obj_idx_x].clone()
            )
        } else {
            return None;
        }
    } 
    fn clear_the_objects_reference(&mut self, object: &Object) {
        let obj_idx_x: usize = (object.pos.x / MARIO_SPRITE_BLOCK_SIZE as f32).round() as usize;
        let obj_idx_y = (object.pos.y / MARIO_SPRITE_BLOCK_SIZE as f32).round() as usize;
        if obj_idx_y < self.objects.len() - 1 && obj_idx_x < self.objects[obj_idx_y].len() -1 {
            self.objects[obj_idx_y][obj_idx_x] = ObjectReference::None;
        }
    }
    fn handle_game_event(&mut self, game_event: GameEvent) {
        match game_event.event {
            GameEventType::GameWon => {
                self.game_state = GameState::GameWon;
            }
            GameEventType::GameOver => {
                self.game_state = GameState::GameOver;
            }
            GameEventType::Kill => {
                if let Some(target) = game_event.target {
                    self.enemies.retain(|enemy| enemy.object != target); // can do more efficient cleaning by swap removal and index from Object reference
                    self.clear_the_objects_reference(&target);
                }
            }
            GameEventType::PlayerHit => { // handled here because it can lead to game over, so we will handle powerup state in general here
                self.player.power_down();
                self.player.apply_gravity();
                let enemy_obj = game_event.triggered_by;
                let enemy_goomba = self
                    .enemies
                    .iter_mut()
                    .find(|enemy| enemy.object == enemy_obj);
                if let Some(enemy) = enemy_goomba {
                    enemy.velocity.x *= -1.0 * self.player.velocity.x.signum();
                    enemy.object.pos =
                        Vec2::new(enemy.object.pos.x + enemy.velocity.x, enemy.object.pos.y);
                }
                self.game_state = GameState::Frozen(2.0);
                match self.player.power_state {
                    PlayerState::Dead => {
                        self.game_state = GameState::GameOver;
                    }
                    _ => {}
                }
            } 
            GameEventType::PlayerPowerUp => {
                self.player.power_up();
                if let Some(target) = game_event.target {
                    self.clear_the_objects_reference(&target);
                    self.powerups.retain(|powerup| powerup.object != target);
                }
                play_sound(
                    &self
                        .sounds
                        .as_ref()
                        .expect("Initialize sounds before handling game event!")
                        .2,
                    PlaySoundParams {
                        volume: SOUND_VOLUME,
                        looped: false,
                    },
                );
            }
            GameEventType::EnemyCollEnemy => {
                if let (Some(target1), target2) = (game_event.target, game_event.triggered_by) {
                    let mut enemy1 = None;
                    let mut enemy2 = None;

                    for enemy in &mut self.enemies {
                        if enemy.object == target1 {
                            enemy1 = Some(enemy);
                        } else if enemy.object == target2 {
                            enemy2 = Some(enemy);
                        }

                        if enemy1.is_some() && enemy2.is_some() {
                            break;
                        }
                    }

                    if let (Some(e1), Some(e2)) = (enemy1, enemy2) {
                        if e1.velocity.x.signum() == e2.velocity.x.signum() {
                            e1.velocity.x *= -1.0;
                        }
                        assert!(e1.velocity.x.signum() != e2.velocity.x.signum());
                    }
                }
            }
            GameEventType::PlayerHitPowerupBlock => {
                if let Some(target) = game_event.target {
                    match target.object_type {
                    ObjectType::Block(BlockType::PowerupBlock) => {
                        let object_ref = self.get_the_objects_reference(&target);
                            match object_ref {
                                Some(ObjectReference::Block(index)) => { 
                                    let block = &mut self.blocks[index];
                                    block.transform_into_regular_block(); 


                                }
                                _ => {}
                            }
                       
                        self.spawn_powerup(Object::new(
                            target.pos.x as usize,
                            target.pos.y as usize- (MARIO_SPRITE_BLOCK_SIZE),
                            ObjectType::Powerup,
                        ));

                    }
                _ => {}
                }
            }
            }
            GameEventType::PlayerHitBlock => {
                if let Some(target) = game_event.target {
                    if target.object_type == ObjectType::Block(BlockType::Block) {
                        let object_ref = self.get_the_objects_reference(&target);
                        match object_ref {
                            Some(ObjectReference::Block(index)) => {
                                let  block = self.blocks[index].borrow_mut();

                                let y = block.object.pos.y;
                                let player_center_x = self.player.object.pos.x + self.player.object.width as f32 / 2.0;
                                if y >= self.player.object.pos.y 
                                || (player_center_x < block.object.pos.x
                                    || player_center_x > block.object.pos.x + block.object.width as f32)  {
                                    return;
                                }
                                let animation = PlayAnimationBuilder::new(block.animate.frames.clone()).pos_offset_frames(
                                    vec![Vec2::new(0.0, -2.0), Vec2::new(0.0, -4.0), Vec2::new(0.0, -6.0), Vec2::new(0.0, -8.0), Vec2::new(0.0, -6.0), Vec2::new(0.0, -4.0), Vec2::new(0.0, -2.0)]).build();
                                block.animate.scale_animation_speed(2.0);
                                block.animate.play_animation(animation);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
    fn update_spawning_objects(&mut self ) {
        let mut completed_spawns = Vec::new();
        for (index, spawning_object) in self.spawning_objects.iter_mut().enumerate() {
            if spawning_object.update() {
                completed_spawns.push(index);
            }
        }
    
        for index in completed_spawns.iter().rev() {
            let spawned_object = self.spawning_objects.swap_remove(*index);
            match spawned_object.object.object().object_type {
                ObjectType::Powerup => {
                    let powerup = spawned_object.object.as_any().downcast_ref::<PowerUp>().expect("Failed to downcast powerup");

                    self.add_object(powerup.object.clone()); 
                }
                _ => {}
            }
        }
    }
    fn update(&mut self) {
        self.update_spawning_objects();
        let mut vec_of_game_events = Vec::new();
        for i in 0..self.enemies.len() {
            let (before, after) = self.enemies.split_at_mut(i);
            let (enemy, after) = &mut after.split_at_mut(1);

            let other_enemies: Vec<Goomba> = before
                .iter_mut()
                .chain(after.iter_mut().skip(1))
                .map(|e| (*e).clone())
                .collect();
            let enemy = &mut enemy[0];
            let surrounding_objects = Self::get_surrounding_objects(
                &self.objects,
                &other_enemies,

                &self.powerups,
                &self.blocks,
                &enemy.object,
        1
            );

            let old_x = (enemy.object.pos.x / MARIO_SPRITE_BLOCK_SIZE as f32).round() as usize;
            let old_y = (enemy.object.pos.y / MARIO_SPRITE_BLOCK_SIZE as f32).round() as usize;

            let game_event = enemy.update(&surrounding_objects, WorldBounds { min_x: 0, max_x: self.width, max_y: self.height });
            vec_of_game_events.push(game_event);

            let new_x = (enemy.object.pos.x / MARIO_SPRITE_BLOCK_SIZE as f32).round() as usize;
            let new_y = (enemy.object.pos.y / MARIO_SPRITE_BLOCK_SIZE as f32).round() as usize;

            if old_x == new_x && old_y == new_y {
                continue;
            }
            if old_y >= self.objects.len() || old_x >= self.objects[old_y].len() {
                continue;
            }
            self.objects[old_y][old_x] = ObjectReference::None;
            if new_y >= self.objects.len() || new_x >= self.objects[new_y].len() {
                continue;
            }
            self.objects[new_y][new_x] = ObjectReference::Enemy(i);
        }
        for i in 0..self.blocks.len() {
            let block = &mut self.blocks[i];
            block.update();
        }
        for i in 0..self.powerups.len() {
            let (before, after) = self.powerups.split_at_mut(i);
            let (powerup, after) = &mut after.split_at_mut(1);

            let other_powerups: Vec<PowerUp> = before
                .iter_mut()
                .chain(after.iter_mut().skip(1))
                .map(|e| (*e).clone())
                .collect();
            let powerup = &mut powerup[0];
            let surrounding_objects = Self::get_surrounding_objects(
                &self.objects,
                &self.enemies,

                &other_powerups,                &self.blocks,
                &powerup.object,
                1,
            );

            let old_x = (powerup.object.pos.x / MARIO_SPRITE_BLOCK_SIZE as f32).round() as usize;
            let old_y = (powerup.object.pos.y / MARIO_SPRITE_BLOCK_SIZE as f32).round() as usize;

            let game_event = powerup.update(&surrounding_objects, WorldBounds { min_x: 0, max_x: self.width, max_y: self.height });
            vec_of_game_events.push(game_event);

            let new_x = (powerup.object.pos.x / MARIO_SPRITE_BLOCK_SIZE as f32).round() as usize;
            let new_y = (powerup.object.pos.y / MARIO_SPRITE_BLOCK_SIZE as f32).round() as usize;

            if old_x == new_x && old_y == new_y {
                continue;
            }
            if old_y >= self.objects.len() || old_x >= self.objects[old_y].len() {
                continue;
            }
            self.objects[old_y][old_x] = ObjectReference::None;
            if new_y >= self.objects.len() || new_x >= self.objects[new_y].len() {
                continue;
            }
            self.objects[new_y][new_x] = ObjectReference::Powerup(i);
        }
        let player_old_x = (self.player.object.pos.x / MARIO_SPRITE_BLOCK_SIZE as f32).round() as usize;
        let player_old_y = (self.player.object.pos.y / MARIO_SPRITE_BLOCK_SIZE as f32).round() as usize;
        self.objects[player_old_y][player_old_x] = ObjectReference::None;
        let player_surrounding_objects: Vec<SurroundingObject> = Self::get_surrounding_objects(
            &self.objects,
            &self.enemies,
                      &self.powerups,&self.blocks,  
            &self.player.object,
            match self.player.power_state {
                PlayerState::Big => 2,
                _ => 1,
            },
        );

        let game_event = self
            .player
            .update(&player_surrounding_objects, WorldBounds { min_x: self.camera.x, max_x: self.width, max_y: self.height });

        vec_of_game_events.push(game_event);

        for game_events in vec_of_game_events {
            for game_event in game_events {
                self.handle_game_event(game_event.clone());
                match game_event.event {
                    GameEventType::GameOver => {
                        return;
                    }
                    GameEventType::GameWon => {
                        return;
                    }
                    _ => {}
                }
            }
        }
        let player_new_x = (self.player.object.pos.x / MARIO_SPRITE_BLOCK_SIZE as f32).round() as usize;
        let player_new_y = (self.player.object.pos.y / MARIO_SPRITE_BLOCK_SIZE as f32).round() as usize;

        if player_new_y >= self.objects.len() || player_new_x >= self.objects[player_new_y].len() {
            return;
        }
        self.objects[player_new_y][player_new_x] = ObjectReference::Player;

        self.camera.update(
            self.player.object.pos.x as usize,
            self.player.object.pos.y as usize,
        );
    }

    fn draw(&self) {
        match self.game_state {
            GameState::GameOver => {
                draw_text(
                    "Game Over",
                    200.0 * SCALE_IMAGE_FACTOR as f32,
                    150.0 * SCALE_IMAGE_FACTOR as f32,
                    40.0,
                    RED,
                );
            }
            GameState::GameWon => {
                draw_text(
                    "You Won!",
                    200.0 * SCALE_IMAGE_FACTOR as f32,
                    150.0 * SCALE_IMAGE_FACTOR as f32,
                    40.0,
                    GREEN,
                );
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
                                (self.camera.width * SCALE_IMAGE_FACTOR) as f32,
                                (self.camera.height * SCALE_IMAGE_FACTOR) as f32,
                            )),
                            flip_y: true,
                            ..Default::default()
                        },
                    );
                    if let GameState::Frozen(frozen_time) = self.game_state {
                        draw_text(
                            &format!("Paused: {:.2}", frozen_time),
                            200.0 * SCALE_IMAGE_FACTOR as f32,
                            150.0 * SCALE_IMAGE_FACTOR as f32,
                            40.0,
                            WHITE,
                        );
                    }
                }
                for spawning_obj in &self.spawning_objects {
                    spawning_obj.draw(self.camera.x, self.camera.y);
                }
                for block in &self.blocks {
                    block.draw(self.camera.x, self.camera.y);
                }
                for enemy in &self.enemies {
                    enemy.draw(self.camera.x, self.camera.y);
                }
                for powerup in &self.powerups {
                    powerup.draw(self.camera.x, self.camera.y);
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
    let mut world = World::new(MARIO_WORLD_SIZE.height, MARIO_WORLD_SIZE.width);

    world.load_sounds().await;
    world.load_level().await;
    world.load_enemies();
    world.load_player().await;

    let mut elapsed_time = 0.0;
    let target_time_step = 1.0 / PHYSICS_FRAME_PER_SECOND;

    loop {
        clear_background(BLACK);

        elapsed_time += get_frame_time();
        while elapsed_time >= target_time_step {
            if let GameState::Frozen(frozen_time) = world.game_state {
                world.game_state = GameState::Frozen(frozen_time - get_frame_time());
                if frozen_time - target_time_step <= 0.0 {
                    world.game_state = GameState::Playing;
                }
                break;
            } else if world.game_state != GameState::Playing {
                break;
            }
            world.handle_input();
            world.update();
            elapsed_time = 0.0;
        }

        world.draw();

        draw_text(&format!("FPS: {}", get_fps()), 10.0, 10.0, 20.0, WHITE);
        next_frame().await;
    }
}