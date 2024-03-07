use macroquad::prelude::Image;
use macroquad::prelude::*;

const MARIO_SPRITE_BLOCK_SIZE: usize = 16;
struct Sprite {
    x: usize,
    y: usize,
    pixels: Vec<Color>,
}
impl PartialEq for Sprite {
    fn eq(&self, other: &Self) -> bool {
        self.pixels == other.pixels
    }
}
impl Sprite {
    fn serialize_to_png(&self, png_dir: &str) {
        let file_name = format!("{}/sprite_{}_{}.png", png_dir, self.x, self.y);
        let mut image = Image::gen_image_color(
            MARIO_SPRITE_BLOCK_SIZE as u16,
            MARIO_SPRITE_BLOCK_SIZE as u16,
            WHITE,
        );
        image.update(self.pixels.as_slice());
        image.export_png(&file_name);
    }
}
fn sprite_is_in_vec(sprite: &Sprite, vec: &Vec<Sprite>) -> bool {
    for s in vec {
        if s == sprite {
            return true;
        }
    }
    false
}

pub fn main() {
    let mario_world_jpeg = match Image::from_file_with_format(
        include_bytes!("../SuperMarioBros.png"),
        Some(ImageFormat::Png),
    ) {
        Ok(image) => image,
        Err(e) => panic!("Error loading image: {:?}", e),
    };
    let mut mario_sprites: Vec<Sprite> = Vec::new();
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
            if sprite_is_in_vec(&sprite, &mario_sprites) {
                println!("Sprite already in vec");
                continue;
            }
            mario_sprites.push(sprite);
        }
    }
    println!("Mario sprites: {:?}", mario_sprites.len());
    for sprite in mario_sprites.iter() {
        sprite.serialize_to_png("sprites");
    }
}
