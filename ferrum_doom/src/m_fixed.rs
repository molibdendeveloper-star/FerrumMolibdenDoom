pub type Fixed = i32;

pub const FRACBITS: u32 = 16;
pub const FRACUNIT: Fixed = 1 << FRACBITS;

/// Умножение двух чисел с фиксированной точкой (16.16)
#[inline]
pub fn fixed_mul(a: Fixed, b: Fixed) -> Fixed {
    (((a as i64) * (b as i64)) >> FRACBITS) as Fixed
}

/// Деление чисел с фиксированной точкой с защитой от переполнения
#[inline]
pub fn fixed_div(a: Fixed, b: Fixed) -> Fixed {
    if (a.unsigned_abs() >> 14) >= b.unsigned_abs() {
        return if (a ^ b) < 0 { i32::MIN } else { i32::MAX };
    }
    fixed_div2(a, b)
}

/// Внутренняя функция деления (аналог версии с плавающей точкой из Doom)
#[inline]
fn fixed_div2(a: Fixed, b: Fixed) -> Fixed {
    // Альтернативный целочисленный вариант (из блока #if 0):
    // let c = ((a as i64) << 16) / (b as i64);
    // return c as Fixed;

    let c = (a as f64) / (b as f64) * (FRACUNIT as f64);

    if c >= 2147483648.0 || c < -2147483648.0 {
        panic!("FixedDiv: divide by zero"); // Аналог I_Error
    }

    c as Fixed
}
