pub mod animation {

    use macroquad::{math::Vec2, texture::Texture2D};

    #[derive(Clone)]
    pub enum FrameType {
        Height(Vec<usize>),
        Width(Vec<usize>),
        PosOffset(Vec<Vec2>),
    }

    #[derive(Clone)]
    pub struct PlayAnimation {

        pub frame_type: Option<FrameType>,
        pub texture_frames: Vec<Texture2D>,
        pub frame_index: usize,
        pub loop_for: Option<f32>,
    }

    impl PlayAnimation {
        pub fn new(
            frame_type: Option<FrameType>,
            texture_frames: Vec<Texture2D>,
            loop_for: Option<f32>,
        ) -> Self {
        PlayAnimation{
                frame_type,
                texture_frames,
                frame_index: 0,
                loop_for,
            }
        }

        pub fn next_frame(&mut self) -> bool {
            let max_index = match &self.frame_type {
                Some(FrameType::Height(frames)) => frames.len(),
                Some(FrameType::Width(frames)) => frames.len(),
                Some(FrameType::PosOffset(frames)) => frames.len(),
                None => self.texture_frames.len(),
            };

            if self.frame_index + 1 < max_index {
                self.frame_index += 1;
                true
            } else if let Some(loop_for) = self.loop_for {
                if loop_for == 0.0 {
                    return false;
                }
                self.frame_index = 0;
                true
            } else {

                false
            }
        }

    }

pub struct PlayAnimationBuilder {
    loop_for: Option<f32>,
    height_frames: Option<Vec<usize>>,
    width_frames: Option<Vec<usize>>,
    pos_offset_frames: Option<Vec<Vec2>>,
    texture_frames: Vec<Texture2D>,
    frame_index: Option<usize>,
    
}

impl PlayAnimationBuilder {
    pub fn new(texture_frames: Vec<Texture2D>) -> Self {

        Self {
            loop_for: None,
            height_frames: None,
            width_frames: None,
            pos_offset_frames: None,
            texture_frames,
            frame_index: None,

        }
    }
    pub fn loop_for(mut self, loop_for: f32) -> Self {
        self.loop_for = Some(loop_for);
        self
    }
    pub fn height_frames(mut self, frames: Vec<usize>) -> Self {
        self.height_frames = Some(frames);
        self.width_frames = None;
        self.pos_offset_frames = None;
        self.frame_index = Some(0);
        self
    }

    pub fn width_frames(mut self, frames: Vec<usize>) -> Self {
        self.width_frames = Some(frames);
        self.height_frames = None;
        self.pos_offset_frames = None;
        self.frame_index = Some(0);
        self
    }

    pub fn pos_offset_frames(mut self, frames: Vec<Vec2>) -> Self {
        self.pos_offset_frames = Some(frames);
        self.height_frames = None;
        self.width_frames = None;
        self.frame_index = Some(0);
        self
    }


    pub fn build(self) -> PlayAnimation {
        let frame_type = if let Some(frames) = self.height_frames {
            Some(FrameType::Height(frames))
        } else if let Some(frames) = self.width_frames {
            Some(FrameType::Width(frames))
        } else if let Some(frames) = self.pos_offset_frames {
            Some(FrameType::PosOffset(frames))
        } else {
            None
        };

        PlayAnimation {
            frame_type,
            texture_frames: self.texture_frames,
            frame_index: self.frame_index.unwrap_or(0),
            loop_for: self.loop_for,
        }
    }
}

}