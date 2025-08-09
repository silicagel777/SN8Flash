use crate::error::{Error, Result};
use std::{cmp::max, ffi::OsStr, path::Path};

#[derive(gset::Getset, PartialEq, Eq, Debug)]
pub struct Section {
    #[getset(get_copy, vis = "pub")]
    offset: usize,
    #[getset(get_deref, vis = "pub")]
    data: Vec<u8>,
}

impl Section {
    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn end(&self) -> usize {
        self.offset + self.len()
    }
}

#[derive(gset::Getset, Debug)]
pub struct Firmware {
    #[getset(get_copy, vis = "pub")]
    len: usize,
    #[getset(get_copy, vis = "pub")]
    page_size: usize,
    #[getset(get_deref, vis = "pub")]
    sections: Vec<Section>,
}

impl Firmware {
    pub fn from_file(path: &str, page_size: usize, base_offset: usize) -> Result<Self> {
        let data = std::fs::read(path)?;
        let extension = Path::new(path)
            .extension()
            .and_then(OsStr::to_str)
            .unwrap_or_default();
        match extension {
            "hex" | "ihex" | "ihx" => {
                log::info!("Loading {path} as Intel HEX");
                Self::from_intel_hex(data, page_size, base_offset)
            }
            _ => {
                log::info!("Loading {path} as raw binary");
                Self::from_raw_bytes(data, page_size, base_offset)
            }
        }
    }

    pub fn from_raw_bytes(raw: Vec<u8>, page_size: usize, base_offset: usize) -> Result<Self> {
        let sections = vec![Section {
            offset: base_offset,
            data: raw,
        }];
        let sections = Self::align_and_merge_sections(sections, page_size);
        Ok(Firmware {
            len: Self::sections_len(&sections),
            page_size,
            sections,
        })
    }

    pub fn from_intel_hex(raw: Vec<u8>, page_size: usize, base_offset: usize) -> Result<Self> {
        let mut hex_offset = 0;
        let mut sections = Vec::new();
        let hex_str = std::str::from_utf8(&raw).map_err(Error::IHexDecodeError)?;
        for (i, record) in ihex::Reader::new(hex_str).enumerate() {
            match record {
                Ok(ihex::Record::Data { offset, value }) => {
                    let full_offset = base_offset + hex_offset as usize + offset as usize;
                    sections.push(Section {
                        offset: full_offset,
                        data: value,
                    });
                }
                Ok(ihex::Record::ExtendedSegmentAddress(address)) => {
                    hex_offset = (address as u32) * 16;
                }
                Ok(ihex::Record::ExtendedLinearAddress(address)) => {
                    hex_offset = (address as u32) << 16;
                }
                Ok(ihex::Record::StartSegmentAddress { .. }) => {}
                Ok(ihex::Record::StartLinearAddress(_)) => {}
                Ok(ihex::Record::EndOfFile) => {}
                Err(err) => return Err(Error::IHexParseError(err, i + 1)),
            };
        }

        let sections = Self::align_and_merge_sections(sections, page_size);
        Ok(Firmware {
            len: Self::sections_len(&sections),
            page_size,
            sections,
        })
    }

    fn align_and_merge_sections(mut sections: Vec<Section>, page_size: usize) -> Vec<Section> {
        sections.sort_by_key(|x| x.offset);
        let filler = 0xFF;
        let mut result: Vec<Section> = Vec::new();
        for mut section in sections {
            let aligned_offset = section.offset / page_size * page_size;
            if let Some(prev_section) = result.last_mut()
                && aligned_offset <= prev_section.end()
            {
                let inner_offset = section.offset - prev_section.offset;
                let required_len = inner_offset + section.len();
                let aligned_len = required_len.next_multiple_of(page_size);
                let target_len = max(aligned_len, prev_section.data.len());
                prev_section.data.resize(target_len, filler);
                prev_section.data[inner_offset..required_len].copy_from_slice(&section.data);
            } else {
                let inner_offset = section.offset - aligned_offset;
                let required_len = inner_offset + section.len();
                let aligned_len = required_len.next_multiple_of(page_size);
                let mut new_section = Section {
                    offset: aligned_offset,
                    data: vec![filler; inner_offset],
                };
                new_section.data.append(&mut section.data);
                new_section.data.resize(aligned_len, filler);
                result.push(new_section);
            }
        }
        result
    }

    fn sections_len(sections: &[Section]) -> usize {
        sections.iter().fold(0, |acc, x| acc + x.len())
    }

    pub fn is_empty(&self) -> bool {
        self.len() != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_align_and_merge_sections() {
        let src = vec![
            Section {
                offset: 1,
                data: vec![2],
            },
            Section {
                offset: 2,
                data: vec![3, 4],
            },
            Section {
                offset: 3,
                data: vec![5],
            },
            Section {
                offset: 9,
                data: vec![7],
            },
            Section {
                offset: 5,
                data: vec![6],
            },
            Section {
                offset: 256,
                data: vec![8],
            },
        ];
        let res = Firmware::align_and_merge_sections(src, 2);
        assert_eq!(
            res,
            vec![
                Section {
                    offset: 0,
                    data: vec![0xFF, 2, 3, 5, 0xFF, 6]
                },
                Section {
                    offset: 8,
                    data: vec![0xFF, 7]
                },
                Section {
                    offset: 256,
                    data: vec![8, 0xFF]
                }
            ]
        );
    }
}
