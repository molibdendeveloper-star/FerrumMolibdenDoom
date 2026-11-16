#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <SDL3/SDL.h>

#define TSF_IMPLEMENTATION
#include "tsf.h"

#define TML_IMPLEMENTATION
#include "tml.h"

#include "mus2mid.h"

#include "z_zone.h"
#include "i_system.h"
#include "i_sound.h"
#include "m_argv.h"
#include "m_misc.h"
#include "w_wad.h"
#include "doomdef.h"

char* sndserver_filename = "sndserver";

#define SAMPLECOUNT     1024
#define NUM_CHANNELS    8
#define BUFMUL          4
#define MIXBUFFERSIZE   (SAMPLECOUNT * BUFMUL)
#define SAMPLERATE      44100
#define DOOM_SFX_RATE   11025

static SDL_AudioStream* audio_stream = NULL;

// --- SFX state ---
static int            lengths[NUMSFX];
static signed short   mixbuffer[MIXBUFFERSIZE];

static unsigned int   channelstep[NUM_CHANNELS];
static unsigned int   channelstepremainder[NUM_CHANNELS];
static unsigned char* channels[NUM_CHANNELS];
static unsigned char* channelsend[NUM_CHANNELS];

static int            channelstart[NUM_CHANNELS];
static int            channelhandles[NUM_CHANNELS];
static int            channelids[NUM_CHANNELS];

static int            steptable[256];
static int            vol_lookup[128 * 256];
static int*           channelleftvol_lookup[NUM_CHANNELS];
static int*           channelrightvol_lookup[NUM_CHANNELS];

// --- MIDI / Music state ---
static tsf*           g_tsf = NULL;
static tml_message*   g_tml_head = NULL;
static tml_message*   g_tml_curr = NULL;
static double         g_msec = 0.0;
static int            g_music_playing = 0;
static int            g_music_looping = 0;
static float          g_music_volume = 0.5f;
static signed short   musicbuffer[MIXBUFFERSIZE];

static void* getsfx(char* sfxname, int* len)
{
    unsigned char* sfx;
    unsigned char* paddedsfx;
    int            i;
    int            size;
    int            paddedsize;
    char           name[20];
    int            sfxlump;

    snprintf(name, sizeof(name), "ds%s", sfxname);

    if (W_CheckNumForName(name) == -1)
        sfxlump = W_GetNumForName("dspistol");
    else
        sfxlump = W_GetNumForName(name);

    size = W_LumpLength(sfxlump);
    sfx = (unsigned char*)W_CacheLumpNum(sfxlump, PU_STATIC);

    paddedsize = ((size - 8 + (SAMPLECOUNT - 1)) / SAMPLECOUNT) * SAMPLECOUNT;
    paddedsfx = (unsigned char*)Z_Malloc(paddedsize + 8, PU_STATIC, 0);

    memcpy(paddedsfx, sfx, size);
    for (i = size; i < paddedsize + 8; i++)
        paddedsfx[i] = 128;

    Z_Free(sfx);
    *len = paddedsize;

    return (void*)(paddedsfx + 8);
}

static int addsfx(int sfxid, int volume, int step, int seperation)
{
    static unsigned short handlenums = 0;
    int i;
    int rc = -1;
    int oldest = gametic;
    int oldestnum = 0;
    int slot;
    int rightvol;
    int leftvol;

    if (sfxid == sfx_sawup  || sfxid == sfx_sawidl ||
        sfxid == sfx_sawful || sfxid == sfx_sawhit ||
        sfxid == sfx_stnmov || sfxid == sfx_pistol)
    {
        for (i = 0; i < NUM_CHANNELS; i++)
        {
            if (channels[i] && channelids[i] == sfxid)
            {
                channels[i] = 0;
                break;
            }
        }
    }

    for (i = 0; (i < NUM_CHANNELS) && channels[i]; i++)
    {
        if (channelstart[i] < oldest)
        {
            oldestnum = i;
            oldest = channelstart[i];
        }
    }

    slot = (i == NUM_CHANNELS) ? oldestnum : i;

    channels[slot] = (unsigned char*)S_sfx[sfxid].data;
    channelsend[slot] = channels[slot] + lengths[sfxid];

    if (!handlenums)
        handlenums = 100;

    channelhandles[slot] = rc = handlenums++;
    channelstep[slot] = step;
    channelstepremainder[slot] = 0;
    channelstart[slot] = gametic;

    seperation += 1;
    leftvol = volume - ((volume * seperation * seperation) >> 16);
    seperation = seperation - 257;
    rightvol = volume - ((volume * seperation * seperation) >> 16);

    if (rightvol < 0) rightvol = 0;
    if (rightvol > 127) rightvol = 127;
    if (leftvol < 0) leftvol = 0;
    if (leftvol > 127) leftvol = 127;

    channelleftvol_lookup[slot] = &vol_lookup[leftvol * 256];
    channelrightvol_lookup[slot] = &vol_lookup[rightvol * 256];
    channelids[slot] = sfxid;

    return rc;
}

