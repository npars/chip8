use super::mmu::Mmu;
use super::window::Window;
use crate::mmu::Chip8Mmu;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use ux::u12;

pub struct Cpu {
    mmu: Rc<RefCell<dyn Mmu>>,
    window: Rc<RefCell<dyn Window>>,
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

    pub fn new(mmu: Rc<RefCell<dyn Mmu>>, window: Rc<RefCell<dyn Window>>) -> Cpu {
        Cpu {
            mmu,
            window,
            registers: vec![0; Cpu::REGISTER_SIZE],
            index: u12::new(0),
            program_counter: u12::new(0x200),
            delay_timer: 0,
            sound_timer: 0,
            stack: VecDeque::with_capacity(Cpu::STACK_SIZE),
        }
    }

    pub fn run_cycle(&mut self) {
        let opcode = self.mmu.as_ref().borrow().read_u16(self.program_counter);
        self.exec_opcode(opcode);
    }

    pub fn run_60hz_cycle(&mut self) {
        if self.sound_timer > 0 {
            self.sound_timer -= 1;
        }

        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }
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
                self.window.borrow_mut().blank_screen();
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
        let (x, y, _) = Self::split_xyn(data);
        if self.registers[x as usize] == self.registers[y as usize] {
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
        // Skips the next instruction if VX doesn't equal VY.
        let (x, y, _) = Self::split_xyn(data);
        if self.registers[x as usize] != self.registers[y as usize] {
            Some(
                self.program_counter
                    .wrapping_add(u12::new(Self::OPCODE_SIZE * 2)),
            )
        } else {
            None
        }
    }

    fn opcode_a(&mut self, data: u12) -> Option<u12> {
        // Sets I to the address NNN
        self.index = data;
        None
    }

    fn opcode_b(&mut self, data: u12) -> Option<u12> {
        // Jumps to the address NNN plus V0.
        Some(u12::from(self.registers[0]).wrapping_add(data))
    }

    fn opcode_c(&mut self, data: u12) -> Option<u12> {
        // Sets VX to the result of a bitwise and operation on a random number and NN.
        let (register_index, bitmask) = Self::split_xnn(data);
        self.registers[register_index as usize] = fastrand::u8(..) & bitmask;
        None
    }

    fn opcode_d(&mut self, data: u12) -> Option<u12> {
        // Draws a sprite at coordinate (VX, VY) that has a width of 8 pixels and a height of N+1 pixels
        let (x, y, n) = Self::split_xyn(data);

        let sprite = (0..n)
            .map(|i| {
                self.mmu
                    .as_ref()
                    .borrow()
                    .read_u8(self.index.wrapping_add(u12::from(i)))
            })
            .collect();
        self.registers[Self::CARRY_REGISTER] = self.window.borrow_mut().draw(
            self.registers[x as usize],
            self.registers[y as usize],
            sprite,
        ) as u8;
        None
    }

    fn opcode_e(&mut self, data: u12) -> Option<u12> {
        let (x, opcode) = Self::split_xnn(data);

        let is_key_pressed = self
            .window
            .as_ref()
            .borrow()
            .is_key_pressed(self.registers[x as usize]);

        match opcode {
            // Skips the next instruction if the key stored in VX is pressed.
            0x9E => {
                if is_key_pressed {
                    Some(
                        self.program_counter
                            .wrapping_add(u12::new(Self::OPCODE_SIZE * 2)),
                    )
                } else {
                    None
                }
            }
            // Skips the next instruction if the key stored in VX isn't pressed.
            0xA1 => {
                if !is_key_pressed {
                    Some(
                        self.program_counter
                            .wrapping_add(u12::new(Self::OPCODE_SIZE * 2)),
                    )
                } else {
                    None
                }
            }
            // Unhandled
            _ => panic!("Unhandled key check operation"),
        }
    }

