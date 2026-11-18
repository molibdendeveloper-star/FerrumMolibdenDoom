use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use crate::bindings::{GameMission_t, GameMode_t, Language_t, doom, english, indetermined};

static GAMEMODE: AtomicU32 = AtomicU32::new(indetermined);
static GAMEMISSION: AtomicU32 = AtomicU32::new(doom);
static LANGUAGE: AtomicU32 = AtomicU32::new(english);
static MODIFIED_GAME: AtomicBool = AtomicBool::new(false);

#[inline]
pub fn get_gamemode() -> GameMode_t {
    GAMEMODE.load(Ordering::Relaxed)
}

#[inline]
pub fn set_gamemode(mode: GameMode_t) {
    GAMEMODE.store(mode, Ordering::Relaxed);
}

#[inline]
pub fn get_gamemission() -> GameMission_t {
    GAMEMISSION.load(Ordering::Relaxed)
}

#[inline]
pub fn set_gamemission(mission: GameMission_t) {
    GAMEMISSION.store(mission, Ordering::Relaxed);
}

#[inline]
pub fn get_language() -> Language_t {
    LANGUAGE.load(Ordering::Relaxed)
}

#[inline]
pub fn set_language(lang: Language_t) {
    LANGUAGE.store(lang, Ordering::Relaxed);
}

#[inline]
pub fn is_modified_game() -> bool {
    MODIFIED_GAME.load(Ordering::Relaxed)
}

#[inline]
pub fn set_modified_game(val: bool) {
    MODIFIED_GAME.store(val, Ordering::Relaxed);
}
