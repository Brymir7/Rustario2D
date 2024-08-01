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
}