    fn opcode_f(&mut self, data: u12) -> Option<u12> {
        let (x, opcode) = Self::split_xnn(data);
        let x = x as usize;

        match opcode {
            // Sets VX to the value of the delay timer.
            0x07 => self.registers[x] = self.delay_timer,
            // A key press is awaited, and then stored in VX.
            0x0A => match self.window.as_ref().borrow().get_pressed_key() {
                Some(key) => self.registers[x] = key,
                None => return Some(self.program_counter),
            },
            // Sets the delay timer to VX.
            0x15 => self.delay_timer = self.registers[x],
            // Sets the sound timer to VX.
            0x18 => self.sound_timer = self.registers[x],
            // Adds VX to I. VF is not affected.
            0x1E => self.index = self.index.wrapping_add(u12::from(self.registers[x])),
            // Sets I to the location of the sprite for the character in VX.
            0x29 => {
                self.index =
                    u12::new((Chip8Mmu::FONT_SPRITE_HEIGHT as u16) * (self.registers[x] as u16))
            }
            // Stores the binary-coded decimal representation of VX
            0x33 => {
                self.mmu
                    .borrow_mut()
                    .write_u8(self.index, self.registers[x] / 100);
                self.mmu.borrow_mut().write_u8(
                    self.index.wrapping_add(u12::new(1)),
                    (self.registers[x] % 100) / 10,
                );
                self.mmu
                    .borrow_mut()
                    .write_u8(self.index.wrapping_add(u12::new(2)), self.registers[x] % 10);
            }
            // Stores V0 to VX (including VX) in memory starting at address I.
            0x55 => {
                for i in 0..=x {
                    self.mmu.borrow_mut().write_u8(
                        self.index.wrapping_add(u12::from(i as u8)),
                        self.registers[i],
                    );
                }
            }
            // Fills V0 to VX (including VX) with values from memory starting at address I.
            0x65 => {
                for i in 0..=x {
                    self.registers[i] = self
                        .mmu
                        .borrow()
                        .read_u8(self.index.wrapping_add(u12::from(i as u8)));
                }
            }
            _ => panic!("Unhandled register operation"),
        }
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
    use super::super::mmu::MockMmu;
    use super::super::window::MockWindow;
    use super::*;
    use mockall::predicate::eq;
    use rstest::*;

    #[fixture]
    fn mmu() -> Rc<RefCell<MockMmu>> {
        Rc::new(RefCell::new(MockMmu::new()))
    }

    #[fixture]
    fn window() -> Rc<RefCell<MockWindow>> {
        Rc::new(RefCell::new(MockWindow::new()))
    }

    #[rstest]
    fn pc_has_default(window: Rc<RefCell<MockWindow>>, mmu: Rc<RefCell<MockMmu>>) {
        let cpu = Cpu::new(mmu.clone(), window.clone());
        assert_eq!(u12::new(0x200), cpu.program_counter);
    }

    #[rstest]
    fn op_00E0_blanks_screen(window: Rc<RefCell<MockWindow>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());
        window.borrow_mut().expect_blank_screen().returning(|| ());

        cpu.exec_opcode(0x00E0);

        assert_eq!(u12::new(0x202), cpu.program_counter);
    }

    #[rstest]
    fn op_00E0_returns_from_subroutine(window: Rc<RefCell<MockWindow>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());
        cpu.stack.push_back(u12::new(0x400));

        cpu.exec_opcode(0x00EE);

