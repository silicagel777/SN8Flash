use crate::transport::Transport;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq)]
enum Sfr {
    /// Data pointer 0 low byte register
    Dpl = 0x82,
    /// Data pointer 0 high byte register   
    Dph = 0x83,
    /// Extended cycle controls register  
    Ckon = 0x8E,
    /// Data pointer selection register
    Dps = 0x92,
    /// Data pointer control register
    Dpc = 0x93,
    /// In-System Program command register
    Pecmd = 0x94,
    /// In-System Program ROM address low byte
    Peroml = 0x95,
    /// In-System Program ROM address high byte
    Peromh = 0x96,
    /// In-System Program RAM mapping address
    Peram = 0x97,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq)]
pub enum RomBank {
    /// Main flash memory
    Main = 0,
    /// Some sort of boot parameter area. I've accidentally wiped it on
    /// SN8F570212, and the chip would no longer leave the built-in bootloader
    /// until I've restored it back. Fun stuff!
    Boot = 1,
}

#[derive(getset::Getters, getset::Setters)]
pub struct Flasher {
    // Inner fields
    transport: Box<dyn Transport>,

    #[getset(get = "pub with_prefix", set = "pub")]
    reset_duration_ms: u64,

    #[getset(get = "pub with_prefix", set = "pub")]
    connect_duration_us: u64,

    #[getset(get = "pub with_prefix", set = "pub")]
    rom_bank: RomBank,

    #[getset(get = "pub with_prefix", set = "pub")]
    dangerous_allow_write_non_main_bank: bool,

    #[getset(get = "pub with_prefix", set = "pub")]
    page_size: usize,
}

impl Flasher {
    // Common stuff ===========================================================

    pub fn new(transport: Box<dyn Transport>) -> Self {
        Flasher {
            transport,
            rom_bank: RomBank::Boot,
            reset_duration_ms: 10,
            connect_duration_us: 1666,
            dangerous_allow_write_non_main_bank: false,
            page_size: 0x20,
        }
    }

    fn sleep_ms(&self, millis: u64) {
        std::thread::sleep(std::time::Duration::from_millis(millis));
    }

    fn sleep_us(&self, micros: u64) {
        std::thread::sleep(std::time::Duration::from_micros(micros));
    }

    // Low-level commands =====================================================

    fn cmd_connect(&mut self) {
        self.transport.write(&[
            0x55, 0x08, 0x29, 0x23, 0xBE, 0x84, 0xE1, 0x6C, 0xD6, 0xAE, 0x52, 0x90, 0x49, 0xF1,
            0xF1, 0xBB, 0xE9, 0xEB, 0xB3, 0xA6, 0xDB, 0x3C, 0x87, 0x0C, 0x3E, 0x99, 0x24, 0x5E,
            0x0D, 0x1C, 0x06, 0xB7, 0x47, 0xDE, 0xB3, 0x12, 0x4D, 0xC8, 0x43, 0xBB, 0x8B, 0xA6,
            0x1F, 0x03, 0x5A, 0x7D, 0x09, 0x38, 0x25, 0x1F, 0x5D, 0xD4, 0xCB, 0xFC, 0x96, 0xF5,
            0x45, 0x3B, 0x13, 0x0D, 0x89, 0x0A, 0x1C, 0xDB, 0xAE, 0x32, 0x20, 0x9A, 0x50, 0xEE,
            0x40, 0x78, 0x36, 0xFD, 0x12, 0x49, 0x32, 0xF6, 0x9E, 0x7D, 0x49, 0xDC, 0xAD, 0x4F,
            0x14, 0xF2, 0x44, 0x40, 0x66, 0xD0, 0x6B, 0xC4, 0x30, 0xB7, 0x32, 0x3B, 0xA1, 0x22,
            0xF6, 0x22, 0x91, 0x9D, 0xE1, 0x8B, 0x1F, 0xDA, 0xB0, 0xCA, 0x99, 0x02, 0xB9, 0x72,
            0x9D, 0x49, 0x2C, 0x80, 0x7E, 0x6B, 0x8F, 0xD3, 0x92,
        ]);
        let mut res = [0; 4];
        self.transport.read(&mut res);
        assert_eq!(res, [0xFF; 4], "Invalid handshake response")
    }

    fn cmd_chip_id(&mut self) -> u32 {
        self.transport.write(&[0x55, 0x21, 0x55, 0xA0]);
        let mut res = [0; 4];
        self.transport.read(&mut res);
        u32::from_le_bytes(res)
    }

    fn cmd_get_byte(&mut self) -> u8 {
        self.transport.write(&[0x55, 0x88]);
        let mut res = [0];
        self.transport.read(&mut res);
        res[0]
    }

    fn cmd_unk_2a(&mut self) {
        self.transport.write(&[0x55, 0x2A]);
    }

    fn cmd_unk_2b(&mut self) {
        self.transport.write(&[0x55, 0x2B]);
    }

    fn cmd_unk_48(&mut self, arg1: u8) {
        self.transport.write(&[0x55, 0x48, arg1]);
    }

