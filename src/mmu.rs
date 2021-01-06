#[cfg(test)]
use mockall::{automock, predicate::*};
use std::error::Error;
use std::fs::File;
use std::io::Read;
use ux::u12;

#[cfg_attr(test, automock)]
pub trait Mmu {
    fn read_u8(&self, address: u12) -> u8;
    fn read_u16(&self, address: u12) -> u16;

    fn write_u8(&mut self, address: u12, data: u8);
    fn write_u16(&mut self, address: u12, data: u16);

    fn load_program(&mut self, file_path: &str) -> Result<(), Box<dyn Error>>;
}

pub struct Chip8Mmu {
    memory: Vec<u8>,
}

impl Chip8Mmu {
    // Address of the first instruction
    const PROGRAM_START: usize = 0x200;
    // Total number of bytes available
    const MEM_SIZE: usize = 4096;
    // Number of bytes in each font sprite
    pub const FONT_SPRITE_HEIGHT: u8 = 5;
    // Collection fo characters at a known location
    const FONT_SET: [u8; 80] = [
        0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
        0x20, 0x60, 0x20, 0x20, 0x70, // 1
        0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
        0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
        0x90, 0x90, 0xF0, 0x10, 0x10, // 4
        0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
        0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
        0xF0, 0x10, 0x20, 0x40, 0x40, // 7
        0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
        0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
        0xF0, 0x90, 0xF0, 0x90, 0x90, // A
        0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
        0xF0, 0x80, 0x80, 0x80, 0xF0, // C
        0xE0, 0x90, 0x90, 0x90, 0xE0, // D
        0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
        0xF0, 0x80, 0xF0, 0x80, 0x80, // F
    ];

    pub fn new() -> Chip8Mmu {
        let mut memory = vec![0; Self::MEM_SIZE];

        // Init font data
        for (i, font_data) in Self::FONT_SET.iter().enumerate() {
            memory[i] = *font_data;
        }

        Chip8Mmu { memory }
    }

    fn to_usize(address: u12) -> usize {
        u16::from(address) as usize
    }
}

impl Mmu for Chip8Mmu {
    fn read_u8(&self, address: u12) -> u8 {
        self.memory[Self::to_usize(address)]
    }

    fn read_u16(&self, address: u12) -> u16 {
        ((self.memory[Self::to_usize(address)] as u16) << 8)
            | (self.memory[Self::to_usize(address + u12::new(1))] as u16)
    }

    fn write_u8(&mut self, address: u12, data: u8) {
        self.memory[Self::to_usize(address)] = data;
    }

    fn write_u16(&mut self, address: u12, data: u16) {
        self.memory[Self::to_usize(address)] = (data >> 8) as u8;
        self.memory[Self::to_usize(address + u12::new(1))] = data as u8;
    }

    fn load_program(&mut self, file_path: &str) -> Result<(), Box<dyn Error>> {
        let file = File::open(&file_path)?;

        if file.metadata()?.len() > (Self::MEM_SIZE - Self::PROGRAM_START) as u64 {
            return Err(format!(
                "Memory overflow, program too large. {:?} > {:?}",
                file.metadata()?.len(),
                Self::MEM_SIZE - Self::PROGRAM_START
            )
            .into());
        }

        for (i, data) in file.bytes().enumerate() {
            self.memory[Self::PROGRAM_START + i] = data?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn fonts_are_present() {
        let mmu = Chip8Mmu::new();
        assert_eq!(Chip8Mmu::FONT_SET, mmu.memory[..Chip8Mmu::FONT_SET.len()]);
    }

    #[test]
    fn can_read_u8() {
        let mmu = Chip8Mmu::new();
        assert_eq!(0x20, mmu.read_u8(u12::new(5))); // First byte of "1" font glyph
    }

    #[test]
    fn can_read_u16() {
        let mmu = Chip8Mmu::new();
        assert_eq!(0x2060, mmu.read_u16(u12::new(5))); // First two bytes of "1" font glyph
    }

    #[test]
    fn can_write_u8() {
        let mut mmu = Chip8Mmu::new();
        mmu.write_u8(u12::new(0x200), 0xFE);
        assert_eq!(vec![0xFE], mmu.memory[0x200..0x201]);
    }

    #[test]
    fn can_write_u16() {
        let mut mmu = Chip8Mmu::new();
        mmu.write_u16(u12::new(0x200), 0xFE12);
        assert_eq!(vec![0xFE, 0x12], mmu.memory[0x200..0x202]);
    }

    #[test]
    #[should_panic]
    fn panics_on_read_u16_overflow() {
        let mmu = Chip8Mmu::new();
        mmu.read_u16(u12::new(0xFFF));
    }

    #[test]
    #[should_panic]
    fn panics_on_write_u16_overflow() {
        let mut mmu = Chip8Mmu::new();
        mmu.write_u16(u12::new(0xFFF), 0xFFFF);
    }

    #[test]
    #[allow(unused_must_use)]
    fn should_load_program() {
        let mut mmu = Chip8Mmu::new();

        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("resources/test/test_opcode.ch8");

        mmu.load_program(path.to_str().unwrap());

        assert_eq!(vec![0x12, 0x4E], mmu.memory[0x200..0x202]); // Verify the first two bytes
    }
}
