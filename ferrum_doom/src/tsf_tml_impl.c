#define TSF_IMPLEMENTATION
#include "../../src/platform/tsf.h"

#define TML_IMPLEMENTATION
#include "../../src/platform/tml.h"

#include "../../src/platform/mus2mid.h"

int rust_mus2mid_convert(const uint8_t* mus_data, uint8_t** out_mid, size_t* out_len) {
    return mus2mid_convert(mus_data, out_mid, out_len);
}

#include <stdarg.h>
#include <stdio.h>

extern void I_Error_rs(const char* msg);

void I_Error(const char* error, ...) {
    char buf[1024];
    va_list argptr;
    va_start(argptr, error);
    vsnprintf(buf, sizeof(buf), error, argptr);
    va_end(argptr);
    I_Error_rs(buf);
}
