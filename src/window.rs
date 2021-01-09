#[cfg(test)]
use mockall::{automock, predicate::*};

#[cfg_attr(test, automock)]
pub trait Window {
    fn blank_screen(&self);

    /// Draw a sprite on the screen. Return true if a collision has occurred.
    fn draw(&self, x: u8, y: u8, sprite: Vec<u8>) -> bool;

    fn is_key_pressed(&self, key: u8) -> bool;

    fn get_pressed_key(&self) -> Option<u8>;
}

pub struct MiniFbWindow {

}

impl MiniFbWindow {
    pub fn new() -> MiniFbWindow {
        MiniFbWindow {  }
    }
}

impl Window for MiniFbWindow {
    fn blank_screen(&self) {
    }

    fn draw(&self, x: u8, y: u8, sprite: Vec<u8>) -> bool {
        false
    }

    fn is_key_pressed(&self, key: u8) -> bool {
        false
    }

    fn get_pressed_key(&self) -> Option<u8> {
        None
    }
}