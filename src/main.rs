use macroquad::prelude::Image;
use macroquad::prelude::*;
pub mod preparation;
const MARIO_SPRITE_BLOCK_SIZE: usize = 16;
const MARIO_WORLD_SIZE: (u16, u16) = (430, 240);

struct Sprite {
    height: usize,
    width: usize,
    pixels: Vec<Color>,
}
const fn get_block_sprite() -> Sprite {
    let image = Image::from_file_with_format(
        include_bytes!("../sprites/sprite_0_208.png"),
        Some(ImageFormat::Png),
    )
    .unwrap();
    let mut pixels: Vec<Color> = vec![];
    for y in 0..MARIO_SPRITE_BLOCK_SIZE {
        for x in 0..MARIO_SPRITE_BLOCK_SIZE {
            pixels.push(image.get_pixel(x as u32, y as u32));
        }
    }
    Sprite {
        width: MARIO_SPRITE_BLOCK_SIZE,
        height: MARIO_SPRITE_BLOCK_SIZE,
        pixels,
    }
}
const BLOCK_WALL_SPRITE: Sprite = get_block_sprite();

#[derive(PartialEq)]
enum BlockType {
    Wall,
    Ground,
    MovementBlock,
}
#[derive(PartialEq)]
enum EnemyType {
    Goomba,
    Koopa,
}
#[derive(PartialEq)]
enum ObjectType {
    Block(BlockType),
    Enemy(EnemyType),
    Player,
    PowerUp,
}

struct Object {
    x: u16,
    y: u16,
    sprite: Sprite,
    object: ObjectType,
    velocity: (i16, i16),
}
impl Object {
    fn new(x: u16, y: u16, sprite: Sprite, object: ObjectType) -> Object {
        Object {
            x,
            y,
            sprite,
            object,
            velocity: (0, 0),
        }
    }
    fn set_velocity(&mut self, velocity: (i16, i16)) {
        self.velocity = velocity;
    }
    fn update(&mut self) {
        self.x = (self.x as i16 + self.velocity.0) as u16;
        self.y = (self.y as i16 + self.velocity.1) as u16;
    }
    fn draw(&self) {
        for (i, pixel) in self.sprite.pixels.iter().enumerate() {
            let x = (i % self.sprite.width) as u16 + self.x;
            let y = (i / self.sprite.height) as u16 + self.y;
            draw_rectangle(x as f32, y as f32, 1.0, 1.0, *pixel);
        }
    }
}
impl PartialEq for Object {
    fn eq(&self, other: &Self) -> bool {
        return self.x == other.x && self.y == other.y && self.object == other.object;
    }
}
struct World {
    height: u16,
    width: u16,
    objects: Vec<Object>,
}
impl World {
    fn new(height: u16, width: u16) -> World {
        World {
            height,
            width,
            objects: Vec::new(),
        }
    }
    fn add_object(&mut self, object: Object) {
        self.objects.push(object);
    }
    fn remove_object(&mut self, object: Object) {
        self.objects.retain(|x| x != &object);
    }
    fn update(&mut self) {}
    fn draw(&self) {}
}
#[macroquad::main("Rustario Bros")]
async fn main() {
    preparation::main(); // Creates the sprites to be used
    let mut world = World::new(MARIO_WORLD_SIZE.0, MARIO_WORLD_SIZE.1);
    let block_obj = Object::new(
        0,
        0,
        BLOCK_WALL_SPRITE,
        ObjectType::Block(BlockType::Ground),
    );
    world.add_object(block_obj);
    loop {
        clear_background(BLACK);
        world.update();
        world.draw();
        next_frame().await;
    }
}
