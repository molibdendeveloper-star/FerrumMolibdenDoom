//! Чистый Rust-конвертер формата DOOM MUS в Standard MIDI File (SMF Type 0).

const MUS_MAGIC: &[u8; 4] = b"MUS\x1a";

// Таблица маппинга контроллеров MUS -> MIDI CC
const MUS_TO_MIDI_CC: [u8; 15] = [
    0,  // 0: Program Change (обрабатывается отдельно)
    0,  // 1: Bank Select
    1,  // 2: Modulation wheel
    7,  // 3: Volume
    10, // 4: Pan
    11, // 5: Expression
    91, // 6: Reverb depth
    93, // 7: Chorus depth
    64, // 8: Sustain pedal
    67, // 9: Soft pedal
    120,// 10: All sounds off
    123,// 11: All notes off
    126,// 12: Mono
    127,// 13: Poly
    121,// 14: Reset all controllers
];

fn write_vlq(out: &mut Vec<u8>, mut val: u32) {
    let mut buf = [0u8; 5];
    let mut len = 0usize;
    buf[len] = (val & 0x7F) as u8;
    len += 1;
    val >>= 7;
    while val > 0 {
        buf[len] = ((val & 0x7F) as u8) | 0x80;
        len += 1;
        val >>= 7;
    }
    while len > 0 {
        len -= 1;
        out.push(buf[len]);
    }
}

pub fn convert_mus_to_mid(mus: &[u8]) -> Option<Vec<u8>> {
    if mus.len() < 16 || &mus[0..4] != MUS_MAGIC {
        return None;
    }

    let score_len = u16::from_le_bytes([mus[4], mus[5]]) as usize;
    let score_start = u16::from_le_bytes([mus[6], mus[7]]) as usize;

    if score_start >= mus.len() || score_start.saturating_add(score_len) > mus.len() {
        return None;
    }

    let score = &mus[score_start..(score_start + score_len)];
    let mut track = Vec::with_capacity(score_len * 2 + 64);

    // Tempo: 140 тиков/сек в DOOM -> 1 000 000 мкс/четверть при division = 140
    track.extend_from_slice(&[0x00, 0xFF, 0x51, 0x03, 0x0F, 0x42, 0x40]);

    let mut channel_vol = [100u8; 16];
    let mut pos = 0usize;
    let mut delta_ticks: u32 = 0;

    let map_channel = |mus_ch: u8| -> u8 {
        if mus_ch == 15 {
            9 // Канал ударных в GM MIDI
        } else if mus_ch >= 9 {
            mus_ch + 1
        } else {
            mus_ch
        }
    };

    while pos < score.len() {
        let desc = score[pos];
        pos += 1;

        let last = (desc & 0x80) != 0;
        let event_type = (desc >> 4) & 0x07;
        let mus_ch = desc & 0x0F;
        let midi_ch = map_channel(mus_ch);

        match event_type {
            0 => {
                // Release note
                if pos >= score.len() {
                    break;
                }
                let note = score[pos] & 0x7F;
                pos += 1;
                write_vlq(&mut track, delta_ticks);
                delta_ticks = 0;
                track.extend_from_slice(&[0x80 | midi_ch, note, 0]);
            }
            1 => {
                // Play note
                if pos >= score.len() {
                    break;
                }
                let raw = score[pos];
                pos += 1;
                let note = raw & 0x7F;
                if (raw & 0x80) != 0 {
                    if pos >= score.len() {
                        break;
                    }
                    channel_vol[mus_ch as usize] = score[pos] & 0x7F;
                    pos += 1;
                }
                let vel = channel_vol[mus_ch as usize];
                write_vlq(&mut track, delta_ticks);
                delta_ticks = 0;
                track.extend_from_slice(&[0x90 | midi_ch, note, vel]);
            }
            2 => {
                // Pitch bend
                if pos >= score.len() {
                    break;
                }
                let bend = score[pos] as u16;
                pos += 1;
                let midi_bend = bend << 6;
                let lsb = (midi_bend & 0x7F) as u8;
                let msb = ((midi_bend >> 7) & 0x7F) as u8;
                write_vlq(&mut track, delta_ticks);
                delta_ticks = 0;
                track.extend_from_slice(&[0xE0 | midi_ch, lsb, msb]);
            }
            3 => {
                // System event
                if pos >= score.len() {
                    break;
                }
                let sys = score[pos] & 0x7F;
                pos += 1;
                if (sys as usize) < MUS_TO_MIDI_CC.len() && sys >= 10 {
                    write_vlq(&mut track, delta_ticks);
                    delta_ticks = 0;
                    track.extend_from_slice(&[0xB0 | midi_ch, MUS_TO_MIDI_CC[sys as usize], 0]);
                }
            }
            4 => {
                // Controller change / Program change
                if pos + 1 >= score.len() {
                    break;
                }
                let ctrl = score[pos] & 0x7F;
                let val = score[pos + 1] & 0x7F;
                pos += 2;
                if ctrl == 0 {
                    write_vlq(&mut track, delta_ticks);
                    delta_ticks = 0;
                    track.extend_from_slice(&[0xC0 | midi_ch, val]);
                } else if (ctrl as usize) < MUS_TO_MIDI_CC.len() {
                    write_vlq(&mut track, delta_ticks);
                    delta_ticks = 0;
                    track.extend_from_slice(&[0xB0 | midi_ch, MUS_TO_MIDI_CC[ctrl as usize], val]);
                }
            }
            5 | 7 => {
                // End of measure / score
            }
            6 => {
                // End of track
                break;
            }
            _ => break,
        }

        if last {
            let mut ticks: u32 = 0;
            while pos < score.len() {
                let b = score[pos];
                pos += 1;
                ticks = (ticks << 7) | ((b & 0x7F) as u32);
                if (b & 0x80) == 0 {
                    break;
                }
            }
            delta_ticks = delta_ticks.saturating_add(ticks);
        }
    }

    // End of Track meta-event
    write_vlq(&mut track, delta_ticks);
    track.extend_from_slice(&[0xFF, 0x2F, 0x00]);

    // Заголовок SMF Type 0: 1 трек, 140 тиков на четверть
    let mut mid = Vec::with_capacity(14 + 8 + track.len());
    mid.extend_from_slice(b"MThd");
    mid.extend_from_slice(&6u32.to_be_bytes());
    mid.extend_from_slice(&0u16.to_be_bytes());   // Format 0
    mid.extend_from_slice(&1u16.to_be_bytes());   // 1 track
    mid.extend_from_slice(&140u16.to_be_bytes()); // 140 ticks/quarter

    mid.extend_from_slice(b"MTrk");
    mid.extend_from_slice(&(track.len() as u32).to_be_bytes());
    mid.extend_from_slice(&track);

    Some(mid)
}