void I_SetChannels(void)
{
    int i, j;
    int* steptablemid = steptable + 128;
    double rate_ratio = (double)DOOM_SFX_RATE / (double)SAMPLERATE;

    for (i = -128; i < 128; i++)
        steptablemid[i] = (int)(pow(2.0, (i / 64.0)) * rate_ratio * 65536.0);

    for (i = 0; i < 128; i++)
        for (j = 0; j < 256; j++)
            vol_lookup[i * 256 + j] = (i * (j - 128) * 256) / 127;
}

void I_SetSfxVolume(int volume)
{
    snd_SfxVolume = volume;
}

void I_SetMusicVolume(int volume)
{
    snd_MusicVolume = volume;
    g_music_volume = (float)volume / 15.0f;
    if (g_tsf)
        tsf_set_volume(g_tsf, g_music_volume);
}

int I_GetSfxLumpNum(sfxinfo_t* sfx)
{
    char namebuf[16];
    snprintf(namebuf, sizeof(namebuf), "ds%s", sfx->name);
    return W_GetNumForName(namebuf);
}

int I_StartSound(int id, int vol, int sep, int pitch, int priority)
{
    (void)priority;
    if (!audio_stream || !S_sfx[id].data)
        return 0;

    return addsfx(id, vol, steptable[pitch], sep);
}

void I_StopSound(int handle)
{
    for (int i = 0; i < NUM_CHANNELS; i++)
    {
        if (channelhandles[i] == handle)
        {
            channels[i] = 0;
            break;
        }
    }
}

int I_SoundIsPlaying(int handle)
{
    for (int i = 0; i < NUM_CHANNELS; i++)
    {
        if (channels[i] && channelhandles[i] == handle)
            return 1;
    }
    return 0;
}

void I_UpdateSoundParams(int handle, int vol, int sep, int pitch)
{
    (void)handle; (void)vol; (void)sep; (void)pitch;
}

void I_UpdateSound(void)
{
    unsigned int sample;
    int dl, dr;
    signed short* leftout = mixbuffer;
    signed short* rightout = mixbuffer + 1;
    signed short* leftend = mixbuffer + SAMPLECOUNT * 2;
    int chan;

    while (leftout != leftend)
    {
        dl = 0;
        dr = 0;

        for (chan = 0; chan < NUM_CHANNELS; chan++)
        {
            if (channels[chan])
            {
                sample = *channels[chan];
                dl += channelleftvol_lookup[chan][sample];
                dr += channelrightvol_lookup[chan][sample];

                channelstepremainder[chan] += channelstep[chan];
                channels[chan] += channelstepremainder[chan] >> 16;
                channelstepremainder[chan] &= 65536 - 1;

                if (channels[chan] >= channelsend[chan])
                    channels[chan] = 0;
            }
        }

        if (dl > 0x7fff) *leftout = 0x7fff;
        else if (dl < -0x8000) *leftout = -0x8000;
        else *leftout = dl;

        if (dr > 0x7fff) *rightout = 0x7fff;
        else if (dr < -0x8000) *rightout = -0x8000;
        else *rightout = dr;

        leftout += 2;
        rightout += 2;
    }
}

