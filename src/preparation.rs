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
    for block_y in (0..mario_world_jpeg.height() - 16 - 1).step_by(MARIO_SPRITE_BLOCK_SIZE) {
        let block_x = block_y;
        let mut pixels: Vec<Color> = vec![];
        for y in (0..MARIO_SPRITE_BLOCK_SIZE).rev() {
            for x in 0..MARIO_SPRITE_BLOCK_SIZE {
                pixels.push(mario_world_jpeg.get_pixel((block_x + x) as u32, (block_y + y) as u32));
            }
        }
        let sprite = Sprite {
            x: block_x,
            y: block_y,
            pixels: pixels,
        };
        if sprite_is_in_vec(&sprite, &mario_sprites) {
            continue;
        }
        mario_sprites.push(sprite);
    }

    println!("Mario sprites: {:?}", mario_sprites.len());
    for sprite in mario_sprites.iter() {
        sprite.serialize_to_png("sprites");
    }
    let mario_character_spritesheet = Image::from_file_with_format(
        include_bytes!("../sprites/Mario/MarioSprites.png"),
        Some(ImageFormat::Png),
    )
    .unwrap();
    let mut mario_character_sprites: Vec<Sprite> = Vec::new();
    let block_x = 128;
    let block_y = 128;
    let mut pixels: Vec<Color> = vec![];
    for x in 0..block_x {
        for y in 0..block_y {
            if x > mario_character_spritesheet.width() as usize
                || y > mario_character_spritesheet.height() as usize
            {
                continue;
            }
            pixels.push(mario_character_spritesheet.get_pixel(x as u32, y as u32));
        }
    }
    let sprite = Sprite {
        x: 0,
        y: 0,
        pixels: pixels,
    };
    mario_character_sprites.push(sprite);
    println!(
        "Mario character sprites: {:?}",
        mario_character_sprites.len()
    );
    for sprite in mario_character_sprites.iter() {
        sprite.serialize_to_png("sprites/Mario");
    }
}
