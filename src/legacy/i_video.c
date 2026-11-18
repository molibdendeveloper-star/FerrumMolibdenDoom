#include <stdlib.h>
#include <stdint.h>
#include <SDL3/SDL.h>

#include "doomdef.h"
#include "doomstat.h"
#include "i_system.h"
#include "v_video.h"
#include "m_argv.h"
#include "d_main.h"
#include "i_video.h"

static SDL_Window*   window   = NULL;
static SDL_Renderer* renderer = NULL;
static SDL_Texture*  texture  = NULL;

static uint32_t palette_argb[256];
static uint32_t frame_buffer[SCREENWIDTH * SCREENHEIGHT];

static int mouse_grabbed = 1;

static void SetMouseGrab(int grab)
{
    mouse_grabbed = grab;
    if (window)
        SDL_SetWindowRelativeMouseMode(window, grab ? true : false);
}

static int TranslateKey(SDL_Keycode key)
{
    switch (key)
    {
        case SDLK_LEFT:      return KEY_LEFTARROW;
        case SDLK_RIGHT:     return KEY_RIGHTARROW;
        case SDLK_DOWN:      return KEY_DOWNARROW;
        case SDLK_UP:        return KEY_UPARROW;
        case SDLK_ESCAPE:    return KEY_ESCAPE;
        case SDLK_RETURN:
        case SDLK_KP_ENTER:  return KEY_ENTER;
        case SDLK_TAB:       return KEY_TAB;
        case SDLK_F1:        return KEY_F1;
        case SDLK_F2:        return KEY_F2;
        case SDLK_F3:        return KEY_F3;
        case SDLK_F4:        return KEY_F4;
        case SDLK_F5:        return KEY_F5;
        case SDLK_F6:        return KEY_F6;
        case SDLK_F7:        return KEY_F7;
        case SDLK_F8:        return KEY_F8;
        case SDLK_F9:        return KEY_F9;
        case SDLK_F10:       return KEY_F10;
        case SDLK_F11:       return KEY_F11;
        case SDLK_F12:       return KEY_F12;
        case SDLK_BACKSPACE:
        case SDLK_DELETE:    return KEY_BACKSPACE;
        case SDLK_PAUSE:     return KEY_PAUSE;
        case SDLK_EQUALS:    return KEY_EQUALS;
        case SDLK_MINUS:     return KEY_MINUS;
        case SDLK_LSHIFT:
        case SDLK_RSHIFT:    return KEY_RSHIFT;
        case SDLK_LCTRL:
        case SDLK_RCTRL:     return KEY_RCTRL;
        case SDLK_LALT:
        case SDLK_RALT:      return KEY_RALT;
        default:
            if (key >= 'A' && key <= 'Z')
                return (int)(key + ('a' - 'A'));
            if (key >= 32 && key <= 126)
                return (int)key;
            return 0;
    }
}


void I_InitGraphics(void)
{
    static int firsttime = 1;
    if (!firsttime)
        return;
    firsttime = 0;

    if (!SDL_Init(SDL_INIT_VIDEO | SDL_INIT_EVENTS))
        I_Error("SDL_Init failed: %s", SDL_GetError());

    // Окно 960x600 (3x масштаб от оригинала 320x200)
    window = SDL_CreateWindow("MolibdenDOOM", SCREENWIDTH * 3, SCREENHEIGHT * 3, SDL_WINDOW_RESIZABLE);
    if (!window)
        I_Error("SDL_CreateWindow failed: %s", SDL_GetError());

    renderer = SDL_CreateRenderer(window, NULL);
    if (!renderer)
        I_Error("SDL_CreateRenderer failed: %s", SDL_GetError());

    // Чёткий пиксель-арт без мыла при растягивании окна
    SDL_SetRenderLogicalPresentation(renderer, SCREENWIDTH, SCREENHEIGHT, SDL_LOGICAL_PRESENTATION_LETTERBOX);

    texture = SDL_CreateTexture(
        renderer,
        SDL_PIXELFORMAT_ARGB8888,
        SDL_TEXTUREACCESS_STREAMING,
        SCREENWIDTH,
        SCREENHEIGHT
    );
    if (!texture)
        I_Error("SDL_CreateTexture failed: %s", SDL_GetError());

    SDL_SetTextureScaleMode(texture, SDL_SCALEMODE_NEAREST);

    // Буфер screens[0], в который движок рендерит кадр
    screens[0] = (byte*)malloc(SCREENWIDTH * SCREENHEIGHT);
    if (!screens[0])
        I_Error("Failed to allocate screens[0]");
    SetMouseGrab(1);
}

void I_ShutdownGraphics(void)
{
    if (texture)  { SDL_DestroyTexture(texture);   texture = NULL; }
    if (renderer) { SDL_DestroyRenderer(renderer); renderer = NULL; }
    if (window)   { SDL_DestroyWindow(window);     window = NULL; }
    SDL_Quit();
}