static void RenderMusicBlock(signed short* dst, int sample_count)
{
    if (!g_tsf || !g_music_playing || !g_tml_curr)
    {
        memset(dst, 0, sample_count * 2 * sizeof(signed short));
        return;
    }

    int samples_rendered = 0;
    while (samples_rendered < sample_count)
    {
        while (g_tml_curr && g_msec >= g_tml_curr->time)
        {
            switch (g_tml_curr->type)
            {
                case TML_PROGRAM_CHANGE:
                    tsf_channel_set_presetnumber(g_tsf, g_tml_curr->channel, g_tml_curr->program, (g_tml_curr->channel == 9));
                    break;
                case TML_NOTE_ON:
                    tsf_channel_note_on(g_tsf, g_tml_curr->channel, g_tml_curr->key, g_tml_curr->velocity / 127.0f);
                    break;
                case TML_NOTE_OFF:
                    tsf_channel_note_off(g_tsf, g_tml_curr->channel, g_tml_curr->key);
                    break;
                case TML_PITCH_BEND:
                    tsf_channel_set_pitchwheel(g_tsf, g_tml_curr->channel, g_tml_curr->pitch_bend);
                    break;
                case TML_CONTROL_CHANGE:
                    tsf_channel_midi_control(g_tsf, g_tml_curr->channel, g_tml_curr->control, g_tml_curr->control_value);
                    break;
            }
            g_tml_curr = g_tml_curr->next;
        }

        if (!g_tml_curr)
        {
            if (g_music_looping && g_tml_head)
            {
                g_tml_curr = g_tml_head;
                g_msec = 0.0;
            }
            else
            {
                g_music_playing = 0;
                memset(dst + samples_rendered * 2, 0, (sample_count - samples_rendered) * 2 * sizeof(signed short));
                break;
            }
        }

        int chunk = 64;
        if (samples_rendered + chunk > sample_count)
            chunk = sample_count - samples_rendered;

        tsf_render_short(g_tsf, dst + samples_rendered * 2, chunk, 0);
        g_msec += (chunk * 1000.0) / (double)SAMPLERATE;
        samples_rendered += chunk;
    }
}

static void SDLCALL AudioStreamCallback(void* userdata, SDL_AudioStream* stream, int additional_amount, int total_amount)
{
    (void)userdata;
    (void)total_amount;

    while (additional_amount > 0)
    {
        I_UpdateSound();
        RenderMusicBlock(musicbuffer, SAMPLECOUNT);

        for (int i = 0; i < SAMPLECOUNT * 2; i++)
        {
            int mixed = (int)mixbuffer[i] + (int)musicbuffer[i];
            if (mixed > 0x7fff) mixed = 0x7fff;
            else if (mixed < -0x8000) mixed = -0x8000;
            mixbuffer[i] = (signed short)mixed;
        }

        SDL_PutAudioStreamData(stream, mixbuffer, SAMPLECOUNT * BUFMUL);
        additional_amount -= SAMPLECOUNT * BUFMUL;
    }
}

void I_SubmitSound(void)
{
}

void I_ShutdownSound(void)
{
    if (audio_stream)
    {
        SDL_DestroyAudioStream(audio_stream);
        audio_stream = NULL;
    }
}

