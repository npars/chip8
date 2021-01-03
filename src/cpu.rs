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
    const CARRY_REGISTER: usize = 0xF;
    const FUNC_MAP: [fn(&mut Self, u12) -> Option<u12>; 16] = [
        Self::opcode_0,
        Self::opcode_1,
        Self::opcode_2,
        Self::opcode_3,
        Self::opcode_4,
        Self::opcode_5,
        Self::opcode_6,
        Self::opcode_7,
        Self::opcode_8,
        Self::opcode_9,
        Self::opcode_a,
        Self::opcode_b,
        Self::opcode_c,
        Self::opcode_d,
        Self::opcode_e,
        Self::opcode_f,
    ];

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
                    .wrapping_add(u12::new(Self::OPCODE_SIZE))
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

    fn opcode_1(&mut self, data: u12) -> Option<u12> {
        // Jump to address
        Some(data)
    }

    fn opcode_2(&mut self, data: u12) -> Option<u12> {
        // Call subroutine
        self.stack.push_back(
            self.program_counter
                .wrapping_add(u12::new(Self::OPCODE_SIZE)),
        );
        Some(data)
    }

    fn opcode_3(&mut self, data: u12) -> Option<u12> {
        // Skips the next instruction if VX equals NN.
        let (reg_index, value) = Self::split_xnn(data);
        if self.registers[reg_index as usize] == value {
            Some(
                self.program_counter
                    .wrapping_add(u12::new(Self::OPCODE_SIZE * 2)),
            )
        } else {
            None
        }
    }

    fn opcode_4(&mut self, data: u12) -> Option<u12> {
        // Skips the next instruction if VX doesn't equal NN.
        let (reg_index, value) = Self::split_xnn(data);
        if self.registers[reg_index as usize] != value {
            Some(
                self.program_counter
                    .wrapping_add(u12::new(Self::OPCODE_SIZE * 2)),
            )
        } else {
            None
        }
    }

    fn opcode_5(&mut self, data: u12) -> Option<u12> {
        // Skips the next instruction if VX equals VY
        let data = u16::from(data);
        let reg_x_index = ((data & 0xF00) >> 8) as usize;
        let reg_y_index = ((data & 0x0F0) >> 4) as usize;
        if self.registers[reg_x_index as usize] == self.registers[reg_y_index as usize] {
            Some(
                self.program_counter
                    .wrapping_add(u12::new(Self::OPCODE_SIZE * 2)),
            )
        } else {
            None
        }
    }

    fn opcode_6(&mut self, data: u12) -> Option<u12> {
        // Sets VX to NN
        let (reg_index, value) = Self::split_xnn(data);
        self.registers[reg_index as usize] = value;
        None
    }

    fn opcode_7(&mut self, data: u12) -> Option<u12> {
        // Adds NN to VX. (Carry flag is not changed)
        let (reg_index, value) = Self::split_xnn(data);
        self.registers[reg_index as usize] = self.registers[reg_index as usize].wrapping_add(value);
        None
    }

    fn opcode_8(&mut self, data: u12) -> Option<u12> {
        let (x, y, opcode) = Self::split_xyn(data);
        let x = x as usize;
        let y = y as usize;
        match opcode {
            // Sets VX to the value of VY.
            0x0 => self.registers[x] = self.registers[y],
            // Sets VX to VX or VY. (Bitwise OR operation)
            0x1 => self.registers[x] |= self.registers[y],
            // Sets VX to VX and VY. (Bitwise AND operation)
            0x2 => self.registers[x] &= self.registers[y],
            // Sets VX to VX xor VY. (Bitwise XOR operation)
            0x3 => self.registers[x] ^= self.registers[y],
            // Adds VY to VX. VF is set to 1 when there's a carry, and to 0 when there isn't.
            0x4 => {
                let (result, overflow) = self.registers[x].overflowing_add(self.registers[y]);
                self.registers[x] = result;
                self.registers[Self::CARRY_REGISTER] = overflow as u8;
            }
            // VY is subtracted from VX. VF is set to 0 when there's a borrow, and 1 when there isn't.
            0x5 => {
                let (result, overflow) = self.registers[x].overflowing_sub(self.registers[y]);
                self.registers[x] = result;
                self.registers[Self::CARRY_REGISTER] = (!overflow) as u8;
            }
            // Stores the least significant bit of VX in VF and then shifts VX to the right by 1.[b]
            0x6 => {
                self.registers[Self::CARRY_REGISTER] = self.registers[x] & 0x1;
                self.registers[x] >>= 1;
            }
            // Sets VX to VY minus VX. VF is set to 0 when there's a borrow, and 1 when there isn't.
            0x7 => {
                let (result, overflow) = self.registers[y].overflowing_sub(self.registers[x]);
                self.registers[x] = result;
                self.registers[Self::CARRY_REGISTER] = (!overflow) as u8;
            }
            // Stores the most significant bit of VX in VF and then shifts VX to the left by 1.
            0xE => {
                self.registers[Self::CARRY_REGISTER] = (self.registers[x] & 0x80) >> 7;
                self.registers[x] <<= 1;
            }
            // Unhandled
            _ => panic!("Unhandled register operation"),
        }
        None
    }

    fn opcode_9(&mut self, data: u12) -> Option<u12> {
        None
    }

    fn opcode_a(&mut self, data: u12) -> Option<u12> {
        None
    }
    fn opcode_b(&mut self, data: u12) -> Option<u12> {
        None
    }
    fn opcode_c(&mut self, data: u12) -> Option<u12> {
        None
    }
    fn opcode_d(&mut self, data: u12) -> Option<u12> {
        None
    }
    fn opcode_e(&mut self, data: u12) -> Option<u12> {
        None
    }
    fn opcode_f(&mut self, data: u12) -> Option<u12> {
        None
    }

    fn split_xnn(data: u12) -> (u8, u8) {
        let data = u16::from(data);
        (((data & 0xF00) >> 8) as u8, (data & 0xFF) as u8)
    }

    fn split_xyn(data: u12) -> (u8, u8, u8) {
        let data = u16::from(data);
        (
            ((data & 0xF00) >> 8) as u8,
            ((data & 0x0F0) >> 4) as u8,
            (data & 0x00F) as u8,
        )
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
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
    fn op_00E0_blanks_screen(video: Rc<RefCell<MockVideo>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), video.clone());
        video.borrow_mut().expect_blank_screen().returning(|| ());

        cpu.exec_opcode(0x00E0);

        assert_eq!(u12::new(0x202), cpu.program_counter);
    }

    #[rstest]
    fn op_00E0_returns_from_subroutine(video: Rc<RefCell<MockVideo>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), video.clone());
        cpu.stack.push_back(u12::new(0x400));

        cpu.exec_opcode(0x00EE);

        assert_eq!(u12::new(0x400), cpu.program_counter);
    }

    #[rstest]
    fn op_1NNN_jumps_to_address(video: Rc<RefCell<MockVideo>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), video.clone());

        cpu.exec_opcode(0x1400);

        assert_eq!(u12::new(0x400), cpu.program_counter);
    }

    #[rstest]
    fn op_2NNN_calls_subroutine(video: Rc<RefCell<MockVideo>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), video.clone());

        cpu.exec_opcode(0x2400);

        assert_eq!(u12::new(0x400), cpu.program_counter);
        assert_eq!(u12::new(0x202), cpu.stack.pop_back().unwrap());
    }

    #[rstest]
    fn op_3XNN_skips_instruction_if_eq(video: Rc<RefCell<MockVideo>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), video.clone());
        cpu.registers[4] = 0x10;

        cpu.exec_opcode(0x3410);

        assert_eq!(u12::new(0x204), cpu.program_counter);
    }

    #[rstest]
    fn op_3XNN_does_not_skip_when_ne(video: Rc<RefCell<MockVideo>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), video.clone());
        cpu.registers[4] = 0x11;

        cpu.exec_opcode(0x3410);

        assert_eq!(u12::new(0x202), cpu.program_counter);
    }

    #[rstest]
    fn op_4XNN_skips_instruction_if_ne(video: Rc<RefCell<MockVideo>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), video.clone());
        cpu.registers[4] = 0x11;

        cpu.exec_opcode(0x4410);

        assert_eq!(u12::new(0x204), cpu.program_counter);
    }

    #[rstest]
    fn op_4XNN_does_not_skip_when_eq(video: Rc<RefCell<MockVideo>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), video.clone());
        cpu.registers[4] = 0x10;

        cpu.exec_opcode(0x4410);

        assert_eq!(u12::new(0x202), cpu.program_counter);
    }

    #[rstest]
    fn op_5XY0_skips_instruction_if_eq(video: Rc<RefCell<MockVideo>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), video.clone());
        cpu.registers[4] = 0x10;
        cpu.registers[5] = 0x10;

        cpu.exec_opcode(0x5450);

        assert_eq!(u12::new(0x204), cpu.program_counter);
    }

    #[rstest]
    fn op_5XY0_does_not_skip_when_ne(video: Rc<RefCell<MockVideo>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), video.clone());
        cpu.registers[4] = 0x10;
        cpu.registers[5] = 0x11;

        cpu.exec_opcode(0x5450);

        assert_eq!(u12::new(0x202), cpu.program_counter);
    }

    #[rstest]
    fn op_6XNN_sets_register(video: Rc<RefCell<MockVideo>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), video.clone());

        cpu.exec_opcode(0x6450);

        assert_eq!(0x50, cpu.registers[4]);
    }

    #[rstest]
    fn op_7XNN_adds_to_register(video: Rc<RefCell<MockVideo>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), video.clone());
        cpu.registers[4] = 0x02;

        cpu.exec_opcode(0x74FF);

        assert_eq!(0x01, cpu.registers[4]);
        assert_eq!(0, cpu.registers[Cpu::CARRY_REGISTER]);
    }

    #[rstest]
    fn op_8XY0_sets_register(video: Rc<RefCell<MockVideo>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), video.clone());
        cpu.registers[4] = 0x02;

        cpu.exec_opcode(0x8140);

        assert_eq!(0x02, cpu.registers[1]);
    }

    #[rstest]
    fn op_8XY1_does_or(video: Rc<RefCell<MockVideo>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), video.clone());
        cpu.registers[1] = 0b1011;
        cpu.registers[4] = 0b1101;

        cpu.exec_opcode(0x8141);

        assert_eq!(0b1111, cpu.registers[1]);
    }

    #[rstest]
    fn op_8XY2_does_and(video: Rc<RefCell<MockVideo>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), video.clone());
        cpu.registers[1] = 0b1011;
        cpu.registers[4] = 0b1101;

        cpu.exec_opcode(0x8142);

        assert_eq!(0b1001, cpu.registers[1]);
    }

    #[rstest]
    fn op_8XY3_does_xor(video: Rc<RefCell<MockVideo>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), video.clone());
        cpu.registers[1] = 0b1011;
        cpu.registers[4] = 0b1101;

        cpu.exec_opcode(0x8143);

        assert_eq!(0b0110, cpu.registers[1]);
    }

    #[rstest]
    fn op_8XY4_does_add(video: Rc<RefCell<MockVideo>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), video.clone());
        cpu.registers[Cpu::CARRY_REGISTER] = 0x01;
        cpu.registers[1] = 0x04;
        cpu.registers[4] = 0x03;

        cpu.exec_opcode(0x8144);

        assert_eq!(0x07, cpu.registers[1]);
        assert_eq!(0x00, cpu.registers[Cpu::CARRY_REGISTER]);
    }

    #[rstest]
    fn op_8XY4_does_add_with_carry(video: Rc<RefCell<MockVideo>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), video.clone());
        cpu.registers[1] = 0xFF;
        cpu.registers[4] = 0x03;

        cpu.exec_opcode(0x8144);

        assert_eq!(0x02, cpu.registers[1]);
        assert_eq!(0x01, cpu.registers[Cpu::CARRY_REGISTER]);
    }

    #[rstest]
    fn op_8XY5_does_sub(video: Rc<RefCell<MockVideo>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), video.clone());
        cpu.registers[1] = 0x05;
        cpu.registers[4] = 0x03;

        cpu.exec_opcode(0x8145);

        assert_eq!(0x02, cpu.registers[1]);
        assert_eq!(0x01, cpu.registers[Cpu::CARRY_REGISTER]);
    }

    #[rstest]
    fn op_8XY5_does_sub_with_carry(video: Rc<RefCell<MockVideo>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), video.clone());
        cpu.registers[Cpu::CARRY_REGISTER] = 0x01;
        cpu.registers[1] = 0x01;
        cpu.registers[4] = 0x02;

        cpu.exec_opcode(0x8145);

        assert_eq!(0xFF, cpu.registers[1]);
        assert_eq!(0x00, cpu.registers[Cpu::CARRY_REGISTER]);
    }

    #[rstest]
    fn op_8XY6_does_right_shift(video: Rc<RefCell<MockVideo>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), video.clone());
        cpu.registers[1] = 0b0101;

        cpu.exec_opcode(0x8146);

        assert_eq!(0b0010, cpu.registers[1]);
        assert_eq!(0x01, cpu.registers[Cpu::CARRY_REGISTER]);
    }

    #[rstest]
    fn op_8XY7_does_reverse_sub(video: Rc<RefCell<MockVideo>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), video.clone());
        cpu.registers[1] = 0x03;
        cpu.registers[4] = 0x05;

        cpu.exec_opcode(0x8147);

        assert_eq!(0x02, cpu.registers[1]);
        assert_eq!(0x01, cpu.registers[Cpu::CARRY_REGISTER]);
    }

    #[rstest]
    fn op_8XY7_does_reverse_sub_with_carry(
        video: Rc<RefCell<MockVideo>>,
        mmu: Rc<RefCell<MockMmu>>,
    ) {
        let mut cpu = Cpu::new(mmu.clone(), video.clone());
        cpu.registers[Cpu::CARRY_REGISTER] = 0x01;
        cpu.registers[1] = 0x02;
        cpu.registers[4] = 0x01;

        cpu.exec_opcode(0x8147);

        assert_eq!(0xFF, cpu.registers[1]);
        assert_eq!(0x00, cpu.registers[Cpu::CARRY_REGISTER]);
    }

    #[rstest]
    fn op_8XYE_does_left_shift(video: Rc<RefCell<MockVideo>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), video.clone());
        cpu.registers[1] = 0b1000_0010;

        cpu.exec_opcode(0x814E);

        assert_eq!(0b0100, cpu.registers[1]);
        assert_eq!(0x01, cpu.registers[Cpu::CARRY_REGISTER]);
    }
}
