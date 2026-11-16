#ifndef MUS2MID_H
#define MUS2MID_H

#include <stdint.h>
#include <stddef.h>
#include <stdlib.h>
#include <string.h>

#define MUS_MAX_CHANS 16
#define VLQ_MAX_BYTES 8

static inline int mus2mid_convert(const uint8_t* mus_data, uint8_t** out_mid, size_t* out_len)
{
    if (!mus_data || memcmp(mus_data, "MUS\x1a", 4) != 0)
        return 0;

    uint16_t score_len   = (uint16_t)(*(mus_data + 4)) | ((uint16_t)(*(mus_data + 5)) << 8);
    uint16_t score_start = (uint16_t)(*(mus_data + 6)) | ((uint16_t)(*(mus_data + 7)) << 8);

    const uint8_t* src = mus_data + score_start;
    const uint8_t* end = src + score_len;

    size_t cap = score_len * 2 + 128;
    uint8_t* dst = (uint8_t*)malloc(cap);
    if (!dst) return 0;

    size_t pos = 0;

    #define ENSURE_CAP(n) do { \
        if (pos + (n) >= cap) { \
            cap *= 2; \
            uint8_t* tmp = (uint8_t*)realloc(dst, cap); \
            if (!tmp) { free(dst); return 0; } \
            dst = tmp; \
        } \
    } while (0)

    static const uint8_t midi_hdr[] = {
        'M', 'T', 'h', 'd', 0, 0, 0, 6,
        0, 0, 0, 1, 0, 70,
        'M', 'T', 'r', 'k', 0, 0, 0, 0
    };
    memcpy(dst, midi_hdr, sizeof(midi_hdr));
    pos = sizeof(midi_hdr);

    uint8_t tempo_ev[] = { 0x00, 0xFF, 0x51, 0x03, 0x07, 0xA1, 0x20 };
    memcpy(dst + pos, tempo_ev, sizeof(tempo_ev));
    pos += sizeof(tempo_ev);

    uint8_t chan_map[MUS_MAX_CHANS];
    for (int i = 0; i < MUS_MAX_CHANS; i++) chan_map[i] = 0xFF;
    int next_chan = 0;

    uint8_t last_vel[MUS_MAX_CHANS];
    memset(last_vel, 100, sizeof(last_vel));

    static const uint8_t ctrl_map[] = {
        0, 0, 1, 7, 10, 11, 91, 93, 64, 67, 120, 123, 126, 127, 121
    };

    uint32_t delay = 0;

    while (src < end)
    {
        uint8_t ev = *src++;
        uint8_t mus_chan = ev & 0x0F;
        uint8_t ev_type  = (ev >> 4) & 0x07;
        uint8_t last     = (ev >> 7) & 0x01;

        uint8_t midi_chan;
        if (mus_chan == 15)
        {
            midi_chan = 9;
        }
        else
        {
            if (chan_map[mus_chan] == 0xFF)
            {
                if (next_chan == 9) next_chan++;
                chan_map[mus_chan] = (uint8_t)(next_chan++ & 0x0F);
            }
            midi_chan = chan_map[mus_chan];
        }

        if (ev_type == 6)
            break;

        ENSURE_CAP(16);

        uint32_t v = delay;
        uint8_t vlq[VLQ_MAX_BYTES];
        int vlen = 0;
        vlq[vlen++] = v & 0x7F;
        while ((v >>= 7) > 0)
            vlq[vlen++] = (v & 0x7F) | 0x80;
        while (vlen > 0)
            dst[pos++] = vlq[--vlen];
        delay = 0;

        switch (ev_type)
        {
            case 0:
            {
                uint8_t note = (*src++) & 0x7F;
                dst[pos++] = 0x80 | midi_chan;
                dst[pos++] = note;
                dst[pos++] = 0;
                break;
            }
            case 1:
            {
                uint8_t b = *src++;
                uint8_t note = b & 0x7F;
                if (b & 0x80)
                    last_vel[mus_chan] = (*src++) & 0x7F;
                dst[pos++] = 0x90 | midi_chan;
                dst[pos++] = note;
                dst[pos++] = last_vel[mus_chan];
                break;
            }
            case 2:
            {
                uint8_t wheel = *src++;
                uint16_t bend = (uint16_t)wheel << 6;
                dst[pos++] = 0xE0 | midi_chan;
                dst[pos++] = bend & 0x7F;
                dst[pos++] = (bend >> 7) & 0x7F;
                break;
            }
            case 3:
            {
                uint8_t sys = (*src++) & 0x7F;
                if (sys >= 10 && sys <= 14)
                {
                    dst[pos++] = 0xB0 | midi_chan;
                    dst[pos++] = ctrl_map[sys];
                    dst[pos++] = 0;
                }
                break;
            }
            case 4:
            {
                uint8_t ctrl = (*src++) & 0x7F;
                uint8_t val  = (*src++) & 0x7F;
                if (ctrl == 0)
                {
                    dst[pos++] = 0xC0 | midi_chan;
                    dst[pos++] = val;
                }
                else if (ctrl < 15)
                {
                    dst[pos++] = 0xB0 | midi_chan;
                    dst[pos++] = ctrl_map[ctrl];
                    dst[pos++] = val;
                }
                break;
            }
        }

        if (last)
        {
            uint8_t b;
            do {
                b = *src++;
                delay = (delay << 7) | (b & 0x7F);
            } while (b & 0x80);
        }
    }

    ENSURE_CAP(8);
    dst[pos++] = 0x00;
    dst[pos++] = 0xFF;
    dst[pos++] = 0x2F;
    dst[pos++] = 0x00;

    uint32_t track_len = (uint32_t)(pos - 22);
    *(dst + 18) = (track_len >> 24) & 0xFF;
    *(dst + 19) = (track_len >> 16) & 0xFF;
    *(dst + 20) = (track_len >> 8)  & 0xFF;
    *(dst + 21) = track_len & 0xFF;

    *out_mid = dst;
    *out_len = pos;
    return 1;
}

#endif
