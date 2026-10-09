//! VB6 Open Runtime x64 C support library.
//! Clean-room implementation of observable VB6 runtime behaviors.

#include <windows.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>

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

// -------------------------------------------------------------
// VB6 String Runtime Subsystem
// -------------------------------------------------------------

char* vb6_string_alloc(const char* s) {
    if (!s) return _strdup("");
    return _strdup(s);
}

void vb6_string_free(char* s) {
    if (s) {
        free(s);
    }
}

char* vb6_string_concat(const char* s1, const char* s2) {
    const char* str1 = s1 ? s1 : "";
    const char* str2 = s2 ? s2 : "";
    size_t len1 = strlen(str1);
    size_t len2 = strlen(str2);
    char* res = (char*)malloc(len1 + len2 + 1);
    if (!res) return NULL;
    memcpy(res, str1, len1);
    memcpy(res + len1, str2, len2);
    res[len1 + len2] = '\0';
    return res;
}

int vb6_string_len(const char* s) {
    if (!s) return 0;
    return (int)strlen(s);
}

char* vb6_string_left(const char* s, int count) {
    if (!s || count <= 0) return _strdup("");
    int len = (int)strlen(s);
    if (count > len) count = len;
    char* res = (char*)malloc(count + 1);
    if (!res) return NULL;
    memcpy(res, s, count);
    res[count] = '\0';
    return res;
}

char* vb6_string_right(const char* s, int count) {
    if (!s || count <= 0) return _strdup("");
    int len = (int)strlen(s);
    if (count > len) count = len;
    char* res = (char*)malloc(count + 1);
    if (!res) return NULL;
    memcpy(res, s + (len - count), count);
    res[count] = '\0';
    return res;
}

char* vb6_string_mid(const char* s, int start, int length) {
    if (!s || start <= 0) return _strdup("");
    int len = (int)strlen(s);
    if (start > len) return _strdup("");
    int idx = start - 1;
    int avail = len - idx;
    int take = (length < 0 || length > avail) ? avail : length;
    char* res = (char*)malloc(take + 1);
    if (!res) return NULL;
    memcpy(res, s + idx, take);
    res[take] = '\0';
    return res;
}

int vb6_string_instr(int start, const char* s, const char* search) {
    if (start <= 0) start = 1;
    if (!s || !search || !*search) return 0;
    int len = (int)strlen(s);
    if (start > len) return 0;
    const char* sub = strstr(s + (start - 1), search);
    if (!sub) return 0;
    return (int)(sub - s) + 1;
}

char* vb6_string_ucase(const char* s) {
    if (!s) return _strdup("");
    size_t len = strlen(s);
    char* res = (char*)malloc(len + 1);
    if (!res) return NULL;
    for (size_t i = 0; i < len; i++) {
        res[i] = (char)toupper((unsigned char)s[i]);
    }
    res[len] = '\0';
    return res;
}

char* vb6_string_lcase(const char* s) {
    if (!s) return _strdup("");
    size_t len = strlen(s);
    char* res = (char*)malloc(len + 1);
    if (!res) return NULL;
    for (size_t i = 0; i < len; i++) {
        res[i] = (char)tolower((unsigned char)s[i]);
    }
    res[len] = '\0';
    return res;
}

char* vb6_string_trim(const char* s) {
    if (!s) return _strdup("");
    while (*s && isspace((unsigned char)*s)) {
        s++;
    }
    if (!*s) return _strdup("");
    const char* end = s + strlen(s) - 1;
    while (end > s && isspace((unsigned char)*end)) {
        end--;
    }
    size_t len = (size_t)(end - s + 1);
    char* res = (char*)malloc(len + 1);
    if (!res) return NULL;
    memcpy(res, s, len);
    res[len] = '\0';
    return res;
}

char* vb6_string_ltrim(const char* s) {
    if (!s) return _strdup("");
    while (*s && isspace((unsigned char)*s)) {
        s++;
    }
    return _strdup(s);
}

