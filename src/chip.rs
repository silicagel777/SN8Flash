use std::fmt::Display;

#[derive(gset::Getset, PartialEq, Eq, Debug, Clone, Copy)]
pub struct ChipInfo {
    #[getset(get_copy, vis = "pub")]
    series: &'static str,
    #[getset(get_copy, vis = "pub")]
    flash_size: u32,
    #[getset(get_copy, vis = "pub")]
    page_size: u8,
}

impl ChipInfo {
    pub fn from_chip_id(chip_id: u32) -> Option<Self> {
        // These values are hand-copied from "SNLINK_C51.INI" of official Keil plug-in
        match chip_id {
            0x1110..0x1120 => Some(Self {
                series: "SNPD5111",
                flash_size: 0x4000,
                page_size: 0x20,
            }),
            0x2710..0x2720 => Some(Self {
                series: "SN8F5283",
                flash_size: 0x4000,
                page_size: 0x20,
            }),
            0x6100..0x6110 => Some(Self {
                series: "SN8F5701",
                flash_size: 0x1000,
                page_size: 0x20,
            }),
            0x6110..0x6120 => Some(Self {
                series: "SN8F5721",
                flash_size: 0x1000,
                page_size: 0x20,
            }),
            0x6200..0x6216 => Some(Self {
                series: "SN8F5702",
                flash_size: 0x1000,
                page_size: 0x20,
            }),
            0x6216..0x6220 => Some(Self {
                series: "SN8F5702A",
                flash_size: 0x1000,
                page_size: 0x20,
            }),
            0x6220..0x6230 => Some(Self {
                series: "SN8F5732",
                flash_size: 0x4000,
                page_size: 0x20,
            }),
            0x6240..0x6250 => Some(Self {
                series: "SN8F5762",
                flash_size: 0x4800,
                page_size: 0x40,
            }),
            0x6260..0x6270 => Some(Self {
                series: "SN8F5782",
                flash_size: 0x10000,
                page_size: 0x40,
            }),
            0x6270..0x6280 => Some(Self {
                series: "SN8F5602",
                flash_size: 0x4800,
                page_size: 0x40,
            }),
            0x6300..0x6310 => Some(Self {
                series: "SN8F5703",
                flash_size: 0x2000,
                page_size: 0x20,
            }),
            0x6310..0x6330 => Some(Self {
                series: "SN8F5713",
                flash_size: 0x2000,
                page_size: 0x20,
            }),
            0x6330..0x6336 => Some(Self {
                series: "SN8F5703",
                flash_size: 0x2000,
                page_size: 0x20,
            }),
            0x6336..0x6340 => Some(Self {
                series: "SN8F5703A",
                flash_size: 0x2000,
                page_size: 0x20,
            }),
            0x6400..0x6410 => Some(Self {
                series: "SN8F5754",
                flash_size: 0x4000,
                page_size: 0x20,
            }),
            0x6700..0x6720 => Some(Self {
                series: "SN8F5708",
                flash_size: 0x4000,
                page_size: 0x20,
            }),
            0x8401..0x8410 => Some(Self {
                series: "SN8F5804",
                flash_size: 0x2000,
                page_size: 0x20,
            }),
            0x8410..0x8420 => Some(Self {
                series: "SN8F5814",
                flash_size: 0x4000,
                page_size: 0x20,
            }),
            0x8420..0x8430 => Some(Self {
                series: "SN8F5804A",
                flash_size: 0x2000,
                page_size: 0x20,
            }),
            0x8500..0x8510 => Some(Self {
                series: "SN8F5835",
                flash_size: 0x8000,
                page_size: 0x40,
            }),
            0x8700..0x8710 => Some(Self {
                series: "SN8F5858",
                flash_size: 0x4000,
                page_size: 0x20,
            }),
            0x8800..0x8820 => Some(Self {
                series: "SN8F5829",
                flash_size: 0x8000,
                page_size: 0x40,
            }),
            0x8820..0x8830 => Some(Self {
                series: "SN8F5840",
                flash_size: 0x08000,
                page_size: 0x40,
            }),
            0x8830..0x8840 => Some(Self {
                series: "SN8F5869",
                flash_size: 0x10000,
                page_size: 0x40,
            }),
            0x9901..0x9910 => Some(Self {
                series: "SN8F5900",
                flash_size: 0x10000,
                page_size: 0x40,
            }),
            0x9910..0x9920 => Some(Self {
                series: "SN8F5910",
                flash_size: 0x08000,
                page_size: 0x40,
            }),
            0x9920..0x9930 => Some(Self {
                series: "SN8F5900A",
                flash_size: 0x10000,
                page_size: 0x40,
            }),
            0x9930..0x9940 => Some(Self {
                series: "SN8F5930",
                flash_size: 0x20000,
                page_size: 0x40,
            }),
            0x9940..0x9950 => Some(Self {
                series: "SN8F5920",
                flash_size: 0x08000,
                page_size: 0x40,
            }),
            0x9950..0x9960 => Some(Self {
                series: "SN8F5950",
                flash_size: 0x20000,
                page_size: 0x40,
            }),
            0x9960..0x9970 => Some(Self {
                series: "SN8F5960",
                flash_size: 0x10000,
                page_size: 0x40,
            }),
            0x99A0..0x99B0 => Some(Self {
                series: "SN8F5900B",
                flash_size: 0x10000,
                page_size: 0x40,
            }),
            0x99B0..0x99C0 => Some(Self {
                series: "SN8F5940",
                flash_size: 0x20000,
                page_size: 0x40,
            }),
            0x99C0..0x99D0 => Some(Self {
                series: "SN8F5900C",
                flash_size: 0x10000,
                page_size: 0x40,
            }),
            _ => None,
        }
    }
}

impl Display for ChipInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} series, {} bytes flash, {}-byte pages",
            self.series(),
            self.flash_size(),
            self.page_size(),
        )
    }
}