void I_SetPalette(byte* palette)
{
    for (int i = 0; i < 256; i++)
    {
        uint8_t r = gammatable[usegamma][*palette++];
        uint8_t g = gammatable[usegamma][*palette++];
        uint8_t b = gammatable[usegamma][*palette++];
        palette_argb[i] = 0xFF000000u | ((uint32_t)r << 16) | ((uint32_t)g << 8) | (uint32_t)b;
    }
}

void I_UpdateNoBlit(void)
{
}

void I_FinishUpdate(void)
{
    if (!texture || !screens[0])
        return;

    // Конвертируем 8-битный буфер Doom в 32-битный ARGB через текущую палитру
    const byte* src = screens[0];
    for (int i = 0; i < SCREENWIDTH * SCREENHEIGHT; i++)
    {
        frame_buffer[i] = palette_argb[src[i]];
    }

    SDL_UpdateTexture(texture, NULL, frame_buffer, SCREENWIDTH * sizeof(uint32_t));
    SDL_RenderClear(renderer);
    SDL_RenderTexture(renderer, texture, NULL, NULL);
    SDL_RenderPresent(renderer);
}

void I_ReadScreen(byte* scr)
{
    memcpy(scr, screens[0], SCREENWIDTH * SCREENHEIGHT);
}

void I_StartTic(void)
{
    SDL_Event sdl_event;
    event_t doom_event;

    while (SDL_PollEvent(&sdl_event))
    {
        switch (sdl_event.type)
        {
            case SDL_EVENT_QUIT:
                I_Quit();
                break;

            case SDL_EVENT_WINDOW_FOCUS_LOST:
                SetMouseGrab(0);
                break;

            case SDL_EVENT_WINDOW_FOCUS_GAINED:
                SetMouseGrab(1);
                break;

            case SDL_EVENT_KEY_DOWN:
                // Alt+Enter или F11 — переключение полного экрана
                if (sdl_event.key.key == SDLK_F11)
                {
                    bool is_fs = (SDL_GetWindowFlags(window) & SDL_WINDOW_FULLSCREEN) != 0;
                    SDL_SetWindowFullscreen(window, !is_fs);
                    break;
                }
                doom_event.type = ev_keydown;
                doom_event.data1 = TranslateKey(sdl_event.key.key);
                if (doom_event.data1)
                    D_PostEvent(&doom_event);
                break;

            case SDL_EVENT_KEY_UP:
                doom_event.type = ev_keyup;
                doom_event.data1 = TranslateKey(sdl_event.key.key);
                if (doom_event.data1)
                    D_PostEvent(&doom_event);
                break;

            case SDL_EVENT_MOUSE_BUTTON_DOWN:
            {
                if (!mouse_grabbed)
                    SetMouseGrab(1);

                SDL_MouseButtonFlags buttons = SDL_GetMouseState(NULL, NULL);
                doom_event.type = ev_mouse;
                doom_event.data1 =
                    ((buttons & SDL_BUTTON_LMASK) ? 1 : 0) |
                    ((buttons & SDL_BUTTON_RMASK) ? 2 : 0) |
                    ((buttons & SDL_BUTTON_MMASK) ? 4 : 0);
                doom_event.data2 = 0;
                doom_event.data3 = 0;
                D_PostEvent(&doom_event);
                break;
            }

            case SDL_EVENT_MOUSE_BUTTON_UP:
            {
                SDL_MouseButtonFlags buttons = SDL_GetMouseState(NULL, NULL);
                doom_event.type = ev_mouse;
                doom_event.data1 =
                    ((buttons & SDL_BUTTON_LMASK) ? 1 : 0) |
                    ((buttons & SDL_BUTTON_RMASK) ? 2 : 0) |
                    ((buttons & SDL_BUTTON_MMASK) ? 4 : 0);
                doom_event.data2 = 0;
                doom_event.data3 = 0;
                D_PostEvent(&doom_event);
                break;
            }

            case SDL_EVENT_MOUSE_MOTION:
                if (!mouse_grabbed)
                    break;
                doom_event.type = ev_mouse;
                doom_event.data1 =
                    ((sdl_event.motion.state & SDL_BUTTON_LMASK) ? 1 : 0) |
                    ((sdl_event.motion.state & SDL_BUTTON_RMASK) ? 2 : 0) |
                    ((sdl_event.motion.state & SDL_BUTTON_MMASK) ? 4 : 0);
                doom_event.data2 = (int)(sdl_event.motion.xrel * 3.0f);
                doom_event.data3 = 0; // Полностью отключаем ходьбу по вертикали мыши!
                D_PostEvent(&doom_event);
                break;
        }
    }
}


void I_StartFrame(void)
{
}