char* vb6_string_rtrim(const char* s) {
    if (!s) return _strdup("");
    size_t len = strlen(s);
    if (len == 0) return _strdup("");
    const char* end = s + len - 1;
    while (end >= s && isspace((unsigned char)*end)) {
        end--;
    }
    size_t new_len = (size_t)(end - s + 1);
    char* res = (char*)malloc(new_len + 1);
    if (!res) return NULL;
    memcpy(res, s, new_len);
    res[new_len] = '\0';
    return res;
}

char* vb6_string_space(int count) {
    if (count <= 0) return _strdup("");
    char* res = (char*)malloc(count + 1);
    if (!res) return NULL;
    memset(res, ' ', count);
    res[count] = '\0';
    return res;
}

char* vb6_string_string(int count, const char* char_str) {
    if (count <= 0 || !char_str || !*char_str) return _strdup("");
    char ch = char_str[0];
    char* res = (char*)malloc(count + 1);
    if (!res) return NULL;
    memset(res, ch, count);
    res[count] = '\0';
    return res;
}

char* vb6_string_replace(const char* expression, const char* find, const char* replace_with) {
    if (!expression) return _strdup("");
    if (!find || !*find) return _strdup(expression);
    const char* repl = replace_with ? replace_with : "";
    size_t find_len = strlen(find);
    size_t repl_len = strlen(repl);

    int count = 0;
    const char* p = expression;
    while ((p = strstr(p, find)) != NULL) {
        count++;
        p += find_len;
    }
    if (count == 0) return _strdup(expression);

    size_t expr_len = strlen(expression);
    size_t new_len = expr_len + count * (repl_len - find_len);
    char* res = (char*)malloc(new_len + 1);
    if (!res) return NULL;

    char* dst = res;
    p = expression;
    while (*p) {
        const char* next_match = strstr(p, find);
        if (next_match == p) {
            memcpy(dst, repl, repl_len);
            dst += repl_len;
            p += find_len;
        } else if (next_match) {
            size_t chunk = (size_t)(next_match - p);
            memcpy(dst, p, chunk);
            dst += chunk;
            p = next_match;
        } else {
            size_t rest = strlen(p);
            memcpy(dst, p, rest);
            dst += rest;
            break;
        }
    }
    *dst = '\0';
    return res;
}

char* vb6_string_chr(int code) {
    char* res = (char*)malloc(2);
    if (!res) return NULL;
    res[0] = (char)(code & 0xFF);
    res[1] = '\0';
    return res;
}

int vb6_string_asc(const char* s) {
    if (!s || !*s) return 0;
    return (int)(unsigned char)s[0];
}

double vb6_string_val(const char* s) {
    if (!s) return 0.0;
    while (*s && isspace((unsigned char)*s)) s++;
    return atof(s);
}

char* vb6_string_str(double val) {
    char buf[64];
    if (val == (double)(long long)val) {
        if (val >= 0) {
            sprintf_s(buf, sizeof(buf), " %lld", (long long)val);
        } else {
            sprintf_s(buf, sizeof(buf), "%lld", (long long)val);
        }
    } else {
        if (val >= 0) {
            sprintf_s(buf, sizeof(buf), " %g", val);
        } else {
            sprintf_s(buf, sizeof(buf), "%g", val);
        }
    }
    return _strdup(buf);
}

char* vb6_str_from_i64(long long val) {
    char buf[32];
    sprintf_s(buf, sizeof(buf), "%lld", val);
    return _strdup(buf);
}

char* vb6_str_from_f64(double val) {
    char buf[64];
    sprintf_s(buf, sizeof(buf), "%g", val);
    return _strdup(buf);
}

char* vb6_str_from_bool(int val) {
    return _strdup(val ? "True" : "False");
}

short vb6_string_equal(const char* s1, const char* s2) {
    const char* str1 = s1 ? s1 : "";
    const char* str2 = s2 ? s2 : "";
    return (strcmp(str1, str2) == 0) ? -1 : 0;
}

int vb6_string_compare(const char* s1, const char* s2) {
    const char* str1 = s1 ? s1 : "";
    const char* str2 = s2 ? s2 : "";
    return strcmp(str1, str2);
}

