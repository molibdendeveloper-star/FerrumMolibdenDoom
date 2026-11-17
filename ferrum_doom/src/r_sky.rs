use std::ffi::c_int;
use crate::m_fixed::FRACUNIT;

#[unsafe(no_mangle)]
pub static mut skyflatnum: c_int = 0;

#[unsafe(no_mangle)]
pub static mut skytexture: c_int = 0;

#[unsafe(no_mangle)]
pub static mut skytexturemid: i32 = 0;

/// Инициализирует вертикальное смещение текстуры неба при изменении размера окна
#[unsafe(no_mangle)]
pub unsafe extern "C" fn R_InitSkyMap() {
    unsafe {
        skytexturemid = 100 * FRACUNIT;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_sky_map() {
        unsafe {
            R_InitSkyMap();
            let mid = skytexturemid;
            assert_eq!(mid, 100 * FRACUNIT);
        }
    }
}
