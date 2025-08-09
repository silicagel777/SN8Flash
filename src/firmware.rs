use std::{cmp::max, ffi::OsStr, path::Path};

pub fn load_firmware(path: &str) -> Vec<u8> {
    let data = std::fs::read(path).unwrap();
    let extension = Path::new(path)
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or_default();
    match extension {
        "hex" | "ihex" | "hexa" => {
            log::info!("Loaded {path} as Intel HEX");
            ihex_to_bin(&data)
        }
        _ => {
            log::info!("Loaded {path} as raw binary");
            data
        }
    }
}

fn ihex_to_bin(data: &[u8]) -> Vec<u8> {
    let mut res = Vec::new();
    let mut base_offset = 0u32;
    for record in ihex::Reader::new(str::from_utf8(data).unwrap()) {
        match record.unwrap() {
            ihex::Record::Data { offset, value } => {
                let full_offset = base_offset as usize + offset as usize;
                res.resize(max(res.len(), full_offset + value.len()), 0xFF);
                res[full_offset..full_offset + value.len()].copy_from_slice(&value);
            }
            ihex::Record::ExtendedSegmentAddress(address) => {
                base_offset = (address as u32) * 16;
            }
            ihex::Record::ExtendedLinearAddress(address) => {
                base_offset = (address as u32) << 16;
            }
            ihex::Record::StartSegmentAddress { .. } => {}
            ihex::Record::StartLinearAddress(_) => {}
            ihex::Record::EndOfFile => {}
        };
    }
    res
}
