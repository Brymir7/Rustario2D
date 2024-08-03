use image::{GenericImageView, ImageBuffer, Rgba};
use serde::{Deserialize, Serialize};
use std::fs::{create_dir_all, File};
use std::io::Write;

use crate::mario_config::mario_config::MARIO_SPRITE_BLOCK_SIZE;

pub struct Tile {
    pub sprite_id: usize,
}

#[derive(Serialize, Deserialize)]
pub struct LevelData {
    pub height: usize,
    pub tiles: Vec<usize>,
}

pub fn main() {
    let img_path = "level1.png";
    let img = image::open(img_path).expect("Failed to open image");

    let (img_width, img_height) = img.dimensions();

    let mut tiles_map = Vec::<ImageBuffer<Rgba<u8>, Vec<u8>>>::new();
    let mut level_data = Vec::new();

    create_dir_all("leveldata").expect("Failed to create directory");

    for y in (0..img_height).step_by(MARIO_SPRITE_BLOCK_SIZE) {
        for x in (0..img_width).step_by(MARIO_SPRITE_BLOCK_SIZE) {
            let tile = img
                .view(
                    x,
                    y,
                    MARIO_SPRITE_BLOCK_SIZE.try_into().unwrap(),
                    MARIO_SPRITE_BLOCK_SIZE.try_into().unwrap(),
                )
                .to_image();
            let mut found = false;
            let mut sprite_id = 0;

            for (i, existing_tile) in tiles_map.iter().enumerate() {
                if tiles_equal(&tile, existing_tile) {
                    found = true;
                    sprite_id = i;
                    break;
                }
            }

            if !found {
                sprite_id = tiles_map.len();
                tiles_map.push(tile.clone());
            }

            level_data.push(Tile { sprite_id });
        }
    }

    let tilesheet_width = MARIO_SPRITE_BLOCK_SIZE;
    let tilesheet_height = MARIO_SPRITE_BLOCK_SIZE * tiles_map.len();
    let mut tilesheet = ImageBuffer::new(tilesheet_width as u32, tilesheet_height as u32);

    for (i, tile) in tiles_map.iter().enumerate() {
        let y_offset = i as u32 * MARIO_SPRITE_BLOCK_SIZE as u32;

        for y in 0..MARIO_SPRITE_BLOCK_SIZE {
            for x in 0..MARIO_SPRITE_BLOCK_SIZE {
                let pixel = tile.get_pixel(x as u32, y as u32);
                tilesheet.put_pixel(x as u32, y as u32 + y_offset, *pixel);
            }
        }
    }

    tilesheet
        .save("sprites/tilesheet.png")
        .expect("Failed to save tilesheet");

    let level_data_json = LevelData {
        height: img_height as usize,
        tiles: level_data.iter().map(|t| t.sprite_id).collect(),
    };

    let json_data =
        serde_json::to_string_pretty(&level_data_json).expect("Failed to serialize level data");
    let mut file = File::create("leveldata/level_data.json").expect("Failed to create file");
    file.write_all(json_data.as_bytes())
        .expect("Failed to write to file");
}

fn tiles_equal(
    tile1: &ImageBuffer<Rgba<u8>, Vec<u8>>,
    tile2: &ImageBuffer<Rgba<u8>, Vec<u8>>,
) -> bool {
    tile1.pixels().zip(tile2.pixels()).all(|(p1, p2)| p1 == p2)
}
