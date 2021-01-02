use std::error::Error;
use std::fs::File;
use std::io::Read;

pub struct Mmu {
    memory: Vec<u8>,
}

impl Mmu {
    // Address of the first instruction
    pub const PROGRAM_START: usize = 0x200;
    // Total number of bytes available
    const MEM_SIZE: usize = 4096;
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

    pub fn new() -> Mmu {
        let mut memory = vec![0; Mmu::MEM_SIZE];

        // Init font data
        for (i, font_data) in Mmu::FONT_SET.iter().enumerate() {
            memory[i] = *font_data;
        }

        Mmu { memory }
    }

    pub fn read_u8(&self, address: usize) -> u8 {
        self.memory[address]
    }

    pub fn read_u16(&self, address: usize) -> u16 {
        ((self.memory[address] as u16) << 8) | (self.memory[address + 1] as u16)
    }

    pub fn write_u8(&mut self, address: usize, data: u8) {
        self.memory[address] = data;
    }

    pub fn write_u16(&mut self, address: usize, data: u16) {
        self.memory[address] = (data >> 8) as u8;
        self.memory[address + 1] = data as u8;
    }

    pub fn load_program(&mut self, file_path: &str) -> Result<(), Box<dyn Error>> {
        let file = File::open(&file_path)?;

        if file.metadata()?.len() > (Mmu::MEM_SIZE - Mmu::PROGRAM_START) as u64 {
            return Err(format!(
                "Memory overflow, program too large. {:?} > {:?}",
                file.metadata()?.len(),
                Mmu::MEM_SIZE - Mmu::PROGRAM_START
            )
            .into());
        }

        for (i, data) in file.bytes().enumerate() {
            self.memory[Mmu::PROGRAM_START + i] = data?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod mmu_tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn fonts_are_present() {
        let mmu = Mmu::new();
        assert_eq!(Mmu::FONT_SET, mmu.memory[..Mmu::FONT_SET.len()]);
    }

    #[test]
    fn can_read_u8() {
        let mmu = Mmu::new();
        assert_eq!(0x20, mmu.read_u8(5)); // First byte of "1" font glyph
    }

    #[test]
    fn can_read_u16() {
        let mmu = Mmu::new();
        assert_eq!(0x2060, mmu.read_u16(5)); // First two bytes of "1" font glyph
    }

    #[test]
    fn can_write_u8() {
        let mut mmu = Mmu::new();
        mmu.write_u8(0x200, 0xFE);
        assert_eq!(vec![0xFE], mmu.memory[0x200..0x201]);
    }

    #[test]
    fn can_write_u16() {
        let mut mmu = Mmu::new();
        mmu.write_u16(0x200, 0xFE12);
        assert_eq!(vec![0xFE, 0x12], mmu.memory[0x200..0x202]);
    }

    #[test]
    #[should_panic]
    fn panics_on_read_u8_overflow() {
        let mmu = Mmu::new();
        mmu.read_u8(Mmu::MEM_SIZE + 1);
    }

    #[test]
    #[should_panic]
    fn panics_on_read_u16_overflow() {
        let mmu = Mmu::new();
        mmu.read_u8(Mmu::MEM_SIZE);
    }

    #[test]
    #[should_panic]
    fn panics_on_write_u8_overflow() {
        let mut mmu = Mmu::new();
        mmu.write_u8(Mmu::MEM_SIZE + 1, 0x01);
    }

    #[test]
    #[should_panic]
    fn panics_on_write_u16_overflow() {
        let mut mmu = Mmu::new();
        mmu.write_u16(Mmu::MEM_SIZE, 0xFFFF);
    }

    #[test]
    #[allow(unused_must_use)]
    fn should_load_program() {
        let mut mmu = Mmu::new();

        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("resources/test/test_opcode.ch8");

        mmu.load_program(path.to_str().unwrap());

        assert_eq!(vec![0x12, 0x4E], mmu.memory[0x200..0x202]); // Verify the first two bytes
    }
}
