const PAGE_SIZE: u8 = 32;

pub enum ResetType {
    Rts,
    Dtr,
}

pub struct SonixFlash {
    port: Box<dyn serialport::SerialPort>,
    reset_type: ResetType,
    reset_invert: bool,
    reset_duration_ms: u64,   // 10ms is recommended
    connect_duration_us: u64, // 1666us is recommended
}

impl SonixFlash {
    // Common stuff ===========================================================

    pub fn new(
        port: &str,
        reset_type: ResetType,
        reset_invert: bool,
        reset_duration_ms: u64,
        connect_duration_us: u64,
    ) -> Self {
        let port = serialport::new(port, 750_000)
            .timeout(std::time::Duration::from_millis(50))
            .open()
            .expect("Failed to open serial port");
        SonixFlash {
            port,
            reset_type,
            reset_invert,
            reset_duration_ms,
            connect_duration_us,
        }
    }

    fn sleep_ms(&self, millis: u64) {
        std::thread::sleep(std::time::Duration::from_millis(millis));
    }

    fn sleep_us(&self, micros: u64) {
        std::thread::sleep(std::time::Duration::from_micros(micros));
    }

    fn write(&mut self, data: &[u8]) {
        let written = self
            .port
            .write(data)
            .expect("Failed to write into serial port");
        assert_eq!(written, data.len(), "Some bytes were not written");
        self.port.flush().expect("Failed to flush serial port");
        let mut res = [0];
        for byte in data {
            self.port
                .read_exact(&mut res)
                .expect("Failed to read written data from serial port, check RX+TX connection");
            assert_eq!(
                *byte, res[0],
                "Write/read data mismatch, check RX+TX connection"
            );
        }
    }

    fn read(&mut self, data: &mut [u8]) {
        for i in 0..data.len() {
            self.port
                .read_exact(&mut data[i..i + 1])
                .expect("Failed to read from serial port");
        }
    }

    fn set_reset(&mut self, mut level: bool) {
        if self.reset_invert {
            level = !level;
        }
        match self.reset_type {
            ResetType::Rts => self
                .port
                .write_request_to_send(level)
                .expect("Failed to set RTS pin"),
            ResetType::Dtr => self
                .port
                .write_data_terminal_ready(level)
                .expect("Failed to set DTR pin"),
        }
    }

    // Low-level commands =====================================================