    fn cmd_unk_4b(&mut self, arg1: u8, arg2: u8) {
        self.transport.write(&[0x55, 0x4B, arg1, arg2]);
    }

    fn cmd_unk_58(&mut self, arg1: u8, arg2: u8, arg3: u8) {
        self.transport.write(&[0x55, 0x58, arg1, arg2, arg3]);
    }

    fn cmd_unk_8b(&mut self) -> [u8; 2] {
        self.transport.write(&[0x55, 0x8B]);
        let mut res = [0; 2];
        self.transport.read(&mut res);
        res
    }

    fn cmd_exec(&mut self, opcode: u8, arg1: u8, arg2: u8) {
        self.cmd_unk_48(0x86);
        self.cmd_unk_58(arg2, arg1, opcode);
        self.cmd_unk_48(0x80);
        self.cmd_unk_4b(0x57, 0x01);
    }

    fn cmd_write_ram(&mut self, address: u8, data: u8) {
        self.cmd_exec(0x75, address, data); // MOV direct, #data
    }

    fn cmd_write_sfr(&mut self, sfr: Sfr, data: u8) {
        self.cmd_write_ram(sfr as u8, data);
    }

    fn cmd_write_xram(&mut self, address: u16, data: u8) {
        let address_bytes = address.to_le_bytes();
        self.cmd_exec(0x90, address_bytes[1], address_bytes[0]); // MOV DPTR, #data16
        self.cmd_exec(0x74, data, 0x00); // MOV A, #data
        self.cmd_exec(0xF0, 0x00, 0x00); // MOVX @DPTR, A
    }

    fn cmd_read_xram(&mut self, address: u16) -> u8 {
        let address_bytes = address.to_le_bytes();
        self.cmd_exec(0x90, address_bytes[1], address_bytes[0]); // MOV DPTR, #data16
        self.cmd_unk_48(0x88);
        self.cmd_unk_48(0x05);
        self.cmd_unk_48(0x88);
        self.cmd_unk_48(0x00);
        self.cmd_unk_48(0x83);
        self.cmd_get_byte()
    }

    fn cmd_get_rom_bank(&mut self) -> u8 {
        self.cmd_read_xram(0xFFFC)
    }

    fn cmd_set_rom_bank(&mut self, bank: u8) {
        self.cmd_write_xram(0xFFFC, bank);
    }

    fn cmd_pre1(&mut self) {
        self.cmd_set_rom_bank(0);
        self.cmd_write_sfr(Sfr::Peram, 0x00);
        self.cmd_write_sfr(Sfr::Peromh, 0x00);
        self.cmd_write_sfr(Sfr::Peroml, 0x00);
        self.cmd_unk_48(0x80);
        self.cmd_unk_4b(0x75, 0x01);
    }

    fn cmd_pre2(&mut self) {
        self.cmd_unk_48(0x80);
        self.cmd_unk_4b(0x55, 0x01);
        self.cmd_write_xram(0xFFF8, 0xC3);
        self.cmd_write_xram(0xFFFB, 0xC3);
    }

    fn cmd_post1(&mut self) {
        self.cmd_unk_48(0x80);
        self.cmd_unk_4b(0x75, 0x01);
    }

    fn cmd_post2(&mut self) {
        self.cmd_unk_48(0x80);
        self.cmd_unk_4b(0x55, 0x01);
        self.cmd_write_xram(0xFFF8, 0xC3);
        self.cmd_write_xram(0xFFFB, 0xC3);
    }

    fn cmd_check_write_finished(&mut self) {
        self.cmd_unk_48(0x81);
        let res = self.cmd_unk_8b();
        assert_eq!(res, [0x5D, 0x01], "Invalid write check result");
    }

    fn cmd_read_ram(&mut self, address: u8) -> u8 {
        self.cmd_exec(0xE5, address, 0x00); // MOV A, direct
        self.cmd_unk_48(0x83);
        self.cmd_get_byte()
    }

    fn cmd_read_sfr(&mut self, sfr: Sfr) -> u8 {
        self.cmd_read_ram(sfr as u8)
    }

    fn cmd_read(&mut self, offset: u16, data: &mut [u8], progress_fn: &dyn Fn(u8)) {
        // Context save
        let old_8e_val = self.cmd_read_sfr(Sfr::Ckon);
        // TODO: CKON only exists for some MCUs, better avoid setting it?
        self.cmd_write_sfr(Sfr::Ckon, 0x71);
        let old_dps_val = self.cmd_read_sfr(Sfr::Dps);
        let old_dpc_val = self.cmd_read_sfr(Sfr::Dpc);
        let old_dpl_val = self.cmd_read_sfr(Sfr::Dpl);
        let old_dph_val = self.cmd_read_sfr(Sfr::Dph);

        // Bulk ROM read
        self.cmd_write_sfr(Sfr::Dps, 0x00);
        self.cmd_write_sfr(Sfr::Dpc, 0x00);
        let offset_bytes = offset.to_le_bytes();
        self.cmd_exec(0x90, offset_bytes[1], offset_bytes[0]);
        self.cmd_unk_48(0x88);
        self.cmd_unk_48(0x04);
        self.cmd_unk_2a();
        let mut progress = 0;
        for i in 0..data.len() {
            data[i] = self.cmd_get_byte();
            let new_progress = i * 100 / data.len();
            if new_progress != progress {
                progress = new_progress;
                progress_fn(progress.try_into().unwrap());
            }
        }
        progress_fn(100);
        self.cmd_unk_2b();
        self.cmd_unk_48(0x88);
        self.cmd_unk_48(0x00);

        // Context restore
        self.cmd_write_sfr(Sfr::Ckon, old_8e_val);
        self.cmd_write_sfr(Sfr::Dps, old_dps_val);
        self.cmd_write_sfr(Sfr::Dpc, old_dpc_val);
        self.cmd_write_sfr(Sfr::Dpl, old_dpl_val);
        self.cmd_write_sfr(Sfr::Dph, old_dph_val);
    }

