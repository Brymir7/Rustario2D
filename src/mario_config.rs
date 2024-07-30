pub mod mario_config {
    pub const MARIO_SPRITE_BLOCK_SIZE: usize = 16;

    pub struct WorldDimensions {
        pub width: u16,
        pub height: u16,
    }

    pub const MARIO_WORLD_SIZE: WorldDimensions = WorldDimensions {
        width: 600,
        height: 240,
    };
}
