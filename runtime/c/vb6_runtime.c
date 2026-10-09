//! VB6 Open Runtime x64 C support library.
//! Clean-room implementation of observable VB6 runtime behaviors.

#include <windows.h>
#include <stdio.h>

void vb6_init(void) {
    CoInitializeEx(NULL, COINIT_APARTMENTTHREADED);
}

void vb6_shutdown(void) {
    CoUninitialize();
}

int vb6_msgbox(const char* prompt, const char* title, unsigned int buttons) {
    if (!title) {
        title = "VB6 Application";
    }
    if (!prompt) {
        prompt = "";
    }
    return MessageBoxA(NULL, prompt, title, buttons);
}

void vb6_debug_print(const char* msg) {
    if (msg) {
        printf("%s\n", msg);
        OutputDebugStringA(msg);
    }
}

void vb6_debug_print_i8(char val) {
    printf("%d\n", (int)val);
    char buf[32];
    sprintf_s(buf, sizeof(buf), "%d\n", (int)val);
    OutputDebugStringA(buf);
}

void vb6_debug_print_i16(short val) {
    printf("%d\n", (int)val);
    char buf[32];
    sprintf_s(buf, sizeof(buf), "%d\n", (int)val);
    OutputDebugStringA(buf);
}

void vb6_debug_print_i32(int val) {
    printf("%d\n", val);
    char buf[32];
    sprintf_s(buf, sizeof(buf), "%d\n", val);
    OutputDebugStringA(buf);
}

void vb6_debug_print_i64(long long val) {
    printf("%lld\n", val);
    char buf[32];
    sprintf_s(buf, sizeof(buf), "%lld\n", val);
    OutputDebugStringA(buf);
}

void vb6_debug_print_f32(float val) {
    printf("%g\n", (double)val);
    char buf[64];
    sprintf_s(buf, sizeof(buf), "%g\n", (double)val);
    OutputDebugStringA(buf);
}

void vb6_debug_print_f64(double val) {
    printf("%g\n", val);
    char buf[64];
    sprintf_s(buf, sizeof(buf), "%g\n", val);
    OutputDebugStringA(buf);
}
