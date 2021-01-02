use crate::mmu::Mmu;
use crate::video::Video;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use ux::u12;

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

    pub fn run_cycle(&mut self) {
        let opcode = self.mmu.borrow().read_u16(self.program_counter);
        self.exec_opcode(opcode);
    }

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
mod tests {
    use super::*;
    use crate::mmu::MockMmu;
    use crate::video::MockVideo;
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
        let cpu = Cpu::new(mmu.clone(), video.clone());
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
