use std::io::{self, Write};

const MEMORY_SIZE: usize = 4096;
const NREGS: usize = 16;

const IP: usize = 0;

pub struct Machine {
    memory: [u8; MEMORY_SIZE],
    regs: [u32; NREGS],
}


#[derive(Debug)]
pub enum MachineError {
    RegisterOutOfBounds,
    MemoryOutOfBoundsStepOn,
    MemoryOutOfBoundsLoad,
    MemoryOutOfBoundsStore,
    WrongInstruction,
    OutOfBounds,
    InvalidRegister(usize),
    InvalidInstruction(u8),
    IoError(std::io::Error),
    // add more errors as needed
}


impl Machine {
    /// Create a new machine in its reset state. The `memory` parameter will
    /// be copied at the beginning of the machine memory.
    ///
    /// # Panics
    /// This function panics when `memory` is larger than the machine memory.
    pub fn new(memory: &[u8]) -> Self {
        if memory.len() > MEMORY_SIZE {
            panic!("memory slice is too large for the machine memory");
        }
    
        let mut machine = Machine {
            regs: [0; NREGS],
            memory: [0; MEMORY_SIZE],
        };
    
        machine.memory[..memory.len()].copy_from_slice(memory);
    
        machine
    }
    

    /// Run until the program terminates or until an error happens.
    /// If output instructions are run, they print on `fd`.
    pub fn run_on<T: Write>(&mut self, fd: &mut T) -> Result<(), MachineError> {
        loop {
            let terminated = self.step_on(fd)?;
            if terminated {
                break;
            }
        }
        Ok(())
    }

    /// Run until the program terminates or until an error happens.
    /// If output instructions are run, they print on standard output.
    pub fn run(&mut self) -> Result<(), MachineError> {
        self.run_on(&mut io::stdout().lock())
    }

    /// Execute the next instruction by doing the following steps:
    ///   - decode the instruction located at IP (register 0)
    ///   - increment the IP by the size of the instruction
    ///   - execute the decoded instruction
    ///
    /// If output instructions are run, they print on `fd`.
    /// If an error happens at either of those steps, an error is
    /// returned.
    ///
    /// In case of success, `true` is returned if the program is
    /// terminated (upon encountering an exit instruction), or
    /// `false` if the execution must continue.
    pub fn step_on<T: Write>(&mut self, fd: &mut T) -> Result<bool, MachineError> {
        let instruction_ad: u32 = self.regs[IP];
        if instruction_ad >= MEMORY_SIZE as u32 {
            return Err(MachineError::MemoryOutOfBoundsStepOn);
        }
        let opcode: u8 = self.memory[instruction_ad as usize];
        let size: u32 = decode(opcode);
        let next_instruction_ad: u32 = instruction_ad + size;
        self.set_reg(0, next_instruction_ad);
        let mut b1: u8 = 0;
        let mut b2: u8 = 0;
        let mut b3: u8 = 0;
        if instruction_ad + 1 < MEMORY_SIZE as u32 {
            b1 = self.memory[(instruction_ad + 1) as usize];
        }
        if instruction_ad + 2 < MEMORY_SIZE as u32 {
            b2 = self.memory[(instruction_ad + 2) as usize];
        }
        if instruction_ad + 3 < MEMORY_SIZE as u32 {
            b3 = self.memory[(instruction_ad + 3) as usize];
        }
        match opcode {
            1 => return self.move_(b1,b2,b3),
            2 => return self.store(b1, b2),
            3 => return self.load(b1,b2),
            4 => return self.load_imm(b1, b2, b3),
            5 => return self.sub(b1, b2, b3),
            6 => return self.out(fd,b1),
            7 => return self.exit(),
            8 => return self.out_number(fd, b1),
            _ => return Err(MachineError::WrongInstruction),
        }
    }
    
    /// Similar to [step_on](Machine::step_on).
    /// If output instructions are run, they print on standard output.
    pub fn step(&mut self) -> Result<bool, MachineError> {
        self.step_on(&mut io::stdout().lock())
    }

    /// Reference onto the machine current set of regs.
    pub fn regs(&self) -> &[u32] {
        &self.regs
    }

    /// Sets a register to the given value.
    pub fn set_reg(&mut self, reg: usize, value: u32) -> Result<(), MachineError> {
        if reg >= NREGS {
            return Err(MachineError::InvalidRegister(reg));
        }
        self.regs[reg] = value;
        Ok(())
    }


    /// Reference onto the machine current memory.
    pub fn memory(&self) -> &[u8] {
        return &self.memory;
    }

