//! Minimal ZIP writer (stored entries). Fab houses accept uncompressed
//! archives, and this keeps the app free of compression dependencies.

fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (i, e) in table.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 {
                0xedb8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
        *e = c;
    }
    let mut crc = 0xffff_ffffu32;
    for &b in data {
        crc = table[((crc ^ b as u32) & 0xff) as usize] ^ (crc >> 8);
    }
    crc ^ 0xffff_ffff
}

#[derive(Default)]
pub struct ZipWriter {
    data: Vec<u8>,
    central: Vec<u8>,
    count: u16,
}

impl ZipWriter {
    pub fn add(&mut self, name: &str, content: &[u8]) {
        let offset = self.data.len() as u32;
        let crc = crc32(content);
        let size = content.len() as u32;
        let name_b = name.as_bytes();
        // DOS date 2026-01-01 00:00.
        let (time, date) = (0u16, ((2026 - 1980) << 9 | 1 << 5 | 1) as u16);
        let mut local = Vec::new();
        local.extend(0x0403_4b50u32.to_le_bytes());
        local.extend(20u16.to_le_bytes());
        local.extend(0x0800u16.to_le_bytes()); // UTF-8 names
        local.extend(0u16.to_le_bytes()); // stored
        local.extend(time.to_le_bytes());
        local.extend(date.to_le_bytes());
        local.extend(crc.to_le_bytes());
        local.extend(size.to_le_bytes());
        local.extend(size.to_le_bytes());
        local.extend((name_b.len() as u16).to_le_bytes());
        local.extend(0u16.to_le_bytes());
        local.extend(name_b);
        self.data.extend(&local);
        self.data.extend(content);

        let c = &mut self.central;
        c.extend(0x0201_4b50u32.to_le_bytes());
        c.extend(20u16.to_le_bytes());
        c.extend(20u16.to_le_bytes());
        c.extend(0x0800u16.to_le_bytes());
        c.extend(0u16.to_le_bytes());
        c.extend(time.to_le_bytes());
        c.extend(date.to_le_bytes());
        c.extend(crc.to_le_bytes());
        c.extend(size.to_le_bytes());
        c.extend(size.to_le_bytes());
        c.extend((name_b.len() as u16).to_le_bytes());
        c.extend([0u8; 8]); // extra, comment, disk, internal attrs
        c.extend(0u32.to_le_bytes()); // external attrs
        c.extend(offset.to_le_bytes());
        c.extend(name_b);
        self.count += 1;
    }

    pub fn finish(mut self) -> Vec<u8> {
        let cd_offset = self.data.len() as u32;
        let cd_size = self.central.len() as u32;
        self.data.extend(&self.central);
        self.data.extend(0x0605_4b50u32.to_le_bytes());
        self.data.extend([0u8; 4]);
        self.data.extend(self.count.to_le_bytes());
        self.data.extend(self.count.to_le_bytes());
        self.data.extend(cd_size.to_le_bytes());
        self.data.extend(cd_offset.to_le_bytes());
        self.data.extend(0u16.to_le_bytes());
        self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_matches_reference() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
    }

    #[test]
    fn archive_structure() {
        let mut z = ZipWriter::default();
        z.add("a.txt", b"hello");
        z.add("b/c.txt", b"world!");
        let bytes = z.finish();
        assert_eq!(&bytes[0..4], &[0x50, 0x4b, 0x03, 0x04]);
        let eocd = bytes.len() - 22;
        assert_eq!(&bytes[eocd..eocd + 4], &[0x50, 0x4b, 0x05, 0x06]);
        assert_eq!(u16::from_le_bytes([bytes[eocd + 10], bytes[eocd + 11]]), 2);
    }
}
