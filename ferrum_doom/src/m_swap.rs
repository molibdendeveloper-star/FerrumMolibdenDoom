/// Меняет порядок байтов в 16-битном числе (Little Endian <-> Big Endian)
#[inline]
pub const fn swap_short(x: u16) -> u16 {
    x.swap_bytes()
}

/// Меняет порядок байтов в 32-битном числе (Little Endian <-> Big Endian)
#[inline]
pub const fn swap_long(x: u32) -> u32 {
    x.swap_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_swapping() {
        assert_eq!(swap_short(0x1234), 0x3412);
        assert_eq!(swap_long(0x12345678), 0x78563412);
    }
}
