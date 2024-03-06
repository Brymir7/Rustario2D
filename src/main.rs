use core::hash;
use std::collections::HashMap;

use macroquad::prelude::Image;
use macroquad::prelude::*;
use std::fs::File;
use std::io::BufWriter;

const MARIO_SPRITE_BLOCK_SIZE: usize = 16;
struct Sprite {
    x: usize,
    y: usize,
    pixels: Vec<Color>,
}
fn main() {
    let mut mario_sprites: HashMap<Vec<Color>, Sprite> = HashMap::new();
    let mario_world_jpeg = match Image::from_file_with_format(
        include_bytes!("../sprites/SuperMarioBrosMap1-1.png"),
        Some(ImageFormat::Png),
    ) {
        Ok(image) => image,
        Err(e) => panic!("Error loading image: {:?}", e),
    };
    for block_y in (0..mario_world_jpeg.height()).step_by(MARIO_SPRITE_BLOCK_SIZE) {
        for block_x in (0..mario_world_jpeg.width()).step_by(MARIO_SPRITE_BLOCK_SIZE) {
            let mut pixels: Vec<Color> = vec![];
            for y in 0..MARIO_SPRITE_BLOCK_SIZE {
                for x in 0..MARIO_SPRITE_BLOCK_SIZE {
                    if x + block_x >= mario_world_jpeg.width() as usize
                        || y + block_y >= mario_world_jpeg.height() as usize
                    {
                        continue;
                    }
                    pixels.push(
                        mario_world_jpeg.get_pixel((block_x + x) as u32, (block_y + y) as u32),
                    );
                }
            }
            let sprite = Sprite {
                x: block_x,
                y: block_y,
                pixels: pixels,
            };
            mario_sprites.insert(format!("{}-{}", block_x, block_y), sprite);
        }
    }
    println!("Mario sprites: {:?}", mario_sprites.len());
}
