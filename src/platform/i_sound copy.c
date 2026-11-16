#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <SDL3/SDL.h>

#include "z_zone.h"
#include "i_system.h"
#include "i_sound.h"
#include "m_argv.h"
#include "m_misc.h"
#include "w_wad.h"
#include "doomdef.h"

char* sndserver_filename = "sndserver";

#define SAMPLECOUNT     512
#define NUM_CHANNELS    8
#define BUFMUL          4
#define MIXBUFFERSIZE   (SAMPLECOUNT * BUFMUL)
#define SAMPLERATE      11025

static SDL_AudioStream* audio_stream = NULL;

static int            lengths[NUMSFX];
static signed short   mixbuffer[MIXBUFFERSIZE];

static unsigned int   channelstep[NUM_CHANNELS];
static unsigned int   channelstepremainder[NUM_CHANNELS];
static unsigned char* channels[NUM_CHANNELS];
static unsigned char* channelsend[NUM_CHANNELS];

static int            channelstart[NUM_CHANNELS];
static int            channelhandles[NUM_CHANNELS];
static int            channelids[NUM_CHANNELS];

static int steptable[256];
static int            vol_lookup[128 * 256];
static int*           channelleftvol_lookup[NUM_CHANNELS];
static int*           channelrightvol_lookup[NUM_CHANNELS];

static void* getsfx(char* sfxname, int* len)
{
    unsigned char* sfx;
    unsigned char* paddedsfx;
    int            i;
    int            size;
    int            paddedsize;
    char name[20];
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

    for (i = -128; i < 128; i++)
        steptablemid[i] = (int)(pow(2.0, (i / 64.0)) * 65536.0);

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
}

int I_GetSfxLumpNum(sfxinfo_t* sfx)
{
    char namebuf[16];
    snprintf(namebuf, sizeof(namebuf), "ds%s", sfx->name);
    return W_GetNumForName(namebuf);
}

int I_StartSound(int id, int vol, int sep, int pitch, int priority)
{
    fprintf(stderr, "I_StartSound: id=%d vol=%d data=%p\n", id, vol, S_sfx[id].data);

    (void)priority;
    if (!audio_stream || !S_sfx[id].data)
        return 0;

    //return addsfx(id, vol, steptable[pitch], sep);
    return addsfx(id, vol, 1 << 16, sep);
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

// void I_SubmitSound(void)
// {
//     if (!audio_stream)
//         return;

//     // Не даём очереди вырасти больше ~100 мс, чтобы не было задержки звука
//     if (SDL_GetAudioStreamQueued(audio_stream) < SAMPLECOUNT * BUFMUL * 3)
//     {
//         SDL_PutAudioStreamData(audio_stream, mixbuffer, SAMPLECOUNT * BUFMUL);
//     }
// }
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

static void SDLCALL AudioStreamCallback(void* userdata, SDL_AudioStream* stream, int additional_amount, int total_amount)
{
    (void)userdata;
    (void)total_amount;

    while (additional_amount > 0)
    {
        I_UpdateSound();
        SDL_PutAudioStreamData(stream, mixbuffer, SAMPLECOUNT * BUFMUL);
        additional_amount -= SAMPLECOUNT * BUFMUL;
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

    SDL_SetAudioStreamGetCallback(audio_stream, AudioStreamCallback, NULL);
    SDL_ResumeAudioStreamDevice(audio_stream);

    // Проход 1: загружаем все самостоятельные звуки
    for (int i = 1; i < NUMSFX; i++)
    {
        if (!S_sfx[i].link)
            S_sfx[i].data = getsfx(S_sfx[i].name, &lengths[i]);
    }
    // Проход 2: привязываем алиасы (когда все оригиналы уже в памяти)
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
    fprintf(stderr, "I_InitSound: SDL3 audio ready (11025Hz 16-bit stereo)\n");
}

// Music API (следующий шаг — синтезатор MUS/OPL3)
void I_InitMusic(void) {}
void I_ShutdownMusic(void) {}
void I_PlaySong(int handle, int looping) { (void)handle; (void)looping; }
void I_PauseSong(int handle) { (void)handle; }
void I_ResumeSong(int handle) { (void)handle; }
void I_StopSong(int handle) { (void)handle; }
void I_UnRegisterSong(int handle) { (void)handle; }
int I_RegisterSong(void* data) { (void)data; return 1; }
int I_QrySongPlaying(int handle) { (void)handle; return 0; }
