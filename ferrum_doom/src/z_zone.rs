#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code)]

use std::collections::HashMap;
use std::ffi::{c_int, c_void};
use std::ptr;
use std::sync::{Mutex, OnceLock};

use crate::v_video::doom_error;

#[repr(C)]
pub struct MemBlock {
    pub size: c_int,
    pub user: *mut *mut c_void,
    pub tag: c_int,
    pub id: c_int,
    pub next: *mut MemBlock,
    pub prev: *mut MemBlock,
}

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

const DEFAULT_ZONE_BYTES: usize = 32 * 1024 * 1024; // 32 МБ виртуального бюджета

struct ZoneBlock {
    _storage: Box<[u64]>,
    size: usize,
    tag: c_int,
    user: usize, // Адрес *mut *mut c_void или 2 (unowned)
}

struct ZoneAllocator {
    blocks: HashMap<usize, ZoneBlock>,
    used_bytes: usize,
    total_budget: usize,
}

impl ZoneAllocator {
    fn new(budget: usize) -> Self {
        Self {
            blocks: HashMap::with_capacity(4096),
            used_bytes: 0,
            total_budget: budget,
        }
    }

    fn alloc(&mut self, raw_size: c_int, tag: c_int, user_addr: usize) -> *mut c_void {
        if raw_size < 0 {
            doom_error(&format!("Z_Malloc: negative allocation {}", raw_size));
        }
        if user_addr == 0 && tag >= PU_PURGELEVEL {
            doom_error("Z_Malloc: an owner is required for purgable blocks");
        }

        let aligned_bytes = ((raw_size as usize) + 7) & !7;
        let words = (aligned_bytes / 8).max(1);

        // Если приближаемся к бюджету — вытесняем кэшируемые блоки (tag >= PU_PURGELEVEL)
        if self.used_bytes + aligned_bytes > self.total_budget {
            self.purge_cache();
        }

        let mut storage = vec![0u64; words].into_boxed_slice();
        let ptr = storage.as_mut_ptr() as *mut c_void;
        let addr = ptr as usize;

        let owner = if user_addr != 0 {
            unsafe {
                *(user_addr as *mut *mut c_void) = ptr;
            }
            user_addr
        } else {
            2
        };

        self.used_bytes += aligned_bytes;
        self.blocks.insert(
            addr,
            ZoneBlock {
                _storage: storage,
                size: aligned_bytes,
                tag,
                user: owner,
            },
        );

        ptr
    }

    fn free(&mut self, ptr: *mut c_void) {
        if ptr.is_null() {
            return;
        }
        let addr = ptr as usize;
        let Some(block) = self.blocks.remove(&addr) else {
            doom_error("Z_Free: freed a pointer without ZONEID");
        };

        if block.user > 0x100 {
            unsafe {
                *(block.user as *mut *mut c_void) = ptr::null_mut();
            }
        }
        self.used_bytes = self.used_bytes.saturating_sub(block.size);
    }

    fn free_tags(&mut self, lowtag: c_int, hightag: c_int) {
        let to_free: Vec<usize> = self
            .blocks
            .iter()
            .filter(|(_, b)| b.tag >= lowtag && b.tag <= hightag)
            .map(|(&addr, _)| addr)
            .collect();

        for addr in to_free {
            self.free(addr as *mut c_void);
        }
    }

    fn purge_cache(&mut self) {
        self.free_tags(PU_PURGELEVEL, i32::MAX);
    }

    fn change_tag(&mut self, ptr: *mut c_void, tag: c_int) {
        if ptr.is_null() {
            return;
        }
        let addr = ptr as usize;
        let Some(block) = self.blocks.get_mut(&addr) else {
            doom_error("Z_ChangeTag: freed a pointer without ZONEID");
        };
        if tag >= PU_PURGELEVEL && block.user < 0x100 {
            doom_error("Z_ChangeTag: an owner is required for purgable blocks");
        }
        block.tag = tag;
    }

    fn free_memory(&self) -> c_int {
        let purgable: usize = self
            .blocks
            .values()
            .filter(|b| b.tag >= PU_PURGELEVEL)
            .map(|b| b.size)
            .sum();
        let avail = self.total_budget.saturating_sub(self.used_bytes) + purgable;
        avail.min(c_int::MAX as usize) as c_int
    }
}

static ZONE: OnceLock<Mutex<ZoneAllocator>> = OnceLock::new();

fn zone() -> &'static Mutex<ZoneAllocator> {
    ZONE.get_or_init(|| Mutex::new(ZoneAllocator::new(DEFAULT_ZONE_BYTES)))
}

pub fn get_block_tag(ptr: *mut c_void) -> Option<c_int> {
    if ptr.is_null() {
        return None;
    }
    let addr = ptr as usize;
    zone().lock().ok().and_then(|z| z.blocks.get(&addr).map(|b| b.tag))
}

#[unsafe(no_mangle)]
pub extern "C" fn Z_Init() {
    let _ = zone();
}

#[unsafe(no_mangle)]
pub extern "C" fn Z_Free(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    zone().lock().unwrap().free(ptr);
}

#[unsafe(no_mangle)]
pub extern "C" fn Z_Malloc(size: c_int, tag: c_int, user: *mut c_void) -> *mut c_void {
    zone().lock().unwrap().alloc(size, tag, user as usize)
}

#[unsafe(no_mangle)]
pub extern "C" fn Z_FreeTags(lowtag: c_int, hightag: c_int) {
    zone().lock().unwrap().free_tags(lowtag, hightag);
}

#[unsafe(no_mangle)]
pub extern "C" fn Z_DumpHeap(lowtag: c_int, hightag: c_int) {
    let z = zone().lock().unwrap();
    println!("zone budget: {}  used: {}", z.total_budget, z.used_bytes);
    println!("tag range: {} to {}", lowtag, hightag);
    for (&addr, b) in &z.blocks {
        if b.tag >= lowtag && b.tag <= hightag {
            println!(
                "block:{:#x}    size:{:7}    user:{:#x}    tag:{:3}",
                addr, b.size, b.user, b.tag
            );
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn Z_FileDumpHeap(_f: *mut c_void) {
    Z_DumpHeap(0, 1000);
}

#[unsafe(no_mangle)]
pub extern "C" fn Z_CheckHeap() {}

#[unsafe(no_mangle)]
pub extern "C" fn Z_ChangeTag2(ptr: *mut c_void, tag: c_int) {
    zone().lock().unwrap().change_tag(ptr, tag);
}

#[unsafe(no_mangle)]
pub extern "C" fn Z_FreeMemory() -> c_int {
    zone().lock().unwrap().free_memory()
}
