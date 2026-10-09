//! VB6 Open Runtime x64 C support library.
//! Clean-room implementation of observable VB6 runtime behaviors.

#include <windows.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

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
        fflush(stdout);
        OutputDebugStringA(msg);
    }
}

void vb6_debug_print_i8(char val) {
    printf("%d\n", (int)val);
    fflush(stdout);
    char buf[32];
    sprintf_s(buf, sizeof(buf), "%d\n", (int)val);
    OutputDebugStringA(buf);
}

void vb6_debug_print_i16(short val) {
    printf("%d\n", (int)val);
    fflush(stdout);
    char buf[32];
    sprintf_s(buf, sizeof(buf), "%d\n", (int)val);
    OutputDebugStringA(buf);
}

void vb6_debug_print_i32(int val) {
    printf("%d\n", val);
    fflush(stdout);
    char buf[32];
    sprintf_s(buf, sizeof(buf), "%d\n", val);
    OutputDebugStringA(buf);
}

void vb6_debug_print_i64(long long val) {
    printf("%lld\n", val);
    fflush(stdout);
    char buf[32];
    sprintf_s(buf, sizeof(buf), "%lld\n", val);
    OutputDebugStringA(buf);
}

void vb6_debug_print_f32(float val) {
    printf("%g\n", (double)val);
    fflush(stdout);
    char buf[64];
    sprintf_s(buf, sizeof(buf), "%g\n", (double)val);
    OutputDebugStringA(buf);
}

void vb6_debug_print_f64(double val) {
    printf("%g\n", val);
    fflush(stdout);
    char buf[64];
    sprintf_s(buf, sizeof(buf), "%g\n", val);
    OutputDebugStringA(buf);
}

// -------------------------------------------------------------
// VB6 Array Runtime Subsystem
// -------------------------------------------------------------

typedef struct {
    int lbound;
    int ubound;
    int count;
} Vb6ArrayBound;

typedef struct {
    void* data;
    int dims;
    int elem_size;
    Vb6ArrayBound bounds[4];
} Vb6Array;

void* vb6_array_create(int dims, const int* bounds_pairs, int elem_size) {
    if (dims <= 0 || dims > 4 || elem_size <= 0) {
        return NULL;
    }
    Vb6Array* arr = (Vb6Array*)malloc(sizeof(Vb6Array));
    if (!arr) return NULL;

    arr->dims = dims;
    arr->elem_size = elem_size;
    int total_elements = 1;

    for (int i = 0; i < dims; i++) {
        int lb = bounds_pairs[i * 2];
        int ub = bounds_pairs[i * 2 + 1];
        int count = (ub >= lb) ? (ub - lb + 1) : 0;
        arr->bounds[i].lbound = lb;
        arr->bounds[i].ubound = ub;
        arr->bounds[i].count = count;
        total_elements *= count;
    }

    arr->data = calloc(total_elements > 0 ? total_elements : 1, elem_size);
    return arr;
}

void* vb6_array_element_ptr(Vb6Array* arr, int dims, const int* indices) {
    if (!arr || !arr->data) return NULL;

    int offset = 0;
    for (int i = 0; i < dims && i < arr->dims; i++) {
        int idx = indices[i];
        if (idx < arr->bounds[i].lbound || idx > arr->bounds[i].ubound) {
            // Out of bounds: safely fallback to lower bound
            idx = arr->bounds[i].lbound;
        }
        int off = idx - arr->bounds[i].lbound;
        offset = offset * arr->bounds[i].count + off;
    }

    return (char*)arr->data + (offset * arr->elem_size);
}

int vb6_array_lbound(const Vb6Array* arr, int dim) {
    if (!arr) return 0;
    int d = (dim <= 0) ? 0 : (dim - 1);
    if (d >= arr->dims) return 0;
    return arr->bounds[d].lbound;
}

int vb6_array_ubound(const Vb6Array* arr, int dim) {
    if (!arr) return 0;
    int d = (dim <= 0) ? 0 : (dim - 1);
    if (d >= arr->dims) return 0;
    return arr->bounds[d].ubound;
}

void* vb6_array_redim(Vb6Array* arr, int dims, const int* bounds_pairs, int elem_size, int preserve) {
    if (!arr) {
        return vb6_array_create(dims, bounds_pairs, elem_size);
    }
    if (dims <= 0 || dims > 4 || elem_size <= 0) {
        return arr;
    }

    int new_total = 1;
    Vb6ArrayBound new_bounds[4];
    for (int i = 0; i < dims; i++) {
        int lb = bounds_pairs[i * 2];
        int ub = bounds_pairs[i * 2 + 1];
        int count = (ub >= lb) ? (ub - lb + 1) : 0;
        new_bounds[i].lbound = lb;
        new_bounds[i].ubound = ub;
        new_bounds[i].count = count;
        new_total *= count;
    }

    void* new_data = calloc(new_total > 0 ? new_total : 1, elem_size);
    if (preserve && arr->data) {
        int old_total = 1;
        for (int i = 0; i < arr->dims; i++) {
            old_total *= arr->bounds[i].count;
        }
        int copy_count = (old_total < new_total) ? old_total : new_total;
        if (copy_count > 0 && new_data) {
            memcpy(new_data, arr->data, copy_count * elem_size);
        }
    }

    if (arr->data) {
        free(arr->data);
    }
    arr->data = new_data;
    arr->dims = dims;
    arr->elem_size = elem_size;
    for (int i = 0; i < dims; i++) {
        arr->bounds[i] = new_bounds[i];
    }
    return arr;
}

void vb6_array_free(Vb6Array* arr) {
    if (arr) {
        if (arr->data) {
            free(arr->data);
        }
        free(arr);
    }
}
