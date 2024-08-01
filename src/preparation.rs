use image::{GenericImageView, ImageBuffer, Rgba};
use serde::{Deserialize, Serialize};
use std::fs::{create_dir_all, File};
use std::io::Write;

use crate::mario_config::mario_config::MARIO_SPRITE_BLOCK_SIZE;

#[derive(Serialize, Deserialize)]
pub struct Tile {
    pub start_x: u32,
    pub start_y: u32,
    pub sprite_name: String,
}

pub fn main() {
    let img_path = "level1.png";
    let img = image::open(img_path).expect("Failed to open image");

    let (img_width, img_height) = img.dimensions();

    let mut tiles_map = Vec::<(ImageBuffer<Rgba<u8>, Vec<u8>>, String)>::new();
    let mut level_data = Vec::new();

    create_dir_all("leveldata").expect("Failed to create directory");
    create_dir_all("sprites").expect("Failed to create directory");

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
            let mut sprite_name = String::new();

            for (existing_tile, existing_sprite_name) in tiles_map.iter() {
                if tiles_equal(&tile, existing_tile) {
                    found = true;
                    sprite_name = existing_sprite_name.clone();
                    break;
                }
            }

            if !found {
                sprite_name = format!("sprites/{}.png", tiles_map.len());
                tile.save(&sprite_name).expect("Failed to save sprite");
                tiles_map.push((tile.clone(), sprite_name.clone()));
            }

            level_data.push(Tile {
                start_x: x,
                start_y: y,
                sprite_name,
            });
        }
    }

    let json_data =
        serde_json::to_string_pretty(&level_data).expect("Failed to serialize level data");
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
