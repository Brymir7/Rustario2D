pub mod animation {

    use macroquad::{math::Vec2, texture::Texture2D};

#[derive(Clone)]
pub struct PlayAnimation {
    pub duration: f32,
    pub height_frames: Option<Vec<usize>>,
    pub width_frames: Option<Vec<usize>>,
    pub pos_frames: Option<Vec<Vec2>>,
    pub texture_frames: Vec<Texture2D>,
    
}

pub struct PlayAnimationBuilder {
    duration: f32,
    height_frames: Option<Vec<usize>>,
    width_frames: Option<Vec<usize>>,
    pos_frames: Option<Vec<Vec2>>,
    texture_frames: Vec<Texture2D>,
    
}

impl PlayAnimationBuilder {
    pub fn new(duration: f32, texture_frames: Vec<Texture2D>) -> Self {
        assert!(duration > 0.0);
        Self {
            duration,
            height_frames: None,
            width_frames: None,
            pos_frames: None,
            texture_frames,
            
        }
    }

    pub fn height_frames(mut self, frames: Vec<usize>) -> Self {
        self.height_frames = Some(frames);
        self.width_frames = None;
        self.pos_frames = None;
        self
    }

    pub fn width_frames(mut self, frames: Vec<usize>) -> Self {
        self.width_frames = Some(frames);
        self.height_frames = None;
        self.pos_frames = None;
        self
    }

    pub fn pos_frames(mut self, frames: Vec<Vec2>) -> Self {
        self.pos_frames = Some(frames);
        self.height_frames = None;
        self.width_frames = None;
        self
    }


    pub fn build(self) -> PlayAnimation {
        PlayAnimation {
            duration: self.duration,
            height_frames: self.height_frames,
            width_frames: self.width_frames,
            pos_frames: self.pos_frames,
            texture_frames: self.texture_frames,

        }
    }
}

}