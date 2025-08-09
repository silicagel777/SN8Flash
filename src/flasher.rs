use crate::{
    error::{Error, Result},
    firmware::Firmware,
    transport::Transport,
};

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Debug)]
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
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum RomBank {
    Main = 0,
    Boot = 1,
}

#[derive(gset::Getset)]
pub struct Flasher {
    // Inner fields
    transport: Box<dyn Transport>,
    connected: bool,
    batch_counter: u64,
    batch_data: Vec<u8>,

    #[getset(get_copy, vis = "pub")]
    #[getset(set, vis = "pub")]
    final_reset: bool,

    #[getset(get_copy, vis = "pub")]
    #[getset(set, vis = "pub")]
    reset_duration_ms: u64,

    #[getset(get_copy, vis = "pub")]
    #[getset(set, vis = "pub")]
    connect_duration_us: u64,

    #[getset(get_copy, vis = "pub")]
    #[getset(set, vis = "pub")]
    rom_bank: RomBank,

    #[getset(get_copy, vis = "pub")]
    #[getset(set, vis = "pub")]
    dangerous_allow_write_non_main_bank: bool,
}

impl Flasher {
    // Common stuff ===========================================================

    pub fn new(transport: Box<dyn Transport>) -> Self {
        Flasher {
            transport,
            connected: false,
            batch_counter: 0,
            batch_data: Vec::new(),
            final_reset: true,
            rom_bank: RomBank::Main,
            reset_duration_ms: 10,
            connect_duration_us: 1666,
            dangerous_allow_write_non_main_bank: false,
        }
    }

    fn sleep_ms(&self, millis: u64) {
        std::thread::sleep(std::time::Duration::from_millis(millis));
    }

    fn sleep_us(&self, micros: u64) {
        std::thread::sleep(std::time::Duration::from_micros(micros));
    }

    fn write_batch_begin(&mut self) {
        self.batch_counter += 1;
    }

    fn write_batch_commit(&mut self) -> Result<()> {
        self.batch_counter -= 1;
        if self.batch_counter == 0 {
            let batch_data = std::mem::take(&mut self.batch_data);
            self.write(&batch_data)?;
        }
        Ok(())
    }

    fn write(&mut self, data: &[u8]) -> Result<()> {
        if self.batch_counter > 0 {
            self.batch_data.extend(data);
        } else {
            self.transport.write(data)?;
        }
        Ok(())
    }

    fn read(&mut self, data: &mut [u8]) -> Result<()> {
        if self.batch_counter != 0 {
            panic!("Can't read during write batch")
        }
        self.transport.read(data)?;
        Ok(())
    }

    // Low-level commands =====================================================

