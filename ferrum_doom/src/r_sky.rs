use std::sync::atomic::{AtomicI32, Ordering};
use crate::m_fixed::FRACUNIT;

static SKY_FLAT_NUM: AtomicI32 = AtomicI32::new(0);
static SKY_TEXTURE: AtomicI32 = AtomicI32::new(0);
static SKY_TEXTURE_MID: AtomicI32 = AtomicI32::new(0);

#[inline]
pub fn get_sky_flat_num() -> i32 {
    SKY_FLAT_NUM.load(Ordering::Relaxed)
}

#[inline]
pub fn set_sky_flat_num(val: i32) {
    SKY_FLAT_NUM.store(val, Ordering::Relaxed);
}

#[inline]
pub fn get_sky_texture() -> i32 {
    SKY_TEXTURE.load(Ordering::Relaxed)
}

#[inline]
pub fn set_sky_texture(val: i32) {
    SKY_TEXTURE.store(val, Ordering::Relaxed);
}

#[inline]
pub fn get_sky_texture_mid() -> i32 {
    SKY_TEXTURE_MID.load(Ordering::Relaxed)
}

#[inline]
pub fn set_sky_texture_mid(val: i32) {
    SKY_TEXTURE_MID.store(val, Ordering::Relaxed);
}

/// Инициализирует вертикальное смещение текстуры неба при изменении размера окна
pub fn r_init_sky_map() {
    set_sky_texture_mid(100 * FRACUNIT);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_sky_map() {
        r_init_sky_map();
        assert_eq!(get_sky_texture_mid(), 100 * FRACUNIT);
    }
}