    fn cmd_erase(&mut self) {
        self.cmd_write_sfr(Sfr::Peroml, 0x0A);
        self.cmd_write_sfr(Sfr::Pecmd, 0x96);
    }

    fn cmd_reload_protection(&mut self) {
        self.cmd_write_xram(0xFFF8, 0x5A);
        self.cmd_write_xram(0xFFFB, 0xA5);
    }

    fn cmd_write_page(&mut self, page: usize, data: &[u8]) {
        assert_eq!(
            data.len(),
            self.page_size,
            "Data length is not equal to page size"
        );
        self.transport.write_batch_begin();
        for (i, byte) in data.iter().enumerate() {
            self.cmd_write_ram(i as u8, *byte);
        }
        self.cmd_write_sfr(Sfr::Peram, 0x00);
        self.cmd_write_sfr(Sfr::Peromh, (page / 8) as u8);
        self.cmd_write_sfr(Sfr::Peroml, ((page % 8) as u8) << 5 | 0x0A);
        self.cmd_write_sfr(Sfr::Pecmd, 0x5A);
        self.transport.write_batch_commit();
    }

    // High-level commands ====================================================

    pub fn reset(&mut self) {
        self.transport.set_reset(true);
        self.sleep_ms(self.reset_duration_ms);
        self.transport.set_reset(false);
    }

    pub fn connect(&mut self) -> u32 {
        self.reset();
        self.sleep_us(self.connect_duration_us);
        self.cmd_connect();
        self.chip_id()
    }

    pub fn chip_id(&mut self) -> u32 {
        let res = self.cmd_chip_id();
        self.cmd_unk_2b();
        res
    }

    pub fn read_flash(&mut self, offset: u16, data: &mut [u8], progress_fn: &dyn Fn(u8)) {
        self.cmd_pre1();
        self.sleep_ms(15);

        self.cmd_pre2();
        self.sleep_ms(15);

        let old_rom_bank = self.cmd_get_rom_bank();
        self.cmd_set_rom_bank(self.rom_bank as u8);
        self.cmd_read(offset, data, progress_fn);
        self.cmd_set_rom_bank(old_rom_bank);
        self.sleep_ms(15);

        self.cmd_post1();
        self.sleep_ms(15);

        self.cmd_post2();
        self.sleep_ms(15);
    }

    pub fn erase_flash(&mut self) {
        if self.rom_bank != RomBank::Main && !self.dangerous_allow_write_non_main_bank {
            panic!("Erasing a non-main ROM bank is not allowed");
        }

        self.cmd_pre1();
        self.sleep_ms(15);

        self.cmd_pre2();
        self.sleep_ms(15);

        let old_rom_bank = self.cmd_get_rom_bank();
        self.cmd_set_rom_bank(self.rom_bank as u8);

        self.cmd_erase();
        self.sleep_ms(15);

        self.cmd_check_write_finished();
        self.sleep_ms(15);

        self.cmd_set_rom_bank(old_rom_bank);

        self.cmd_reload_protection();
        self.sleep_ms(15);

        self.cmd_post1();
        self.sleep_ms(15);

        self.cmd_post2();
        self.sleep_ms(15);
    }

    pub fn write_flash(&mut self, data: &[u8], progress_fn: &dyn Fn(u8)) {
        if self.rom_bank != RomBank::Main && !self.dangerous_allow_write_non_main_bank {
            panic!("Writing to a non-main ROM bank is not allowed");
        }

        self.cmd_pre1();
        self.sleep_ms(15);

        self.cmd_pre2();
        self.sleep_ms(15);

        let old_rom_bank = self.cmd_get_rom_bank();
        self.cmd_set_rom_bank(self.rom_bank as u8);

        let page_count = data.len() / self.page_size;
        for (page, data) in data.chunks(self.page_size).enumerate() {
            self.cmd_write_page(page, data);
            self.sleep_ms(5);

            self.cmd_check_write_finished();
            self.sleep_ms(5);

            progress_fn((page * 100 / page_count).try_into().unwrap());
        }
        progress_fn(100);

        self.cmd_set_rom_bank(old_rom_bank);

        self.cmd_post1();
        self.sleep_ms(15);

        self.cmd_post2();
        self.sleep_ms(15);
    }
}
