use crate::m_fixed::Fixed;

pub const BOXTOP: usize = 0;
pub const BOXBOTTOM: usize = 1;
pub const BOXLEFT: usize = 2;
pub const BOXRIGHT: usize = 3;

/// Сбрасывает ограничивающий прямоугольник в начальное (вывернутое) состояние
#[inline]
pub fn m_clear_box(box_arr: &mut [Fixed; 4]) {
    box_arr[BOXTOP] = i32::MIN;
    box_arr[BOXRIGHT] = i32::MIN;
    box_arr[BOXBOTTOM] = i32::MAX;
    box_arr[BOXLEFT] = i32::MAX;
}

/// Расширяет границы прямоугольника, чтобы включить точку (x, y)
#[inline]
pub fn m_add_to_box(box_arr: &mut [Fixed; 4], x: Fixed, y: Fixed) {
    if x < box_arr[BOXLEFT] {
        box_arr[BOXLEFT] = x;
    } else if x > box_arr[BOXRIGHT] {
        box_arr[BOXRIGHT] = x;
    }

    if y < box_arr[BOXBOTTOM] {
        box_arr[BOXBOTTOM] = y;
    } else if y > box_arr[BOXTOP] {
        box_arr[BOXTOP] = y;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::m_fixed::FRACUNIT;

    #[test]
    fn test_bbox_operations() {
        let mut bbox = [0; 4];
        m_clear_box(&mut bbox);

        assert_eq!(bbox[BOXTOP], i32::MIN);
        assert_eq!(bbox[BOXBOTTOM], i32::MAX);

        // Добавляем первую точку (10, 20)
        // Примечание: в оригинальном DOOM из-за конструкции `else if` первая точка
        // обновляет только LEFT и BOTTOM, а вторая расширяет RIGHT и TOP — точно как в C!
        m_add_to_box(&mut bbox, 10 * FRACUNIT, 20 * FRACUNIT);
        assert_eq!(bbox[BOXLEFT], 10 * FRACUNIT);
        assert_eq!(bbox[BOXBOTTOM], 20 * FRACUNIT);

        // Добавляем вторую точку (50, 60)
        m_add_to_box(&mut bbox, 50 * FRACUNIT, 60 * FRACUNIT);
        assert_eq!(bbox[BOXRIGHT], 50 * FRACUNIT);
        assert_eq!(bbox[BOXTOP], 60 * FRACUNIT);
    }
}
