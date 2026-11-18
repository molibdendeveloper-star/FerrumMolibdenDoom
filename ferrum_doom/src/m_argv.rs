use std::ffi::{CString, c_char};
use std::sync::{OnceLock, RwLock};

static ARGS: OnceLock<RwLock<Vec<String>>> = OnceLock::new();

fn store() -> &'static RwLock<Vec<String>> {
    ARGS.get_or_init(|| RwLock::new(Vec::new()))
}

/// Инициализирует или обновляет список аргументов командной строки (включая @response-файлы).
pub fn set_args(args: Vec<String>) {
    if let Ok(mut guard) = store().write() {
        *guard = args;
    }
}

/// Возвращает количество аргументов командной строки.
pub fn args_len() -> usize {
    store().read().map(|g| g.len()).unwrap_or(0)
}

/// Возвращает аргумент по индексу в виде `String`.
pub fn get_arg(index: usize) -> Option<String> {
    store().read().ok().and_then(|g| g.get(index).cloned())
}

/// Возвращает C-строку для совместимости с legacy-функциями, ожидающими `*mut c_char`.
pub fn get_arg_cptr(index: usize) -> *mut c_char {
    match get_arg(index).and_then(|s| CString::new(s).ok()) {
        Some(cstr) => cstr.into_raw(),
        None => std::ptr::null_mut(),
    }
}

/// Проверяет наличие аргумента в командной строке (без учета регистра).
/// Возвращает индекс аргумента (от 1 до len - 1) или 0, если аргумент не найден.
pub fn m_check_parm(check: &str) -> i32 {
    let Ok(guard) = store().read() else {
        return 0;
    };
    for (i, arg) in guard.iter().enumerate().skip(1) {
        if arg.eq_ignore_ascii_case(check) {
            return i as i32;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_parm() {
        set_args(vec![
            "./fmdoom".into(),
            "-file".into(),
            "DOOM.WAD".into(),
            "-SKILL".into(),
        ]);

        assert_eq!(m_check_parm("-file"), 1);
        assert_eq!(m_check_parm("-FILE"), 1);
        assert_eq!(m_check_parm("-skill"), 3);
        assert_eq!(m_check_parm("-warp"), 0);
        assert_eq!(get_arg(2).as_deref(), Some("DOOM.WAD"));
    }
}
