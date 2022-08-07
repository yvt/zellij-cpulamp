//! Utilities for the plugin
mod process;
pub mod slist;
pub mod sysinfo;

pub fn bitmap_to_braille(bitmap: u8) -> char {
    // ISO/TR 11548-1 dot numbering      Our fragment bit positions:
    // (mapped to bit positions of
    // Unicode code points):
    //
    //             0  3                             0  1
    //             1  4                             2  3
    //             2  5                             4  5
    //             6  7                             6  7
    //
    // Notice that only the positions 1–4 differ between them. Therefore, we
    // use a 16x4-bit LUT to remap these bits.
    const LUT: u64 = {
        let mut lut = 0u64;
        let mut i = 0;
        while i < 16 {
            let b1 = i & 0b0001;
            let b2 = i & 0b0010;
            let b3 = i & 0b0100;
            let b4 = i & 0b1000;
            let uni_b4321 = (b1 << 2) | (b2 >> 1) | (b3 << 1) | (b4 >> 2);
            lut |= uni_b4321 << (i * 4);
            i += 1;
        }
        lut.rotate_left(1)
    };

    let uni_b7650 = bitmap & 0b11100001;
    let b4321 = (bitmap & 0b00011110) >> 1;
    let uni_b4321 = LUT.rotate_right((b4321 * 4) as _) & 0b11110;
    let uni = uni_b7650 as u32 | uni_b4321 as u32;

    const BRAILLE_BLANK: char = '\u{2800}'; // '⠀'
    char::from_u32(BRAILLE_BLANK as u32 + uni).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn braille() {
        assert_eq!(bitmap_to_braille(0b00_00_00_00), '⠀');
        assert_eq!(bitmap_to_braille(0b00_00_00_01), '⠁');
        assert_eq!(bitmap_to_braille(0b00_00_00_10), '⠈');
        assert_eq!(bitmap_to_braille(0b00_00_01_00), '⠂');
        assert_eq!(bitmap_to_braille(0b00_00_10_00), '⠐');
        assert_eq!(bitmap_to_braille(0b00_01_00_00), '⠄');
        assert_eq!(bitmap_to_braille(0b00_10_00_00), '⠠');
        assert_eq!(bitmap_to_braille(0b01_00_00_00), '⡀');
        assert_eq!(bitmap_to_braille(0b10_00_00_00), '⢀');
        for i in 0..=255 {
            bitmap_to_braille(i);
        }
    }
}
