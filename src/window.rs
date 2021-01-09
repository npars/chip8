use minifb::WindowOptions;
#[cfg(test)]
use mockall::{automock, predicate::*};
use std::process;

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
    is_dirty: bool,
}

impl MiniFbWindow {
    const SPRITE_WIDTH: usize = 8;
    const WIDTH: usize = 64;
    const HEIGHT: usize = 32;
    const BUFFER_SIZE: usize = Self::WIDTH * Self::HEIGHT;

    const PIXEL_HI: u32 = 0x00FFBF00u32;
    const PIXEL_LO: u32 = 0x00000000u32;
    const PIXEL_MAP: [u32; 2] = [Self::PIXEL_LO, Self::PIXEL_HI];
    const KEY_MAP: [minifb::Key; 16] = [
        minifb::Key::X,    // 0
        minifb::Key::Key1, // 1
        minifb::Key::Key2, // 2
        minifb::Key::Key3, // 3
        minifb::Key::Q,    // 4
        minifb::Key::W,    // 5
        minifb::Key::E,    // 6
        minifb::Key::A,    // 7
        minifb::Key::S,    // 8
        minifb::Key::D,    // 9
        minifb::Key::Z,    // A
        minifb::Key::C,    // B
        minifb::Key::Key4, // C
        minifb::Key::R,    // D
        minifb::Key::F,    // E
        minifb::Key::V,    // F
    ];

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
        let buffer = vec![0; Self::BUFFER_SIZE];
        MiniFbWindow {
            window,
            buffer,
            is_dirty: false,
        }
    }
}

impl Window for MiniFbWindow {
    fn blank_screen(&mut self) {
        for i in 0..Self::BUFFER_SIZE {
            self.buffer[i] = Self::PIXEL_LO;
        }
        self.is_dirty = true;
    }

    fn draw(&mut self, x: u8, y: u8, sprite: Vec<u8>) -> bool {
        let (x, y) = (x as usize, y as usize);
        let mut collision = false;
        for (y_offset, row) in sprite.iter().enumerate() {
            for x_offset in 0..Self::SPRITE_WIDTH {
                if (x_offset + x) >= Self::WIDTH || (y_offset + y) >= Self::HEIGHT {
                    continue;
                }

                let pixel =
                    Self::PIXEL_MAP[((row >> (Self::SPRITE_WIDTH - x_offset - 1)) & 0x1) as usize];
                let pixel_index = (x + x_offset + ((y + y_offset) * Self::WIDTH)) as usize;
                if pixel == Self::PIXEL_HI {
                    if self.buffer[pixel_index] == Self::PIXEL_HI {
                        self.buffer[pixel_index] = Self::PIXEL_LO;
                        collision = true;
                    } else {
                        self.buffer[pixel_index] = Self::PIXEL_HI;
                    }
                }
            }
        }
        self.is_dirty = true;
        collision
    }

    fn render(&mut self) {
        if !self.window.is_open() {
            process::exit(0);
        }

        if self.is_dirty {
            self.window
                .update_with_buffer(&self.buffer, Self::WIDTH, Self::HEIGHT)
                .expect("Failed to update window");
        } else {
            self.window.update();
        }
    }

    fn is_key_pressed(&self, key: u8) -> bool {
        self.window.is_key_down(Self::KEY_MAP[key as usize])
    }

    fn get_pressed_key(&self) -> Option<u8> {
        for (key_val, key) in Self::KEY_MAP.iter().enumerate() {
            if self.window.is_key_down(*key) {
                return Some(key_val as u8);
            }
        }
        None
    }
}
