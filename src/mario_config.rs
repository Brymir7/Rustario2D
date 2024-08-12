pub mod mario_config {

    pub const GRAVITY: usize = 16;
    pub struct WorldDimensions {
        pub width: usize,
        pub height: usize,
    }
    pub const MARIO_NON_MUSIC_VOLUME: f32 = 0.1;
    pub const SOUND_VOLUME: f32 = 0.3;
    pub const SCALE_IMAGE_FACTOR: usize = 2;
    pub const MARIO_SPRITE_BLOCK_SIZE: usize = 16;
    pub const MARIO_WORLD_SIZE: WorldDimensions = WorldDimensions {
        width: 3392,
        height: 224,
    };
    pub const PHYSICS_FRAME_PER_SECOND: f32 = 60.0;
    pub const PHYSICS_FRAME_TIME: f32 = 1.0 / PHYSICS_FRAME_PER_SECOND;
    pub const MAX_VELOCITY_X: f32 = 2.8;
    pub const ACCELERATION: f32 = 3.0;
    pub const JUMP_STRENGTH: f32 = 12.0;
}