    fn cmd_connect(&mut self) -> Result<()> {
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
        ])?;
        let mut res = [0; 4];
        self.read(&mut res)?;
        if res != [0xFF; 4] {
            return Err(Error::HandshakeError(res));
        }
        Ok(())
    }

    fn cmd_chip_id(&mut self) -> Result<u32> {
        self.write(&[0x55, 0x21])?;
        let res = self.cmd_get_u32()?;
        self.cmd_unk_2b()?;
        Ok(res)
    }

    fn cmd_get_u8(&mut self) -> Result<u8> {
        self.write(&[0x55, 0x88])?;
        let mut res = [0];
        self.read(&mut res)?;
        Ok(res[0])
    }

    fn cmd_get_u16(&mut self) -> Result<u16> {
        self.write(&[0x55, 0x8B])?;
        let mut res = [0; 2];
        self.read(&mut res)?;
        Ok(u16::from_le_bytes(res))
    }

    fn cmd_get_u32(&mut self) -> Result<u32> {
        self.write(&[0x55, 0xA0])?;
        let mut res = [0; 4];
        self.read(&mut res)?;
        Ok(u32::from_le_bytes(res))
    }

    fn cmd_unk_2a(&mut self) -> Result<()> {
        self.write(&[0x55, 0x2A])?;
        Ok(())
    }

    fn cmd_unk_2b(&mut self) -> Result<()> {
        self.write(&[0x55, 0x2B])?;
        Ok(())
    }

    fn cmd_unk_48(&mut self, arg1: u8) -> Result<()> {
        self.write(&[0x55, 0x48, arg1])?;
        Ok(())
    }

    fn cmd_unk_4b(&mut self, arg1: u8, arg2: u8) -> Result<()> {
        self.write(&[0x55, 0x4B, arg1, arg2])?;
        Ok(())
    }

    fn cmd_unk_58(&mut self, arg1: u8, arg2: u8, arg3: u8) -> Result<()> {
        self.write(&[0x55, 0x58, arg1, arg2, arg3])?;
        Ok(())
    }

    fn cmd_exec_op(&mut self, opcode: u8, arg1: u8, arg2: u8) -> Result<()> {
        self.cmd_unk_48(0x86)?;
        self.cmd_unk_58(arg2, arg1, opcode)?;
        self.cmd_unk_48(0x80)?;
        self.cmd_unk_4b(0x57, 0x01)?;
        Ok(())
    }

    fn cmd_write_ram(&mut self, address: u8, data: u8) -> Result<()> {
        self.cmd_exec_op(0x75, address, data)?; // MOV direct, #data
        Ok(())
    }

    fn cmd_read_ram(&mut self, address: u8) -> Result<u8> {
        self.cmd_exec_op(0xE5, address, 0x00)?; // MOV A, direct
        self.cmd_unk_48(0x83)?;
        self.cmd_get_u8()
    }

    fn cmd_write_sfr(&mut self, sfr: Sfr, data: u8) -> Result<()> {
        self.cmd_write_ram(sfr as u8, data)?;
        Ok(())
    }

    fn cmd_read_sfr(&mut self, sfr: Sfr) -> Result<u8> {
        self.cmd_read_ram(sfr as u8)
    }

    fn cmd_write_xram(&mut self, address: u16, data: u8) -> Result<()> {
        let address_bytes = address.to_le_bytes();
        self.cmd_exec_op(0x90, address_bytes[1], address_bytes[0])?; // MOV DPTR, #data16
        self.cmd_exec_op(0x74, data, 0x00)?; // MOV A, #data
        self.cmd_exec_op(0xF0, 0x00, 0x00)?; // MOVX @DPTR, A
        Ok(())
    }

    fn cmd_read_xram(&mut self, address: u16) -> Result<u8> {
        let address_bytes = address.to_le_bytes();
        self.cmd_exec_op(0x90, address_bytes[1], address_bytes[0])?; // MOV DPTR, #data16
        self.cmd_unk_48(0x88)?;
        self.cmd_unk_48(0x05)?;
        self.cmd_unk_48(0x88)?;
        self.cmd_unk_48(0x00)?;
        self.cmd_unk_48(0x83)?;
        self.cmd_get_u8()
    }

    fn cmd_get_rom_bank(&mut self) -> Result<u8> {
        self.cmd_read_xram(0xFFFC)
    }

    fn cmd_set_rom_bank(&mut self, bank: u8) -> Result<()> {
        self.cmd_write_xram(0xFFFC, bank)?;
        Ok(())
    }

    fn cmd_pre1(&mut self) -> Result<()> {
        self.cmd_set_rom_bank(0)?;
        self.cmd_write_sfr(Sfr::Peram, 0x00)?;
        self.cmd_write_sfr(Sfr::Peromh, 0x00)?;
        self.cmd_write_sfr(Sfr::Peroml, 0x00)?;
        self.cmd_unk_48(0x80)?;
        self.cmd_unk_4b(0x75, 0x01)?;
        Ok(())
    }

    fn cmd_pre2(&mut self) -> Result<()> {
        self.cmd_unk_48(0x80)?;
        self.cmd_unk_4b(0x55, 0x01)?;
        self.cmd_write_xram(0xFFF8, 0xC3)?;
        self.cmd_write_xram(0xFFFB, 0xC3)?;
        Ok(())
    }

    fn cmd_post1(&mut self) -> Result<()> {
        self.cmd_unk_48(0x80)?;
        self.cmd_unk_4b(0x75, 0x01)?;
        Ok(())
    }

    fn cmd_post2(&mut self) -> Result<()> {
        self.cmd_unk_48(0x80)?;
        self.cmd_unk_4b(0x55, 0x01)?;
        self.cmd_write_xram(0xFFF8, 0xC3)?;
        self.cmd_write_xram(0xFFFB, 0xC3)?;
        Ok(())
    }

    fn cmd_check_write_finished(&mut self) -> Result<()> {
        self.cmd_unk_48(0x81)?;
        let res = self.cmd_get_u16()?;
        if res != 0x015D {
            return Err(Error::WriteCheckError(res));
        }
        Ok(())
    }

    fn cmd_read(&mut self, offset: u16, data: &mut [u8], progress: &dyn Fn(u64)) -> Result<()> {
        // Context save
        let old_ckon_val = self.cmd_read_sfr(Sfr::Ckon)?;
        // TODO: CKON only exists for some MCUs, better avoid setting it?
        self.cmd_write_sfr(Sfr::Ckon, 0x71)?;
        let old_dps_val = self.cmd_read_sfr(Sfr::Dps)?;
        let old_dpc_val = self.cmd_read_sfr(Sfr::Dpc)?;
        let old_dpl_val = self.cmd_read_sfr(Sfr::Dpl)?;
        let old_dph_val = self.cmd_read_sfr(Sfr::Dph)?;

        // Bulk ROM read
        self.cmd_write_sfr(Sfr::Dps, 0x00)?;
        self.cmd_write_sfr(Sfr::Dpc, 0x00)?;
        let offset_bytes = offset.to_le_bytes();
        self.cmd_exec_op(0x90, offset_bytes[1], offset_bytes[0])?;
        self.cmd_unk_48(0x88)?;
        self.cmd_unk_48(0x04)?;
        self.cmd_unk_2a()?;
        for byte in data.iter_mut() {
            *byte = self.cmd_get_u8()?;
            progress(1);
        }
        self.cmd_unk_2b()?;
        self.cmd_unk_48(0x88)?;
        self.cmd_unk_48(0x00)?;

        // Context restore
        self.cmd_write_sfr(Sfr::Ckon, old_ckon_val)?;
        self.cmd_write_sfr(Sfr::Dps, old_dps_val)?;
        self.cmd_write_sfr(Sfr::Dpc, old_dpc_val)?;
        self.cmd_write_sfr(Sfr::Dpl, old_dpl_val)?;
        self.cmd_write_sfr(Sfr::Dph, old_dph_val)?;

        Ok(())
    }

    fn cmd_erase(&mut self) -> Result<()> {
        self.cmd_write_sfr(Sfr::Peroml, 0x0A)?;
        self.cmd_write_sfr(Sfr::Pecmd, 0x96)?;
        Ok(())
    }

    fn cmd_reload_protection(&mut self) -> Result<()> {
        self.cmd_write_xram(0xFFF8, 0x5A)?;
        self.cmd_write_xram(0xFFFB, 0xA5)?;
        Ok(())
    }

    fn cmd_write_page(&mut self, page: usize, data: &[u8]) -> Result<()> {
        self.write_batch_begin();
        for (i, byte) in data.iter().enumerate() {
            self.cmd_write_ram(i as u8, *byte)?;
        }
        self.cmd_write_sfr(Sfr::Peram, 0x00)?;
        self.cmd_write_sfr(Sfr::Peromh, (page / 8) as u8)?;
        self.cmd_write_sfr(Sfr::Peroml, ((page % 8) as u8) << 5 | 0x0A)?;
        self.cmd_write_sfr(Sfr::Pecmd, 0x5A)?;
        self.write_batch_commit()?;
        Ok(())
    }

    // High-level commands ====================================================

    pub fn reset(&mut self) -> Result<()> {
        self.transport.set_reset(true)?;
        self.connected = false;
        self.sleep_ms(self.reset_duration_ms);
        self.transport.set_reset(false)?;
        Ok(())
    }

    pub fn connect(&mut self) -> Result<u32> {
        self.reset()?;
        self.sleep_us(self.connect_duration_us);
        self.cmd_connect()?;
        self.connected = true;
        self.chip_id()
    }

    pub fn chip_id(&mut self) -> Result<u32> {
        self.cmd_chip_id()
    }

    pub fn read_flash(
        &mut self,
        offset: u16,
        data: &mut [u8],
        progress: &dyn Fn(u64),
    ) -> Result<()> {
        self.cmd_pre1()?;
        self.sleep_ms(15);

        self.cmd_pre2()?;
        self.sleep_ms(15);

        let old_rom_bank = self.cmd_get_rom_bank()?;
        self.cmd_set_rom_bank(self.rom_bank as u8)?;
        self.cmd_read(offset, data, progress)?;
        self.cmd_set_rom_bank(old_rom_bank)?;
        self.sleep_ms(15);

        self.cmd_post1()?;
        self.sleep_ms(15);

        self.cmd_post2()?;
        self.sleep_ms(15);

        Ok(())
    }

    pub fn erase_flash(&mut self) -> Result<()> {
        if self.rom_bank != RomBank::Main && !self.dangerous_allow_write_non_main_bank {
            return Err(Error::NonMainBankErase);
        }

        self.cmd_pre1()?;
        self.sleep_ms(15);

        self.cmd_pre2()?;
        self.sleep_ms(15);

        let old_rom_bank = self.cmd_get_rom_bank()?;
        self.cmd_set_rom_bank(self.rom_bank as u8)?;

        self.cmd_erase()?;
        self.sleep_ms(15);

        self.cmd_check_write_finished()?;
        self.sleep_ms(15);

        self.cmd_set_rom_bank(old_rom_bank)?;

        self.cmd_reload_protection()?;
        self.sleep_ms(15);

        self.cmd_post1()?;
        self.sleep_ms(15);

        self.cmd_post2()?;
        self.sleep_ms(15);

        Ok(())
    }

    pub fn write_flash(&mut self, firmware: &Firmware, progress: &dyn Fn(u64)) -> Result<()> {
        if self.rom_bank != RomBank::Main && !self.dangerous_allow_write_non_main_bank {
            return Err(Error::NonMainBankErase);
        }

        self.cmd_pre1()?;
        self.sleep_ms(15);

        self.cmd_pre2()?;
        self.sleep_ms(15);

        let old_rom_bank = self.cmd_get_rom_bank()?;
        self.cmd_set_rom_bank(self.rom_bank as u8)?;

        for section in firmware.sections() {
            for (i, data) in section.data().chunks(firmware.page_size()).enumerate() {
                let page = section.offset() / firmware.page_size() + i;
                log::debug!("Writing page {}, size {}", page, firmware.page_size());
                self.cmd_write_page(page, data)?;
                self.sleep_ms(5);

                self.cmd_check_write_finished()?;
                self.sleep_ms(5);

                progress(data.len() as _);
            }
        }

        self.cmd_set_rom_bank(old_rom_bank)?;

        self.cmd_post1()?;
        self.sleep_ms(15);

        self.cmd_post2()?;
        self.sleep_ms(15);

        Ok(())
    }

    pub fn verify_flash(&mut self, firmware: &Firmware, progress: &dyn Fn(u64)) -> Result<()> {
        self.cmd_pre1()?;
        self.sleep_ms(15);

        self.cmd_pre2()?;
        self.sleep_ms(15);

        let old_rom_bank = self.cmd_get_rom_bank()?;
        self.cmd_set_rom_bank(self.rom_bank as u8)?;

        let mut errors: Vec<usize> = Vec::new();
        for section in firmware.sections() {
            let mut verify = vec![0; section.len()];
            self.cmd_read(section.offset() as u16, verify.as_mut_slice(), progress)?;
            let errors_it = std::iter::zip(section.data(), verify)
                .enumerate()
                .filter(|(_, (x, y))| *x != y)
                .map(|(j, _)| j + section.offset());
            errors.extend(errors_it);
        }

        if !errors.is_empty() {
            return Err(Error::VerifyMismatch(errors));
        }

        self.cmd_set_rom_bank(old_rom_bank)?;

        self.cmd_post1()?;
        self.sleep_ms(15);

        self.cmd_post2()?;
        self.sleep_ms(15);

        Ok(())
    }
}

impl Drop for Flasher {
    fn drop(&mut self) {
        if self.final_reset && self.connected {
            log::debug!("Running final reset");
            if let Err(err) = self.reset() {
                log::debug!("Error running final reset: {err:?}");
            }
        }
    }
}