void I_InitSound(void)
{
    I_SetChannels();

    if (!SDL_InitSubSystem(SDL_INIT_AUDIO))
    {
        fprintf(stderr, "I_InitSound: SDL_InitSubSystem(AUDIO) failed: %s\n", SDL_GetError());
        return;
    }
    SDL_AudioSpec spec;
    spec.format = SDL_AUDIO_S16;
    spec.channels = 2;
    spec.freq = SAMPLERATE;
    audio_stream = SDL_OpenAudioDeviceStream(SDL_AUDIO_DEVICE_DEFAULT_PLAYBACK, &spec, NULL, NULL);
    if (!audio_stream)
    {
        fprintf(stderr, "I_InitSound: SDL_OpenAudioDeviceStream failed: %s\n", SDL_GetError());
        return;
    }
    for (int i = 1; i < NUMSFX; i++)
    {
        if (!S_sfx[i].link)
        S_sfx[i].data = getsfx(S_sfx[i].name, &lengths[i]);
    }
    for (int i = 1; i < NUMSFX; i++)
    {
        if (S_sfx[i].link)
        {
            int link_idx = (int)(S_sfx[i].link - S_sfx);
            S_sfx[i].data = S_sfx[link_idx].data;
            lengths[i] = lengths[link_idx];
        }
    }
    memset(mixbuffer, 0, sizeof(mixbuffer));
    memset(musicbuffer, 0, sizeof(musicbuffer));
    SDL_SetAudioStreamGetCallback(audio_stream, AudioStreamCallback, NULL);
    SDL_ResumeAudioStreamDevice(audio_stream);
    fprintf(stderr, "I_InitSound: SDL3 audio ready (44100Hz 16-bit stereo)\n");
}

// --- MUSIC API ---
void I_InitMusic(void)
{
    const char* sf_paths[] = {
    "soundfont.sf2",
    "/usr/share/soundfonts/FluidR3_GM.sf2",
    "/usr/share/sounds/sf2/FluidR3_GM.sf2",
    "/usr/share/soundfonts/default.sf2",
    NULL
    };
    for (int i = 0; sf_paths[i]; i++)
    {
        g_tsf = tsf_load_filename(sf_paths[i]);
        if (g_tsf)
        {
            tsf_channel_set_bank_preset(g_tsf, 9, 128, 0);
            tsf_set_output(g_tsf, TSF_STEREO_INTERLEAVED, SAMPLERATE, 0.0f);
            tsf_set_volume(g_tsf, g_music_volume);
            fprintf(stderr, "I_InitMusic: loaded SoundFont '%s'\n", sf_paths[i]);
            return;
        }
    }
    fprintf(stderr, "I_InitMusic: no SoundFont (.sf2) found, music disabled\n");
}

void I_ShutdownMusic(void)
{
    g_music_playing = 0;
    if (g_tml_head)
    {
        tml_free(g_tml_head);
        g_tml_head = g_tml_curr = NULL;
    }
    if (g_tsf)
    {
        tsf_close(g_tsf);
        g_tsf = NULL;
    }
}

void I_PlaySong(int handle, int looping)
{
    (void)handle;
    if (!g_tml_head) return;
    g_tml_curr = g_tml_head;
    g_msec = 0.0;
    g_music_looping = looping;
    g_music_playing = 1;
}

void I_PauseSong(int handle)
{
    (void)handle;
    g_music_playing = 0;
}

void I_ResumeSong(int handle)
{
    (void)handle;
    if (g_tml_head)
    g_music_playing = 1;
}

void I_StopSong(int handle)
{
    (void)handle;
    g_music_playing = 0;
    if (g_tsf)
    tsf_note_off_all(g_tsf);
}

void I_UnRegisterSong(int handle)
{
    (void)handle;
    g_music_playing = 0;
    if (g_tsf)
        tsf_note_off_all(g_tsf);
    if (g_tml_head)
    {
        tml_free(g_tml_head);
        g_tml_head = g_tml_curr = NULL;
    }
}

int I_RegisterSong(void* data)
{
    I_UnRegisterSong(1);
    if (!data) return 0;
    uint8_t* midi_bytes = NULL;
    size_t midi_len = 0;
    if (!mus2mid_convert((const uint8_t*)data, &midi_bytes, &midi_len))
    {
        fprintf(stderr, "I_RegisterSong: failed to convert MUS to MIDI\n");
        return 0;
    }
    g_tml_head = tml_load_memory(midi_bytes, (int)midi_len);
    free(midi_bytes);
    if (!g_tml_head)
    {
        fprintf(stderr, "I_RegisterSong: tml_load_memory failed\n");
        return 0;
    }
    g_tml_curr = g_tml_head;
    g_msec = 0.0;
    return 1;
}

int I_QrySongPlaying(int handle)
{
    (void)handle;
    return g_music_playing;
}