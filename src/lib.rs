pub struct Mmu {
    memory: Vec<u8>,
}

impl Mmu {
    const MEM_SIZE: usize = 4096;
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

    pub fn load_program(&mut self, file_path: &String) {
        // TODO: Implement
    }
}