    pub fn move_(&mut self, b1: u8, b2: u8, b3: u8) -> Result<bool, MachineError> {
        const NREGS_U8: u8 = NREGS as u8; // store the number of registers as an u8
        let reg_idx = &[b1, b2, b3]; // store the register indices
        if reg_idx.iter().any(|&i| i >= NREGS_U8) {
            return Err(MachineError::OutOfBounds);
        }
        let reg_b = self.regs[b2 as usize];
        let reg_c = self.regs[b3 as usize];
        if reg_c != 0 {
            self.set_reg(b1 as usize, reg_b);
        }
        Ok(false)
    }

    pub fn store(&mut self, dest_reg: u8, src_reg: u8) -> Result<bool, MachineError> {
        const LAST_REG: u8 = (NREGS - 1) as u8;
        const MEM_SIZE: u32 = (MEMORY_SIZE - 4) as u32;
    
        if dest_reg > LAST_REG || src_reg > LAST_REG {
            return Err(MachineError::OutOfBounds);
        }
    
        let dest_addr = self.regs[dest_reg as usize];
        if dest_addr > MEM_SIZE {
            return Err(MachineError::MemoryOutOfBoundsStore);
        }
    
        let src_data = self.regs[src_reg as usize];
    
        self.memory[dest_addr as usize..(dest_addr + 4) as usize].copy_from_slice(&src_data.to_le_bytes());
    
        Ok(false)
    }
    

    /// Loads a 32-bit value from memory and stores it into a register.
    pub fn load(&mut self, b1: u8, b2: u8) -> Result<bool, MachineError> {
        let reg_nb: u8 = (NREGS - 1) as u8;
        let mem_size: usize = MEMORY_SIZE - 4;
    
        if b1 > reg_nb || b2 > reg_nb {
            return Err(MachineError::RegisterOutOfBounds);
        }
    
        let regb_ad: usize = self.regs[b2 as usize] as usize;
    
        if regb_ad > mem_size {
            return Err(MachineError::MemoryOutOfBoundsLoad);
        }
    
        let mut value: u32 = 0;
        for i in 0..4 {
            value |= (self.memory[regb_ad + i] as u32) << (i * 8);
        }
    
        self.regs[b1 as usize] = value;
    
        Ok(false)
    }
    


    pub fn load_imm(&mut self, b1: u8, b2: u8, b3: u8) -> Result<bool, MachineError> {

        let reg_nb: u8 = (NREGS - 1) as u8;

        if b1 > reg_nb {
            return Err(MachineError::OutOfBounds);
        }

        self.regs[b1 as usize]  = ((b3 as i16) << 8 | (b2 as i16)) as u32;

        return Ok(false);
    }


    pub fn sub(&mut self, b1: u8, b2: u8, b3: u8) -> Result<bool, MachineError> {

        let reg_nb: u8 = (NREGS - 1) as u8;
    
        if b1 > reg_nb || b2 > reg_nb || b3 > reg_nb {
            return Err(MachineError::OutOfBounds);
        }
    
        let regc_data: i64 = self.regs[b3 as usize] as i64;
        let regb_data: i64 = self.regs[b2 as usize] as i64;
    
        self.regs[b1 as usize] = (regb_data - regc_data) as u32;
    
        return Ok(false);
    }
    

    pub fn out<T: Write>(&mut self, fd: &mut T, b1: u8) -> Result<bool, MachineError> {
        let reg_nb: u8 = (NREGS - 1) as u8;
    
        if b1 > reg_nb {
            return Err(MachineError::OutOfBounds);
        }
    
        let rega_data: u8 = self.regs[b1 as usize] as u8;
        let c: char = rega_data as char;
        let mut buf: [u8; 4] = [0; 4];
        let str = c.encode_utf8(&mut buf);
        
        match fd.write(str.as_bytes()) {
            Ok(_) => Ok(false),
            Err(err) => Err(MachineError::IoError(err.into())),
        }
    }

    pub fn exit(&mut self) -> Result<bool, MachineError> {
        return Ok(true);
    }

    pub fn out_number<T: Write>(&mut self, fd: &mut T, b1: u8) -> Result<bool, MachineError> {
        let reg_nb: u8 = (NREGS - 1) as u8;

        if b1 > reg_nb {
            return Err(MachineError::OutOfBounds);
        }

        let rega_data: i32 = self.regs[b1 as usize] as i32;

        fd.write(rega_data.to_string().as_bytes())
            .map_err(|e| MachineError::IoError(e))?;

        Ok(false)
    }

        
}

fn decode(opcode: u8) -> u32 {

    let size: u32;

    match opcode {

        1 | 4 | 5  => size = 4, 
        2 | 3 => size = 3, 
        6 | 8 => size = 2, 
        7 => size = 1,
        _ => size = 0,
         
    }
    return size;
}

