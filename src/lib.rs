use libc::{c_void, mmap, munmap, MAP_FAILED, MAP_SHARED, PROT_READ, PROT_WRITE};
use std::{convert::TryFrom, fs::OpenOptions, os::unix::io::AsRawFd, path::Path, ptr};

#[derive(Clone, Copy, Debug)]
pub enum Bank {
    A = 0,
    B = 1,
    C = 2,
    D = 3,
    E = 4,
    F = 5,
    G = 6,
}

#[derive(Clone, Copy, Debug)]
pub enum PinCfg {
    Input = 0,
    Output = 1,
}

pub struct DevMemIo {
    mapped_mem: *mut u32,
    size: usize,
}

impl DevMemIo {
    /// start_addr and size must be divisible by the system page size
    pub fn new<P: AsRef<Path>>(path: P, start_addr: i64, size: usize) -> std::io::Result<Self> {
        let mem_file = OpenOptions::new().read(true).write(true).open(path)?;

        let mapped_mem = unsafe {
            mmap(
                ptr::null_mut(),
                size,
                PROT_READ | PROT_WRITE,
                MAP_SHARED,
                mem_file.as_raw_fd(),
                start_addr,
            )
        };

        if mapped_mem == MAP_FAILED {
            return Err(std::io::Error::last_os_error());
        }

        Ok(Self {
            mapped_mem: mapped_mem as *mut u32,
            size,
        })
    }

    pub fn ptr(&self) -> *mut u32 {
        self.mapped_mem
    }
}

impl Drop for DevMemIo {
    fn drop(&mut self) {
        unsafe {
            munmap(self.mapped_mem as *mut c_void, self.size);
        }
    }
}

pub struct Gpio {
    mem_io: DevMemIo,
}

impl Gpio {
    pub fn new() -> std::io::Result<Self> {
        let mem_io = DevMemIo::new("/dev/mem", 0x01C2_0000, 0x1000)?;
        Ok(Self { mem_io })
    }

    fn get_bank_ptr(&self, bank: Bank, reg_offset: isize) -> *mut u32 {
        let n = bank as isize;
        self.mem_io
            .ptr()
            .wrapping_offset((0x0800 + n * 0x24 + reg_offset) / 4)
    }

    fn get_data_reg_ptr(&self, bank: Bank) -> *mut u32 {
        self.get_bank_ptr(bank, 0x10)
    }

    pub fn read_bank(&self, bank: Bank) -> u32 {
        unsafe { ptr::read_volatile(self.get_data_reg_ptr(bank)) }
    }

    pub fn write_bank(&mut self, bank: Bank, value: u32) {
        unsafe { ptr::write_volatile(self.get_data_reg_ptr(bank), value) }
    }

    pub fn configure_pin(&mut self, bank: Bank, pin: u32, cfg: PinCfg) {
        // 8 pins per register
        let reg_num = pin / 8;
        let reg_offset = pin % 8;

        unsafe {
            let ptr = self.get_bank_ptr(bank, 4 * isize::try_from(reg_num).unwrap());
            let mut value = ptr::read_volatile(ptr);
            let shift = 4 * reg_offset;
            let mask = 0b1111 << shift;
            value &= !mask;
            value |= (cfg as u32) << shift;
            ptr::write_volatile(ptr, value);
        }
    }
}
