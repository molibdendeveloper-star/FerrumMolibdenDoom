use std::ffi::{CStr, c_char, c_int};
use std::ptr;

// Экспортируем глобальные переменные, так как main() на C записывает в них argc и argv
#[unsafe(no_mangle)]
pub static mut myargc: c_int = 0;

#[unsafe(no_mangle)]
pub static mut myargv: *mut *mut c_char = ptr::null_mut();

/// Проверяет наличие аргумента в командной строке (без учета регистра).
/// Возвращает индекс аргумента (от 1 до myargc - 1) или 0, если аргумент не найден.
pub fn m_check_parm(check: &str) -> i32 {
    unsafe {
        if myargc <= 1 || myargv.is_null() {
            return 0;
        }

        // Превращаем сырой указатель char** в безопасный срез Rust
        let args = std::slice::from_raw_parts(myargv, myargc as usize);

        // Начинаем с 1, пропуская имя самого исполняемого файла (args[0])
        for (i, &arg_ptr) in args.iter().enumerate().skip(1) {
            if arg_ptr.is_null() {
                continue;
            }

            if let Ok(arg_str) = CStr::from_ptr(arg_ptr).to_str() {
                if arg_str.eq_ignore_ascii_case(check) {
                    return i as i32;
                }
            }
        }
    }

    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_check_parm() {
        let arg0 = CString::new("./fmdoom").unwrap();
        let arg1 = CString::new("-file").unwrap();
        let arg2 = CString::new("DOOM.WAD").unwrap();
        let arg3 = CString::new("-SKILL").unwrap();

        let mut ptrs = [
            arg0.as_ptr() as *mut c_char,
            arg1.as_ptr() as *mut c_char,
            arg2.as_ptr() as *mut c_char,
            arg3.as_ptr() as *mut c_char,
        ];

        unsafe {
            myargc = ptrs.len() as c_int;
            myargv = ptrs.as_mut_ptr();

            assert_eq!(m_check_parm("-file"), 1);
            assert_eq!(m_check_parm("-FILE"), 1); // Проверка без учета регистра
            assert_eq!(m_check_parm("-skill"), 3);
            assert_eq!(m_check_parm("-warp"), 0); // Несуществующий флаг

            // Сбрасываем указатели после теста
            myargc = 0;
            myargv = ptr::null_mut();
        }
    }
}
