#[cfg(test)]
use mockall::{automock, predicate::*};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::rc::Rc;
use ux::u12;

#[cfg_attr(test, automock)]
pub trait Mmu {
    fn read_u8(&self, address: usize) -> u8;
    fn read_u16(&self, address: usize) -> u16;

    fn write_u8(&mut self, address: usize, data: u8);
    fn write_u16(&mut self, address: usize, data: u16);

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
}

impl Mmu for Chip8Mmu {
    fn read_u8(&self, address: usize) -> u8 {
        self.memory[address]
    }

    fn read_u16(&self, address: usize) -> u16 {
        ((self.memory[address] as u16) << 8) | (self.memory[address + 1] as u16)
    }

    fn write_u8(&mut self, address: usize, data: u8) {
        self.memory[address] = data;
    }

    fn write_u16(&mut self, address: usize, data: u16) {
        self.memory[address] = (data >> 8) as u8;
        self.memory[address + 1] = data as u8;
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
mod mmu_tests {
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
        assert_eq!(0x20, mmu.read_u8(5)); // First byte of "1" font glyph
    }

    #[test]
    fn can_read_u16() {
        let mmu = Chip8Mmu::new();
        assert_eq!(0x2060, mmu.read_u16(5)); // First two bytes of "1" font glyph
    }

    #[test]
    fn can_write_u8() {
        let mut mmu = Chip8Mmu::new();
        mmu.write_u8(0x200, 0xFE);
        assert_eq!(vec![0xFE], mmu.memory[0x200..0x201]);
    }

    #[test]
    fn can_write_u16() {
        let mut mmu = Chip8Mmu::new();
        mmu.write_u16(0x200, 0xFE12);
        assert_eq!(vec![0xFE, 0x12], mmu.memory[0x200..0x202]);
    }

    #[test]
    #[should_panic]
    fn panics_on_read_u8_overflow() {
        let mmu = Chip8Mmu::new();
        mmu.read_u8(Chip8Mmu::MEM_SIZE + 1);
    }

    #[test]
    #[should_panic]
    fn panics_on_read_u16_overflow() {
        let mmu = Chip8Mmu::new();
        mmu.read_u8(Chip8Mmu::MEM_SIZE);
    }

    #[test]
    #[should_panic]
    fn panics_on_write_u8_overflow() {
        let mut mmu = Chip8Mmu::new();
        mmu.write_u8(Chip8Mmu::MEM_SIZE + 1, 0x01);
    }

    #[test]
    #[should_panic]
    fn panics_on_write_u16_overflow() {
        let mut mmu = Chip8Mmu::new();
        mmu.write_u16(Chip8Mmu::MEM_SIZE, 0xFFFF);
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

pub struct Cpu {
    mmu: Rc<RefCell<dyn Mmu>>,
    video: Rc<RefCell<dyn Video>>,
    registers: Vec<u8>,
    index: u12,
    program_counter: u12,
    delay_timer: u8,
    sound_timer: u8,
    stack: VecDeque<u12>,
}

impl Cpu {
    const OPCODE_SIZE: u16 = 2;
    const REGISTER_SIZE: usize = 16;
    const STACK_SIZE: usize = 16;
    const FUNC_MAP: [fn(&mut Self, u12) -> Option<u12>; 1] = [Self::opcode_0];

    pub fn new(mmu: Rc<RefCell<dyn Mmu>>, video: Rc<RefCell<dyn Video>>) -> Cpu {
        Cpu {
            mmu,
            video,
            registers: vec![0; Cpu::REGISTER_SIZE],
            index: u12::new(0),
            program_counter: u12::new(0x200),
            delay_timer: 0,
            sound_timer: 0,
            stack: VecDeque::with_capacity(Cpu::STACK_SIZE),
        }
    }

    pub fn run_cycle(&self) {}

    fn exec_opcode(&mut self, opcode: u16) {
        // Run the opcode, then update the program_counter
        match Cpu::FUNC_MAP[(opcode >> 12) as usize](self, u12::new(opcode & 0xFFF)) {
            Some(program_counter) => self.program_counter = program_counter,
            None => {
                self.program_counter = self
                    .program_counter
                    .wrapping_add(u12::new(Cpu::OPCODE_SIZE))
            }
        }
    }

    fn opcode_0(&mut self, data: u12) -> Option<u12> {
        match u16::from(data) {
            // Blank Screen
            0x0E0 => {
                self.video.borrow_mut().blank_screen();
                None
            }
            // Return from subroutine
            0x0EE => Some(
                self.stack
                    .pop_back()
                    .unwrap_or_else(|| panic!("Stack underflow!")),
            ),
            // Unhandled: Call machine code routine
            _ => panic!("Unhandled machine code routine instruction"),
        }
    }
}

#[cfg(test)]
mod cpu_tests {
    use super::*;
    use rstest::*;

    #[fixture]
    fn mmu() -> Rc<RefCell<MockMmu>> {
        Rc::new(RefCell::new(MockMmu::new()))
    }

    #[fixture]
    fn video() -> Rc<RefCell<MockVideo>> {
        Rc::new(RefCell::new(MockVideo::new()))
    }

    #[rstest]
    fn pc_has_default(video: Rc<RefCell<MockVideo>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), video.clone());
        assert_eq!(u12::new(0x200), cpu.program_counter);
    }

    #[rstest]
    fn blanks_screen(video: Rc<RefCell<MockVideo>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), video.clone());
        video.borrow_mut().expect_blank_screen().returning(|| ());

        cpu.exec_opcode(0x00E0);

        assert_eq!(u12::new(0x202), cpu.program_counter);
    }
}

#[cfg_attr(test, automock)]
pub trait Video {
    fn blank_screen(&self);
}
