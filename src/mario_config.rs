pub mod mario_config {
    pub const MARIO_SPRITE_BLOCK_SIZE: usize = 16;
    pub const GRAVITY: usize = 16;
    pub struct WorldDimensions {
        pub width: usize,
        pub height: usize,
    }

    pub const MARIO_WORLD_SIZE: WorldDimensions = WorldDimensions {
        width: 3392,
        height: 224,
    };
    pub const PHYSICS_FRAME_PER_SECOND: f32 = 60.0;
    pub const PHYSICS_FRAME_TIME: f32 = 1.0 / PHYSICS_FRAME_PER_SECOND;
    pub const MAX_VELOCITY_X: i32 = 3;
    pub const ACCELERATION: f32 = 3.0;
    pub const JUMP_STRENGTH: f32 = 12.0;
}