        assert_eq!(u12::new(0x400), cpu.program_counter);
    }

    #[rstest]
    fn op_1NNN_jumps_to_address(window: Rc<RefCell<MockWindow>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());

        cpu.exec_opcode(0x1400);

        assert_eq!(u12::new(0x400), cpu.program_counter);
    }

    #[rstest]
    fn op_2NNN_calls_subroutine(window: Rc<RefCell<MockWindow>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());

        cpu.exec_opcode(0x2400);

        assert_eq!(u12::new(0x400), cpu.program_counter);
        assert_eq!(u12::new(0x202), cpu.stack.pop_back().unwrap());
    }

    #[rstest]
    fn op_3XNN_skips_instruction_if_eq(window: Rc<RefCell<MockWindow>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());
        cpu.registers[4] = 0x10;

        cpu.exec_opcode(0x3410);

        assert_eq!(u12::new(0x204), cpu.program_counter);
    }

    #[rstest]
    fn op_3XNN_does_not_skip_when_ne(window: Rc<RefCell<MockWindow>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());
        cpu.registers[4] = 0x11;

        cpu.exec_opcode(0x3410);

        assert_eq!(u12::new(0x202), cpu.program_counter);
    }

    #[rstest]
    fn op_4XNN_skips_instruction_if_ne(window: Rc<RefCell<MockWindow>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());
        cpu.registers[4] = 0x11;

        cpu.exec_opcode(0x4410);

        assert_eq!(u12::new(0x204), cpu.program_counter);
    }

    #[rstest]
    fn op_4XNN_does_not_skip_when_eq(window: Rc<RefCell<MockWindow>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());
        cpu.registers[4] = 0x10;

        cpu.exec_opcode(0x4410);

        assert_eq!(u12::new(0x202), cpu.program_counter);
    }

    #[rstest]
    fn op_5XY0_skips_instruction_if_eq(window: Rc<RefCell<MockWindow>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());
        cpu.registers[4] = 0x10;
        cpu.registers[5] = 0x10;

        cpu.exec_opcode(0x5450);

        assert_eq!(u12::new(0x204), cpu.program_counter);
    }

    #[rstest]
    fn op_5XY0_does_not_skip_when_ne(window: Rc<RefCell<MockWindow>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());
        cpu.registers[4] = 0x10;
        cpu.registers[5] = 0x11;

        cpu.exec_opcode(0x5450);

        assert_eq!(u12::new(0x202), cpu.program_counter);
    }

    #[rstest]
    fn op_6XNN_sets_register(window: Rc<RefCell<MockWindow>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());

        cpu.exec_opcode(0x6450);

        assert_eq!(0x50, cpu.registers[4]);
    }

    #[rstest]
    fn op_7XNN_adds_to_register(window: Rc<RefCell<MockWindow>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());
        cpu.registers[4] = 0x02;

        cpu.exec_opcode(0x74FF);

        assert_eq!(0x01, cpu.registers[4]);
        assert_eq!(0, cpu.registers[Cpu::CARRY_REGISTER]);
    }

    #[rstest]
    fn op_8XY0_sets_register(window: Rc<RefCell<MockWindow>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());
        cpu.registers[4] = 0x02;

        cpu.exec_opcode(0x8140);

        assert_eq!(0x02, cpu.registers[1]);
    }

    #[rstest]
    fn op_8XY1_does_or(window: Rc<RefCell<MockWindow>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());
        cpu.registers[1] = 0b1011;
        cpu.registers[4] = 0b1101;

        cpu.exec_opcode(0x8141);

        assert_eq!(0b1111, cpu.registers[1]);
    }

    #[rstest]
    fn op_8XY2_does_and(window: Rc<RefCell<MockWindow>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());
        cpu.registers[1] = 0b1011;
        cpu.registers[4] = 0b1101;

        cpu.exec_opcode(0x8142);

        assert_eq!(0b1001, cpu.registers[1]);
    }

    #[rstest]
    fn op_8XY3_does_xor(window: Rc<RefCell<MockWindow>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());
        cpu.registers[1] = 0b1011;
        cpu.registers[4] = 0b1101;

        cpu.exec_opcode(0x8143);

        assert_eq!(0b0110, cpu.registers[1]);
    }

    #[rstest]
    fn op_8XY4_does_add(window: Rc<RefCell<MockWindow>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());
        cpu.registers[Cpu::CARRY_REGISTER] = 0x01;
        cpu.registers[1] = 0x04;
        cpu.registers[4] = 0x03;

        cpu.exec_opcode(0x8144);

        assert_eq!(0x07, cpu.registers[1]);
        assert_eq!(0x00, cpu.registers[Cpu::CARRY_REGISTER]);
    }

    #[rstest]
    fn op_8XY4_does_add_with_carry(window: Rc<RefCell<MockWindow>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());
        cpu.registers[1] = 0xFF;
        cpu.registers[4] = 0x03;

        cpu.exec_opcode(0x8144);

        assert_eq!(0x02, cpu.registers[1]);
        assert_eq!(0x01, cpu.registers[Cpu::CARRY_REGISTER]);
    }

    #[rstest]
    fn op_8XY5_does_sub(window: Rc<RefCell<MockWindow>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());
        cpu.registers[1] = 0x05;
        cpu.registers[4] = 0x03;

        cpu.exec_opcode(0x8145);

        assert_eq!(0x02, cpu.registers[1]);
        assert_eq!(0x01, cpu.registers[Cpu::CARRY_REGISTER]);
    }

    #[rstest]
    fn op_8XY5_does_sub_with_carry(window: Rc<RefCell<MockWindow>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());
        cpu.registers[Cpu::CARRY_REGISTER] = 0x01;
        cpu.registers[1] = 0x01;
        cpu.registers[4] = 0x02;

        cpu.exec_opcode(0x8145);

        assert_eq!(0xFF, cpu.registers[1]);
        assert_eq!(0x00, cpu.registers[Cpu::CARRY_REGISTER]);
    }

    #[rstest]
    fn op_8XY6_does_right_shift(window: Rc<RefCell<MockWindow>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());
        cpu.registers[1] = 0b0101;

        cpu.exec_opcode(0x8146);

        assert_eq!(0b0010, cpu.registers[1]);
        assert_eq!(0x01, cpu.registers[Cpu::CARRY_REGISTER]);
    }

    #[rstest]
    fn op_8XY7_does_reverse_sub(window: Rc<RefCell<MockWindow>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());
        cpu.registers[1] = 0x03;
        cpu.registers[4] = 0x05;

        cpu.exec_opcode(0x8147);

        assert_eq!(0x02, cpu.registers[1]);
        assert_eq!(0x01, cpu.registers[Cpu::CARRY_REGISTER]);
    }

    #[rstest]
    fn op_8XY7_does_reverse_sub_with_carry(
        window: Rc<RefCell<MockWindow>>,
        mmu: Rc<RefCell<MockMmu>>,
    ) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());
        cpu.registers[Cpu::CARRY_REGISTER] = 0x01;
        cpu.registers[1] = 0x02;
        cpu.registers[4] = 0x01;

        cpu.exec_opcode(0x8147);

        assert_eq!(0xFF, cpu.registers[1]);
        assert_eq!(0x00, cpu.registers[Cpu::CARRY_REGISTER]);
    }

    #[rstest]
    fn op_8XYE_does_left_shift(window: Rc<RefCell<MockWindow>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());
        cpu.registers[1] = 0b1000_0010;

        cpu.exec_opcode(0x814E);

        assert_eq!(0b0100, cpu.registers[1]);
        assert_eq!(0x01, cpu.registers[Cpu::CARRY_REGISTER]);
    }

    #[rstest]
    fn op_9XY0_skips_instruction_if_ne(window: Rc<RefCell<MockWindow>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());
        cpu.registers[4] = 0x10;
        cpu.registers[5] = 0x11;

        cpu.exec_opcode(0x9450);

        assert_eq!(u12::new(0x204), cpu.program_counter);
    }

    #[rstest]
    fn op_ANNN_sets_index(window: Rc<RefCell<MockWindow>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());

        cpu.exec_opcode(0xA123);

        assert_eq!(u12::new(0x123), cpu.index);
    }

    #[rstest]
    fn op_BNNN_jumps(window: Rc<RefCell<MockWindow>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());
        cpu.registers[0] = 0x10;

        cpu.exec_opcode(0xB113);

        assert_eq!(u12::new(0x123), cpu.program_counter);
    }

    #[rstest]
    fn op_DXYN_draws_sprite(window: Rc<RefCell<MockWindow>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());
        cpu.registers[3] = 7;
        cpu.registers[2] = 8;
        cpu.index = u12::new(0x010);
        mmu.borrow_mut()
            .expect_read_u8()
            .returning(|x| u16::from(x) as u8);
        window
            .borrow_mut()
            .expect_draw()
            .with(eq(7), eq(8), eq(vec![0x10]))
            .returning(|_, _, _| true);

        cpu.exec_opcode(0xD321);

        assert_eq!(0x1, cpu.registers[0xF])
    }

    #[rstest]
    fn op_DXYN_draws_non_zero_sprite(window: Rc<RefCell<MockWindow>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());
        cpu.registers[3] = 7;
        cpu.registers[2] = 8;
        cpu.index = u12::new(0x010);
        mmu.borrow_mut()
            .expect_read_u8()
            .times(2)
            .returning(|x| u16::from(x) as u8);
        window
            .borrow_mut()
            .expect_draw()
            .with(eq(7), eq(8), eq(vec![0x10, 0x11]))
            .returning(|_, _, _| false);

        cpu.exec_opcode(0xD322);
        assert_eq!(0x0, cpu.registers[0xF])
    }

    #[rstest]
    fn op_EX9E_skips_if_key_pressed(window: Rc<RefCell<MockWindow>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());
        window
            .borrow_mut()
            .expect_is_key_pressed()
            .with(eq(0xA))
            .returning(|_| true);
        cpu.registers[4] = 0xA;

        cpu.exec_opcode(0xE49E);

        assert_eq!(u12::new(0x204), cpu.program_counter);
    }

    #[rstest]
    fn op_EXA1_skips_if_key_not_pressed(
        window: Rc<RefCell<MockWindow>>,
        mmu: Rc<RefCell<MockMmu>>,
    ) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());
        window
            .borrow_mut()
            .expect_is_key_pressed()
            .with(eq(0xA))
            .returning(|_| false);
        cpu.registers[4] = 0xA;

        cpu.exec_opcode(0xE4A1);

        assert_eq!(u12::new(0x204), cpu.program_counter);
    }

    #[rstest]
    fn op_FX07_sets_vx_to_delay(window: Rc<RefCell<MockWindow>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());
        cpu.delay_timer = 0xA1;

        cpu.exec_opcode(0xF407);

        assert_eq!(0xA1, cpu.registers[4]);
    }

    #[rstest]
    fn op_FX0A_sets_vx_to_key(window: Rc<RefCell<MockWindow>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());
        window
            .borrow_mut()
            .expect_get_pressed_key()
            .returning(|| Some(0x8));

        cpu.exec_opcode(0xF40A);

        assert_eq!(0x8, cpu.registers[4]);
        assert_eq!(u12::new(0x202), cpu.program_counter);
    }

    #[rstest]
    fn op_FX0A_blocks_when_no_key(window: Rc<RefCell<MockWindow>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());
        window
            .borrow_mut()
            .expect_get_pressed_key()
            .returning(|| None);

        cpu.exec_opcode(0xF40A);

        assert_eq!(u12::new(0x200), cpu.program_counter);
    }

    #[rstest]
    fn op_FX15_sets_delay(window: Rc<RefCell<MockWindow>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());
        cpu.registers[4] = 0xA2;

        cpu.exec_opcode(0xF415);

        assert_eq!(0xA2, cpu.delay_timer);
    }

    #[rstest]
    fn op_FX15_sets_sound(window: Rc<RefCell<MockWindow>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());
        cpu.registers[4] = 0xA3;

        cpu.exec_opcode(0xF418);

        assert_eq!(0xA3, cpu.sound_timer);
    }

    #[rstest]
    fn op_FX1E_increments_index(window: Rc<RefCell<MockWindow>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());
        cpu.index = u12::new(0xA00);
        cpu.registers[4] = 0xFF;

        cpu.exec_opcode(0xF41E);

        assert_eq!(u12::new(0xAFF), cpu.index);
    }

    #[rstest]
    fn op_FX29_sets_index_to_sprite(window: Rc<RefCell<MockWindow>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());
        cpu.registers[4] = 0xB;

        cpu.exec_opcode(0xF429);

        assert_eq!(u12::new(55), cpu.index);
    }

    #[rstest]
    fn op_FX33_writes_bcd(window: Rc<RefCell<MockWindow>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());
        cpu.index = u12::new(0x100);
        cpu.registers[4] = 213;

        mmu.borrow_mut()
            .expect_write_u8()
            .with(eq(u12::new(0x100)), eq(2))
            .returning(|_, _| ());
        mmu.borrow_mut()
            .expect_write_u8()
            .with(eq(u12::new(0x101)), eq(1))
            .returning(|_, _| ());
        mmu.borrow_mut()
            .expect_write_u8()
            .with(eq(u12::new(0x102)), eq(3))
            .returning(|_, _| ());

        cpu.exec_opcode(0xF433);
    }

    #[rstest]
    fn op_FX55_dumps_registers(window: Rc<RefCell<MockWindow>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());
        cpu.index = u12::new(0x100);
        cpu.registers[0] = 0x10;
        cpu.registers[1] = 0x23;

        mmu.borrow_mut()
            .expect_write_u8()
            .with(eq(u12::new(0x100)), eq(0x10))
            .returning(|_, _| ());
        mmu.borrow_mut()
            .expect_write_u8()
            .with(eq(u12::new(0x101)), eq(0x23))
            .returning(|_, _| ());

        cpu.exec_opcode(0xF155);
    }

    #[rstest]
    fn op_FX55_loads_registers(window: Rc<RefCell<MockWindow>>, mmu: Rc<RefCell<MockMmu>>) {
        let mut cpu = Cpu::new(mmu.clone(), window.clone());
        cpu.index = u12::new(0x100);

        mmu.borrow_mut()
            .expect_read_u8()
            .with(eq(u12::new(0x100)))
            .return_const(7);

        mmu.borrow_mut()
            .expect_read_u8()
            .with(eq(u12::new(0x101)))
            .return_const(8);

        cpu.exec_opcode(0xF165);

        assert_eq!(7, cpu.registers[0]);
        assert_eq!(8, cpu.registers[1]);
    }
}
