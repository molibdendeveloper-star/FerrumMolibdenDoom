use std::ffi::{CString, c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr;

pub const ZONEID: i32 = 0x1d4a11;
pub const MINFRAGMENT: i32 = 64;

// Константы тегов из z_zone.h
pub const PU_STATIC: c_int = 1;
pub const PU_SOUND: c_int = 2;
pub const PU_MUSIC: c_int = 3;
pub const PU_DAVE: c_int = 4;
pub const PU_LEVEL: c_int = 50;
pub const PU_LEVSPEC: c_int = 51;
pub const PU_PURGELEVEL: c_int = 100;
pub const PU_CACHE: c_int = 101;

// Импортируем функции из C-части движка
unsafe extern "C" {
    fn I_ZoneBase(size: *mut c_int) -> *mut u8;
    fn I_Error(error: *const c_char, ...);
}

#[repr(C)]
pub struct MemBlock {
    pub size: c_int,
    pub user: *mut *mut c_void,
    pub tag: c_int,
    pub id: c_int,
    pub next: *mut MemBlock,
    pub prev: *mut MemBlock,
}

#[repr(C)]
pub struct MemZone {
    pub size: c_int,
    pub blocklist: MemBlock,
    pub rover: *mut MemBlock,
}

#[unsafe(no_mangle)]
pub static mut mainzone: *mut MemZone = ptr::null_mut();

/// Вспомогательная функция для вызова C-функции I_Error из Rust
fn doom_error(msg: &str) -> ! {
    let c_msg = CString::new(msg).unwrap_or_else(|_| CString::new("Zone memory error").unwrap());
    unsafe {
        I_Error(c_msg.as_ptr());
    }
    unreachable!("I_Error should never return");
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Z_ClearZone(zone: *mut MemZone) {
    unsafe {
        let block = (zone as *mut u8).add(size_of::<MemZone>()) as *mut MemBlock;

        (*zone).blocklist.next = block;
        (*zone).blocklist.prev = block;
        (*zone).blocklist.user = zone as *mut *mut c_void;
        (*zone).blocklist.tag = PU_STATIC;
        (*zone).rover = block;

        (*block).prev = &mut (*zone).blocklist;
        (*block).next = &mut (*zone).blocklist;
        (*block).user = ptr::null_mut();
        (*block).size = (*zone).size - size_of::<MemZone>() as c_int;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Z_Init() {
    unsafe {
        let mut size: c_int = 0;
        let base = I_ZoneBase(&mut size);
        mainzone = base as *mut MemZone;
        (*mainzone).size = size;

        Z_ClearZone(mainzone);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Z_Free(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }

    unsafe {
        let mut block = (ptr as *mut u8).sub(size_of::<MemBlock>()) as *mut MemBlock;

        if (*block).id != ZONEID {
            doom_error("Z_Free: freed a pointer without ZONEID");
        }

        // Значения <= 0x100 используются как маркеры (например, (void*)2 для unowned), а не реальные указатели
        if ((*block).user as usize) > 0x100 {
            *(*block).user = ptr::null_mut();
        }

        (*block).user = ptr::null_mut();
        (*block).tag = 0;
        (*block).id = 0;

        // Слияние с предыдущим свободным блоком
        let other_prev = (*block).prev;
        if (*other_prev).user.is_null() {
            (*other_prev).size += (*block).size;
            (*other_prev).next = (*block).next;
            (*(*other_prev).next).prev = other_prev;

            if block == (*mainzone).rover {
                (*mainzone).rover = other_prev;
            }
            block = other_prev;
        }

        // Слияние со следующим свободным блоком
        let other_next = (*block).next;
        if (*other_next).user.is_null() {
            (*block).size += (*other_next).size;
            (*block).next = (*other_next).next;
            (*(*block).next).prev = block;

            if other_next == (*mainzone).rover {
                (*mainzone).rover = block;
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Z_Malloc(mut size: c_int, tag: c_int, user: *mut c_void) -> *mut c_void {
    unsafe {
        // Выравнивание по 8 байт (критично для 64-битного Linux!)
        size = (size + 7) & !7;
        size += size_of::<MemBlock>() as c_int;

        let mut base = (*mainzone).rover;

        if (*(*base).prev).user.is_null() {
            base = (*base).prev;
        }

        let mut rover = base;
        let start = (*base).prev;

        loop {
            if rover == start {
                doom_error(&format!("Z_Malloc: failed on allocation of {} bytes", size));
            }

            if !(*rover).user.is_null() {
                if (*rover).tag < PU_PURGELEVEL {
                    rover = (*rover).next;
                    base = rover;
                } else {
                    base = (*base).prev;
                    let data_ptr = (rover as *mut u8).add(size_of::<MemBlock>()) as *mut c_void;
                    Z_Free(data_ptr);
                    base = (*base).next;
                    rover = (*base).next;
                }
            } else {
                rover = (*rover).next;
            }

            if (*base).user.is_null() && (*base).size >= size {
                break;
            }
        }

        let extra = (*base).size - size;
        if extra > MINFRAGMENT {
            let newblock = (base as *mut u8).add(size as usize) as *mut MemBlock;
            (*newblock).size = extra;
            (*newblock).user = ptr::null_mut();
            (*newblock).tag = 0;
            (*newblock).prev = base;
            (*newblock).next = (*base).next;
            (*(*newblock).next).prev = newblock;

            (*base).next = newblock;
            (*base).size = size;
        }

        if !user.is_null() {
            (*base).user = user as *mut *mut c_void;
            let data_ptr = (base as *mut u8).add(size_of::<MemBlock>()) as *mut c_void;
            *(user as *mut *mut c_void) = data_ptr;
        } else {
            if tag >= PU_PURGELEVEL {
                doom_error("Z_Malloc: an owner is required for purgable blocks");
            }
            (*base).user = 2 as *mut *mut c_void;
        }

        (*base).tag = tag;
        (*mainzone).rover = (*base).next;
        (*base).id = ZONEID;

        (base as *mut u8).add(size_of::<MemBlock>()) as *mut c_void
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Z_FreeTags(lowtag: c_int, hightag: c_int) {
    unsafe {
        let mut block = (*mainzone).blocklist.next;
        while block != &mut (*mainzone).blocklist {
            let next = (*block).next;

            if !(*block).user.is_null() && (*block).tag >= lowtag && (*block).tag <= hightag {
                let data_ptr = (block as *mut u8).add(size_of::<MemBlock>()) as *mut c_void;
                Z_Free(data_ptr);
            }

            block = next;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Z_DumpHeap(lowtag: c_int, hightag: c_int) {
    unsafe {
        let zone = mainzone;
        println!("zone size: {}  location: {:p}", (*zone).size, zone);
        println!("tag range: {} to {}", lowtag, hightag);

        let mut block = (*mainzone).blocklist.next;
        loop {
            if (*block).tag >= lowtag && (*block).tag <= hightag {
                println!(
                    "block:{:p}    size:{:7}    user:{:p}    tag:{:3}",
                    block, (*block).size, (*block).user, (*block).tag
                );
            }

            if (*block).next == &mut (*mainzone).blocklist {
                break;
            }

            if (block as *mut u8).add((*block).size as usize) != (*block).next as *mut u8 {
                println!("ERROR: block size does not touch the next block");
            }
            if (*(*block).next).prev != block {
                println!("ERROR: next block doesn't have proper back link");
            }
            if (*block).user.is_null() && (*(*block).next).user.is_null() {
                println!("ERROR: two consecutive free blocks");
            }

            block = (*block).next;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Z_FileDumpHeap(_f: *mut c_void) {
    // Вызываем консольный дамп для совместимости ABI
    unsafe {
        Z_DumpHeap(0, 1000);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Z_CheckHeap() {
    unsafe {
        let mut block = (*mainzone).blocklist.next;
        loop {
            if (*block).next == &mut (*mainzone).blocklist {
                break;
            }

            if (block as *mut u8).add((*block).size as usize) != (*block).next as *mut u8 {
                doom_error("Z_CheckHeap: block size does not touch the next block\n");
            }
            if (*(*block).next).prev != block {
                doom_error("Z_CheckHeap: next block doesn't have proper back link\n");
            }
            if (*block).user.is_null() && (*(*block).next).user.is_null() {
                doom_error("Z_CheckHeap: two consecutive free blocks\n");
            }

            block = (*block).next;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Z_ChangeTag2(ptr: *mut c_void, tag: c_int) {
    unsafe {
        let block = (ptr as *mut u8).sub(size_of::<MemBlock>()) as *mut MemBlock;

        if (*block).id != ZONEID {
            doom_error("Z_ChangeTag: freed a pointer without ZONEID");
        }

        if tag >= PU_PURGELEVEL && ((*block).user as usize) < 0x100 {
            doom_error("Z_ChangeTag: an owner is required for purgable blocks");
        }

        (*block).tag = tag;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Z_FreeMemory() -> c_int {
    unsafe {
        let mut free: c_int = 0;
        let mut block = (*mainzone).blocklist.next;

        while block != &mut (*mainzone).blocklist {
            if (*block).user.is_null() || (*block).tag >= PU_PURGELEVEL {
                free += (*block).size;
            }
            block = (*block).next;
        }
        free
    }
}
