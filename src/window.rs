use minifb::WindowOptions;
#[cfg(test)]
use mockall::{automock, predicate::*};

#[cfg_attr(test, automock)]
pub trait Window {
    fn blank_screen(&mut self);

    /// Draw a sprite on the screen. Return true if a collision has occurred.
    fn draw(&mut self, x: u8, y: u8, sprite: Vec<u8>) -> bool;

    fn render(&mut self);

    fn is_key_pressed(&self, key: u8) -> bool;

    fn get_pressed_key(&self) -> Option<u8>;
}

pub struct MiniFbWindow {
    window: minifb::Window,
    buffer: Vec<u32>,
}

impl MiniFbWindow {
    const SPRITE_WIDTH: usize = 8;
    const WIDTH: usize = 64;
    const HEIGHT: usize = 32;
    const BUFFER_SIZE: usize = Self::WIDTH * Self::HEIGHT;

    const PIXEL_HI: u32 = 0x00FFFFFFu32;
    const PIXEL_LO: u32 = 0x00000000u32;
    const PIXEL_MAP: [u32; 2] = [Self::PIXEL_LO, Self::PIXEL_HI];

    pub fn new() -> MiniFbWindow {
        let mut window = minifb::Window::new(
            "Chip8",
            Self::WIDTH,
            Self::HEIGHT,
            WindowOptions {
                scale: minifb::Scale::X8,
                scale_mode: minifb::ScaleMode::AspectRatioStretch,
                resize: true,
                ..WindowOptions::default()
            },
        )
        .expect("Unable to open Window");
        window.update();
        let buffer = vec![0; Self::WIDTH * Self::HEIGHT];
        MiniFbWindow { window, buffer }
    }
}

impl Window for MiniFbWindow {
    fn blank_screen(&mut self) {
        for i in 0..Self::BUFFER_SIZE {
            self.buffer[i] = Self::PIXEL_LO;
        }
    }

    fn draw(&mut self, x: u8, y: u8, sprite: Vec<u8>) -> bool {
        println!("x:{:?}, y:{:?}", x, y);
        let (x, y) = (x as usize, y as usize);
        let mut collision = false;
        for (y_offset, row) in sprite.iter().enumerate() {
            for x_offset in 0..Self::SPRITE_WIDTH {
                let pixel =
                    Self::PIXEL_MAP[((row >> (Self::SPRITE_WIDTH - x_offset - 1)) & 0x1) as usize];
                let pixel_index = (x + x_offset + ((y + y_offset) * Self::WIDTH)) as usize;
                if pixel == self.buffer[pixel_index] {
                    if self.buffer[pixel_index] == Self::PIXEL_HI {
                        collision = true;
                    }
                    self.buffer[pixel_index] = Self::PIXEL_LO;
                } else {
                    self.buffer[pixel_index] = Self::PIXEL_HI;
                }
            }
        }
        collision
    }

    fn render(&mut self) {
        self.window
            .update_with_buffer(&self.buffer, Self::WIDTH, Self::HEIGHT)
            .expect("Failed to update window");
    }

    fn is_key_pressed(&self, _key: u8) -> bool {
        false
    }

    fn get_pressed_key(&self) -> Option<u8> {
        None
    }
}
