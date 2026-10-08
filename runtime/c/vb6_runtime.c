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
