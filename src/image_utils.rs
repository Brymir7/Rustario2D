use macroquad::{color::Color, texture::Image};

pub fn is_white(color: Color) -> bool {
    color.r == 1.0 && color.g == 1.0 && color.b == 1.0
}

pub fn convert_white_to_transparent(image: &mut Image) {
    for pixel in image.get_image_data_mut().iter_mut() {
        if is_white((*pixel).into()) {
            *pixel = Color::new(0.0, 0.0, 0.0, 0.0).into(); // Transparent color
        }
    }
}