    fn cmd_connect(&mut self) {
        self.write(&[
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
        self.read(&mut res);
        assert_eq!(res, [0xFF; 4], "Invalid handshake response")
    }

    fn cmd_chip_id(&mut self) -> u32 {
        self.write(&[0x55, 0x21, 0x55, 0xA0]);
        let mut res = [0; 4];
        self.read(&mut res);
        u32::from_le_bytes(res)
    }

    fn cmd_get_byte(&mut self) -> u8 {
        self.write(&[0x55, 0x88]);
        let mut res = [0];
        self.read(&mut res);
        res[0]
    }

    fn cmd_unk_2a(&mut self) {
        self.write(&[0x55, 0x2A]);
    }

    fn cmd_unk_2b(&mut self) {
        self.write(&[0x55, 0x2B]);
    }

    fn cmd_unk_48(&mut self, arg1: u8) {
        self.write(&[0x55, 0x48, arg1]);
    }

    fn cmd_unk_4b(&mut self, arg1: u8, arg2: u8) {
        self.write(&[0x55, 0x4B, arg1, arg2]);
    }

    fn cmd_unk_58(&mut self, arg1: u8, arg2: u8, arg3: u8) {
        self.write(&[0x55, 0x58, arg1, arg2, arg3]);
    }

    fn cmd_unk_8b(&mut self) -> [u8; 2] {
        self.write(&[0x55, 0x8B]);
        let mut res = [0; 2];
        self.read(&mut res);
        res
    }

    fn cmd_opcode(&mut self, arg2: u8, arg1: u8, opcode: u8) {
        self.cmd_unk_48(0x86);
        self.cmd_unk_58(arg2, arg1, opcode);
        self.cmd_unk_48(0x80);
        self.cmd_unk_4b(0x57, 0x01);
    }

    fn cmd_opcode_mov_direct(&mut self, data: u8, address: u8) {
        self.cmd_opcode(data, address, 0x75);
    }

    fn cmd_unk_post_write(&mut self) {
        self.cmd_unk_48(0x81);
        assert_eq!(self.cmd_unk_8b(), [0x5D, 0x01]);
    }

    fn cmd_unk_pre1(&mut self) {
        self.cmd_opcode(0xFC, 0xFF, 0x90);
        self.cmd_opcode(0x00, 0x00, 0x74);
        self.cmd_opcode(0x00, 0x00, 0xF0);
        self.cmd_opcode_mov_direct(0x00, 0x97);
        self.cmd_opcode_mov_direct(0x00, 0x96);
        self.cmd_opcode_mov_direct(0x00, 0x95);
        self.cmd_unk_48(0x80);
        self.cmd_unk_4b(0x75, 0x01);
    }

    fn cmd_unk_pre2(&mut self) {
        self.cmd_unk_48(0x80);
        self.cmd_unk_4b(0x55, 0x01);
        self.cmd_opcode(0xF8, 0xFF, 0x90);
        self.cmd_opcode(0x00, 0xC3, 0x74);
        self.cmd_opcode(0x00, 0x00, 0xF0);
        self.cmd_opcode(0xFB, 0xFF, 0x90);
        self.cmd_opcode(0x00, 0xC3, 0x74);
        self.cmd_opcode(0x00, 0x00, 0xF0);
    }

    fn cmd_unk_post1(&mut self) {
        self.cmd_unk_48(0x80);
        self.cmd_unk_4b(0x75, 0x01);
    }

    fn cmd_unk_post2(&mut self) {
        self.cmd_unk_48(0x80);
        self.cmd_unk_4b(0x55, 0x01);
        self.cmd_opcode(0xF8, 0xFF, 0x90);
        self.cmd_opcode(0x00, 0xC3, 0x74);
        self.cmd_opcode(0x00, 0x00, 0xF0);
        self.cmd_opcode(0xFB, 0xFF, 0x90);
        self.cmd_opcode(0x00, 0xC3, 0x74);
        self.cmd_opcode(0x00, 0x00, 0xF0);
    }

    fn cmd_read(&mut self, offset: u16, data: &mut [u8], bank: u8, progress_fn: &dyn Fn(u8)) {
        // Read byte from address 0x8E???
        self.cmd_opcode(0x00, 0x8E, 0xE5);
        self.cmd_unk_48(0x83);
        assert_eq!(self.cmd_get_byte(), 0x71);

        // Write byte to address 0x8E???
        self.cmd_opcode_mov_direct(0x71, 0x8E);

        // Read byte from address 0x92???
        self.cmd_opcode(0x00, 0x92, 0xE5);
        self.cmd_unk_48(0x83);
        assert_eq!(self.cmd_get_byte(), 0x00);

        // Read byte from address 0x93???
        self.cmd_opcode(0x00, 0x93, 0xE5);
        self.cmd_unk_48(0x83);
        assert_eq!(self.cmd_get_byte(), 0x00);

        // Read byte from address 0x82???
        self.cmd_opcode(0x00, 0x82, 0xE5);
        self.cmd_unk_48(0x83);
        assert_eq!(self.cmd_get_byte(), 0xFB);

        // Read byte from address 0x83???
        self.cmd_opcode(0x00, 0x83, 0xE5);
        self.cmd_unk_48(0x83);
        assert_eq!(self.cmd_get_byte(), 0xFF);

        // Bank 0x01 is some sort of bootloader area
        if bank > 0 {
            assert_eq!(self.cmd_get_rom_bank(), 0x00);
            self.cmd_set_rom_bank(bank);
        }

        // Write byte to address 0x92???
        // Write byte to address 0x93???
        self.cmd_opcode_mov_direct(0x00, 0x92);
        self.cmd_opcode_mov_direct(0x00, 0x93);

        // the only stuff that's really necessary start...
        let offset_bytes = offset.to_le_bytes();
        self.cmd_opcode(offset_bytes[0], offset_bytes[1], 0x90);
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
        // the only stuff that's really necessary end...

        // Seems to reuse previously-read numbers?..
        self.cmd_opcode_mov_direct(0x71, 0x8E);

        if bank > 0 {
            assert_eq!(self.cmd_get_rom_bank(), bank);
            self.cmd_set_rom_bank(0);
        }

        self.cmd_opcode_mov_direct(0x00, 0x92);
        self.cmd_opcode_mov_direct(0x00, 0x93);
        self.cmd_opcode_mov_direct(0xFB, 0x82);
        self.cmd_opcode_mov_direct(0xFF, 0x83);
    }

    fn cmd_erase(&mut self) {
        // Why current rom bank is read? Maybe it should be restored after?
        assert_eq!(self.cmd_get_rom_bank(), 0x00);
        self.cmd_set_rom_bank(0);
        self.cmd_opcode_mov_direct(0x0A, 0x95);
        self.cmd_opcode_mov_direct(0x96, 0x94);
    }

    fn cmd_reload_protection(&mut self) {
        self.cmd_opcode(0xF8, 0xFF, 0x90);
        self.cmd_opcode(0x00, 0x5A, 0x74);
        self.cmd_opcode(0x00, 0x00, 0xF0);
        self.cmd_opcode(0xFB, 0xFF, 0x90);
        self.cmd_opcode(0x00, 0xA5, 0x74);
        self.cmd_opcode(0x00, 0x00, 0xF0);
    }

    fn cmd_write_page(&mut self, page: usize, data: &[u8]) {
        assert_eq!(data.len(), PAGE_SIZE as usize);
        for (i, byte) in data.iter().enumerate() {
            self.cmd_opcode_mov_direct(*byte, i as u8);
        }
        self.cmd_opcode_mov_direct(0x00, 0x97);
        self.cmd_opcode_mov_direct((page / 8) as u8, 0x96);
        self.cmd_opcode_mov_direct(((page % 8) as u8) << 5 | 0x0A, 0x95);
        self.cmd_opcode_mov_direct(0x5A, 0x94);
    }

    fn cmd_get_rom_bank(&mut self) -> u8 {
        self.cmd_opcode(0xFC, 0xFF, 0x90); // MOV DPTR, #data16
        self.cmd_unk_48(0x88);
        self.cmd_unk_48(0x05);
        self.cmd_unk_48(0x88);
        self.cmd_unk_48(0x00);
        self.cmd_unk_48(0x83);
        self.cmd_get_byte()
    }

    fn cmd_set_rom_bank(&mut self, bank: u8) {
        self.cmd_opcode(0xFC, 0xFF, 0x90); // MOV DPTR, #data16
        self.cmd_opcode(0x00, bank, 0x74); // MOV A, #data
        self.cmd_opcode(0x00, 0x00, 0xF0); // MOVX @DPTR, A
    }

    // High-level commands ====================================================

    pub fn reset(&mut self) {
        self.set_reset(true);
        self.sleep_ms(self.reset_duration_ms);
        self.set_reset(false);
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

    pub fn read_flash(&mut self, offset: u16, data: &mut [u8], bank: u8, progress_fn: &dyn Fn(u8)) {
        self.cmd_unk_pre1();
        self.sleep_ms(15);

        self.cmd_unk_pre2();
        self.sleep_ms(15);

        self.cmd_read(offset, data, bank, progress_fn);
        self.sleep_ms(15);

        self.cmd_unk_post1();
        self.sleep_ms(15);

        self.cmd_unk_post2();
        self.sleep_ms(15);
    }

    pub fn erase_flash(&mut self) {
        self.cmd_unk_pre1();
        self.sleep_ms(15);

        self.cmd_unk_pre2();
        self.sleep_ms(15);

        self.cmd_erase();
        self.sleep_ms(15);

        self.cmd_unk_post_write();
        self.sleep_ms(15);

        self.cmd_reload_protection();
        self.sleep_ms(15);

        self.cmd_unk_post1();
        self.sleep_ms(15);

        self.cmd_unk_post2();
        self.sleep_ms(15);
    }

    pub fn write_flash(&mut self, data: &[u8], bank: u8, progress_fn: &dyn Fn(u8)) {
        self.cmd_unk_pre1();
        self.sleep_ms(15);

        self.cmd_unk_pre2();
        self.sleep_ms(15);

        if bank > 0 {
            assert_eq!(self.cmd_get_rom_bank(), 0x00);
            self.cmd_set_rom_bank(bank);
        }

        let page_count = data.len() / PAGE_SIZE as usize;
        for (page, data) in data.chunks(PAGE_SIZE as usize).enumerate() {
            self.cmd_write_page(page, data);
            self.sleep_ms(5);

            self.cmd_unk_post_write();
            self.sleep_ms(5);

            progress_fn((page * 100 / page_count).try_into().unwrap());
        }
        progress_fn(100);

        if bank > 0 {
            assert_eq!(self.cmd_get_rom_bank(), bank);
            self.cmd_set_rom_bank(0);
        }

        self.cmd_unk_post1();
        self.sleep_ms(15);

        self.cmd_unk_post2();
        self.sleep_ms(15);
    }

    pub fn test(&mut self) {
        self.cmd_unk_pre1();
        self.sleep_ms(15);

        self.cmd_unk_pre2();
        self.sleep_ms(15);

        // let src = 0x69;
        // // Write byte to address 0x02
        // self.cmd_opcode_mov_direct(src, 0x02);
        // // Read byte from address 0x02
        // self.cmd_opcode(0x00, 0x02, 0xE5); // MOV A, direct
        // // Read reg A
        // self.cmd_unk_48(0x83);
        // let res = self.cmd_get_byte();
        // println!("res is 0x{res:X}");
        // assert_eq!(res, src);

        let mut file = std::fs::File::create("dump_test.bin").unwrap();

        for i in 0..=15u16 {
            use std::io::Write;
            let i_bytes = i.to_le_bytes();
            self.cmd_opcode(i_bytes[0], i_bytes[1], 0x90); // MOV DPTR, #data16
            self.cmd_opcode(0x00, 0x00, 0x74); // MOV A, #data
            self.cmd_opcode(0x00, 0x00, 0x93); // MOVX @DPTR, A
            self.cmd_unk_48(0x83); // Read reg A
            let res = self.cmd_get_byte();
            println!("res is 0x{res:X}");
            file.write_all(&[res]).expect("Failed to write");
        }

        self.cmd_unk_post1();
        self.sleep_ms(15);

        self.cmd_unk_post2();
        self.sleep_ms(15);
    }
}
