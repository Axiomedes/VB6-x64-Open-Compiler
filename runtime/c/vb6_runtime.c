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

void vb6_debug_print_i8(unsigned char val) {
    printf("%u\n", (unsigned int)val);
    fflush(stdout);
    char buf[32];
    sprintf_s(buf, sizeof(buf), "%u\n", (unsigned int)val);
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

// -------------------------------------------------------------
// VB6 Variant Runtime Subsystem
// -------------------------------------------------------------

#define VT_EMPTY    0
#define VT_NULL     1
#define VT_I2       2
#define VT_I4       3
#define VT_R4       4
#define VT_R8       5
#define VT_CY       6
#define VT_DATE     7
#define VT_BSTR     8
#define VT_DISPATCH 9
#define VT_ERROR    10
#define VT_BOOL     11
#define VT_VARIANT  12
#define VT_UNKNOWN  13
#define VT_DECIMAL  14
#define VT_I1       16
#define VT_UI1      17
#define VT_I8       20
#define VT_ARRAY    0x2000
#define VT_BYREF    0x4000

typedef struct {
    unsigned short vt;
    unsigned short reserved1;
    unsigned short reserved2;
    unsigned short reserved3;
    union {
        char cVal;
        unsigned char bVal;
        short iVal;
        int lVal;
        long long llVal;
        float fltVal;
        double dblVal;
        short boolVal;
        char* strVal;
        void* pArray;
        void* pObj;
        void* pVal;
        long long pad[2];
    } data;
} Vb6Variant;

void vb6_variant_init(Vb6Variant* v) {
    if (!v) return;
    memset(v, 0, sizeof(Vb6Variant));
    v->vt = VT_EMPTY;
}

void vb6_variant_clear(Vb6Variant* v) {
    if (!v) return;
    if (v->vt == VT_BSTR && v->data.strVal) {
        free(v->data.strVal);
        v->data.strVal = NULL;
    }
    memset(v, 0, sizeof(Vb6Variant));
    v->vt = VT_EMPTY;
}

void vb6_variant_copy(Vb6Variant* dest, const Vb6Variant* src) {
    if (!dest) return;
    if (dest == src) return;
    vb6_variant_clear(dest);
    if (!src) {
        dest->vt = VT_EMPTY;
        return;
    }
    memcpy(dest, src, sizeof(Vb6Variant));
    if (src->vt == VT_BSTR && src->data.strVal) {
        dest->data.strVal = _strdup(src->data.strVal);
    }
}

void vb6_variant_from_i8(Vb6Variant* v, char val) {
    if (!v) return;
    vb6_variant_clear(v);
    v->vt = VT_UI1;
    v->data.bVal = (unsigned char)val;
}

void vb6_variant_from_i16(Vb6Variant* v, short val) {
    if (!v) return;
    vb6_variant_clear(v);
    v->vt = VT_I2;
    v->data.iVal = val;
}

void vb6_variant_from_i32(Vb6Variant* v, int val) {
    if (!v) return;
    vb6_variant_clear(v);
    v->vt = VT_I4;
    v->data.lVal = val;
}

void vb6_variant_from_i64(Vb6Variant* v, long long val) {
    if (!v) return;
    vb6_variant_clear(v);
    v->vt = VT_I8;
    v->data.llVal = val;
}

void vb6_variant_from_f32(Vb6Variant* v, float val) {
    if (!v) return;
    vb6_variant_clear(v);
    v->vt = VT_R4;
    v->data.fltVal = val;
}

void vb6_variant_from_f64(Vb6Variant* v, double val) {
    if (!v) return;
    vb6_variant_clear(v);
    v->vt = VT_R8;
    v->data.dblVal = val;
}

void vb6_variant_from_bool(Vb6Variant* v, short val) {
    if (!v) return;
    vb6_variant_clear(v);
    v->vt = VT_BOOL;
    v->data.boolVal = val ? -1 : 0;
}

void vb6_variant_from_str(Vb6Variant* v, const char* s) {
    if (!v) return;
    vb6_variant_clear(v);
    v->vt = VT_BSTR;
    v->data.strVal = s ? _strdup(s) : _strdup("");
}

void vb6_variant_from_date(Vb6Variant* v, double val) {
    if (!v) return;
    vb6_variant_clear(v);
    v->vt = VT_DATE;
    v->data.dblVal = val;
}

void vb6_variant_from_empty(Vb6Variant* v) {
    if (!v) return;
    vb6_variant_clear(v);
    v->vt = VT_EMPTY;
}

void vb6_variant_from_null(Vb6Variant* v) {
    if (!v) return;
    vb6_variant_clear(v);
    v->vt = VT_NULL;
}

double vb6_variant_to_f64(const Vb6Variant* v) {
    if (!v) return 0.0;
    switch (v->vt & ~VT_ARRAY & ~VT_BYREF) {
        case VT_UI1: return (double)v->data.bVal;
        case VT_I2: return (double)v->data.iVal;
        case VT_I4: return (double)v->data.lVal;
        case VT_I8: return (double)v->data.llVal;
        case VT_R4: return (double)v->data.fltVal;
        case VT_R8:
        case VT_DATE: return v->data.dblVal;
        case VT_BOOL: return v->data.boolVal ? -1.0 : 0.0;
        case VT_BSTR: return v->data.strVal ? atof(v->data.strVal) : 0.0;
        default: return 0.0;
    }
}

int vb6_variant_to_i32(const Vb6Variant* v) {
    return (int)vb6_variant_to_f64(v);
}

short vb6_variant_to_i16(const Vb6Variant* v) {
    return (short)vb6_variant_to_f64(v);
}

char vb6_variant_to_i8(const Vb6Variant* v) {
    return (char)vb6_variant_to_f64(v);
}

long long vb6_variant_to_i64(const Vb6Variant* v) {
    if (v && v->vt == VT_I8) return v->data.llVal;
    return (long long)vb6_variant_to_f64(v);
}

float vb6_variant_to_f32(const Vb6Variant* v) {
    return (float)vb6_variant_to_f64(v);
}

short vb6_variant_to_bool(const Vb6Variant* v) {
    return (vb6_variant_to_f64(v) != 0.0) ? -1 : 0;
}

char* vb6_variant_to_str(const Vb6Variant* v) {
    if (!v) return _strdup("");
    switch (v->vt & ~VT_ARRAY & ~VT_BYREF) {
        case VT_EMPTY:
        case VT_NULL:
            return _strdup("");
        case VT_BSTR:
            return _strdup(v->data.strVal ? v->data.strVal : "");
        case VT_BOOL:
            return _strdup(v->data.boolVal ? "True" : "False");
        case VT_I8: {
            char buf[32];
            sprintf_s(buf, sizeof(buf), "%lld", v->data.llVal);
            return _strdup(buf);
        }
        case VT_UI1: {
            char buf[32];
            sprintf_s(buf, sizeof(buf), "%u", (unsigned int)v->data.bVal);
            return _strdup(buf);
        }
        case VT_I2: {
            char buf[32];
            sprintf_s(buf, sizeof(buf), "%d", (int)v->data.iVal);
            return _strdup(buf);
        }
        case VT_I4: {
            char buf[32];
            sprintf_s(buf, sizeof(buf), "%d", v->data.lVal);
            return _strdup(buf);
        }
        case VT_R4: {
            char buf[64];
            sprintf_s(buf, sizeof(buf), "%g", (double)v->data.fltVal);
            return _strdup(buf);
        }
        case VT_R8:
        case VT_DATE: {
            char buf[64];
            sprintf_s(buf, sizeof(buf), "%g", v->data.dblVal);
            return _strdup(buf);
        }
        default:
            return _strdup("");
    }
}

int vb6_variant_vartype(const Vb6Variant* v) {
    return v ? (int)v->vt : VT_EMPTY;
}

char* vb6_variant_typename(const Vb6Variant* v) {
    if (!v) return _strdup("Empty");
    switch (v->vt & ~VT_ARRAY & ~VT_BYREF) {
        case VT_EMPTY: return _strdup("Empty");
        case VT_NULL: return _strdup("Null");
        case VT_I2: return _strdup("Integer");
        case VT_I4: return _strdup("Long");
        case VT_R4: return _strdup("Single");
        case VT_R8: return _strdup("Double");
        case VT_CY: return _strdup("Currency");
        case VT_DATE: return _strdup("Date");
        case VT_BSTR: return _strdup("String");
        case VT_DISPATCH:
        case VT_UNKNOWN: return _strdup("Object");
        case VT_BOOL: return _strdup("Boolean");
        case VT_UI1: return _strdup("Byte");
        case VT_I8: return _strdup("LongLong");
        default: return _strdup("Variant");
    }
}

short vb6_variant_is_empty(const Vb6Variant* v) {
    return (!v || v->vt == VT_EMPTY) ? -1 : 0;
}

short vb6_variant_is_null(const Vb6Variant* v) {
    return (v && v->vt == VT_NULL) ? -1 : 0;
}

short vb6_variant_is_numeric(const Vb6Variant* v) {
    if (!v) return 0;
    switch (v->vt) {
        case VT_UI1:
        case VT_I2:
        case VT_I4:
        case VT_I8:
        case VT_R4:
        case VT_R8:
        case VT_CY:
        case VT_BOOL:
            return -1;
        case VT_BSTR: {
            if (!v->data.strVal || !*v->data.strVal) return 0;
            char* end;
            strtod(v->data.strVal, &end);
            while (*end && isspace((unsigned char)*end)) end++;
            return (*end == '\0') ? -1 : 0;
        }
        default:
            return 0;
    }
}

short vb6_variant_is_date(const Vb6Variant* v) {
    return (v && v->vt == VT_DATE) ? -1 : 0;
}

short vb6_variant_is_array(const Vb6Variant* v) {
    return (v && (v->vt & VT_ARRAY) != 0) ? -1 : 0;
}

void vb6_variant_add(Vb6Variant* res, const Vb6Variant* l, const Vb6Variant* r) {
    if (!res) return;
    if (l && r && l->vt == VT_BSTR && r->vt == VT_BSTR) {
        char* joined = vb6_string_concat(l->data.strVal, r->data.strVal);
        vb6_variant_from_str(res, joined);
        if (joined) free(joined);
        return;
    }
    if ((l && (l->vt == VT_R8 || l->vt == VT_R4)) || (r && (r->vt == VT_R8 || r->vt == VT_R4))) {
        double d = vb6_variant_to_f64(l) + vb6_variant_to_f64(r);
        vb6_variant_from_f64(res, d);
    } else if ((l && l->vt == VT_I8) || (r && r->vt == VT_I8)) {
        long long ll = vb6_variant_to_i64(l) + vb6_variant_to_i64(r);
        vb6_variant_from_i64(res, ll);
    } else {
        int i = vb6_variant_to_i32(l) + vb6_variant_to_i32(r);
        vb6_variant_from_i32(res, i);
    }
}

void vb6_variant_sub(Vb6Variant* res, const Vb6Variant* l, const Vb6Variant* r) {
    if (!res) return;
    if ((l && (l->vt == VT_R8 || l->vt == VT_R4)) || (r && (r->vt == VT_R8 || r->vt == VT_R4))) {
        vb6_variant_from_f64(res, vb6_variant_to_f64(l) - vb6_variant_to_f64(r));
    } else {
        vb6_variant_from_i32(res, vb6_variant_to_i32(l) - vb6_variant_to_i32(r));
    }
}

void vb6_variant_mul(Vb6Variant* res, const Vb6Variant* l, const Vb6Variant* r) {
    if (!res) return;
    if ((l && (l->vt == VT_R8 || l->vt == VT_R4)) || (r && (r->vt == VT_R8 || r->vt == VT_R4))) {
        vb6_variant_from_f64(res, vb6_variant_to_f64(l) * vb6_variant_to_f64(r));
    } else {
        vb6_variant_from_i32(res, vb6_variant_to_i32(l) * vb6_variant_to_i32(r));
    }
}

void vb6_variant_div(Vb6Variant* res, const Vb6Variant* l, const Vb6Variant* r) {
    if (!res) return;
    double den = vb6_variant_to_f64(r);
    vb6_variant_from_f64(res, den != 0.0 ? (vb6_variant_to_f64(l) / den) : 0.0);
}

void vb6_variant_int_div(Vb6Variant* res, const Vb6Variant* l, const Vb6Variant* r) {
    if (!res) return;
    int den = vb6_variant_to_i32(r);
    vb6_variant_from_i32(res, den != 0 ? (vb6_variant_to_i32(l) / den) : 0);
}

void vb6_variant_mod(Vb6Variant* res, const Vb6Variant* l, const Vb6Variant* r) {
    if (!res) return;
    int den = vb6_variant_to_i32(r);
    vb6_variant_from_i32(res, den != 0 ? (vb6_variant_to_i32(l) % den) : 0);
}

void vb6_variant_concat(Vb6Variant* res, const Vb6Variant* l, const Vb6Variant* r) {
    if (!res) return;
    char* s1 = vb6_variant_to_str(l);
    char* s2 = vb6_variant_to_str(r);
    char* joined = vb6_string_concat(s1, s2);
    vb6_variant_from_str(res, joined);
    if (s1) free(s1);
    if (s2) free(s2);
    if (joined) free(joined);
}

void vb6_variant_neg(Vb6Variant* res, const Vb6Variant* v) {
    if (!res) return;
    if (v && (v->vt == VT_R8 || v->vt == VT_R4)) {
        vb6_variant_from_f64(res, -vb6_variant_to_f64(v));
    } else {
        vb6_variant_from_i32(res, -vb6_variant_to_i32(v));
    }
}

void vb6_variant_not(Vb6Variant* res, const Vb6Variant* v) {
    if (!res) return;
    int val = vb6_variant_to_i32(v);
    vb6_variant_from_i32(res, ~val);
}

void vb6_variant_and(Vb6Variant* res, const Vb6Variant* l, const Vb6Variant* r) {
    if (!res) return;
    vb6_variant_from_i32(res, vb6_variant_to_i32(l) & vb6_variant_to_i32(r));
}

void vb6_variant_or(Vb6Variant* res, const Vb6Variant* l, const Vb6Variant* r) {
    if (!res) return;
    vb6_variant_from_i32(res, vb6_variant_to_i32(l) | vb6_variant_to_i32(r));
}

void vb6_variant_xor(Vb6Variant* res, const Vb6Variant* l, const Vb6Variant* r) {
    if (!res) return;
    vb6_variant_from_i32(res, vb6_variant_to_i32(l) ^ vb6_variant_to_i32(r));
}

int vb6_variant_compare(const Vb6Variant* l, const Vb6Variant* r) {
    if (!l && !r) return 0;
    if (!l) return -1;
    if (!r) return 1;
    if (l->vt == VT_BSTR && r->vt == VT_BSTR) {
        return strcmp(l->data.strVal ? l->data.strVal : "", r->data.strVal ? r->data.strVal : "");
    }
    double dl = vb6_variant_to_f64(l);
    double dr = vb6_variant_to_f64(r);
    return (dl < dr) ? -1 : (dl > dr ? 1 : 0);
}

short vb6_variant_equal(const Vb6Variant* l, const Vb6Variant* r) {
    return (vb6_variant_compare(l, r) == 0) ? -1 : 0;
}

void vb6_debug_print_variant(const Vb6Variant* v) {
    char* s = vb6_variant_to_str(v);
    vb6_debug_print(s);
    if (s) free(s);
}

// Memory and UDT helpers
void vb6_memory_copy(void* dest, const void* src, size_t size) {
    if (!dest || !src || size == 0) return;
    memcpy(dest, src, size);
}

void vb6_memory_zero(void* dest, size_t size) {
    if (!dest || size == 0) return;
    memset(dest, 0, size);
}

void vb6_fixed_str_set(char* dest, size_t fixed_len, const char* src) {
    if (!dest || fixed_len == 0) return;
    size_t src_len = src ? strlen(src) : 0;
    if (src_len >= fixed_len) {
        memcpy(dest, src, fixed_len);
    } else {
        if (src_len > 0) {
            memcpy(dest, src, src_len);
        }
        memset(dest + src_len, ' ', fixed_len - src_len);
    }
}

char* vb6_fixed_str_get(const char* src, size_t fixed_len) {
    char* res = (char*)malloc(fixed_len + 1);
    if (!res) return NULL;
    if (src && fixed_len > 0) {
        memcpy(res, src, fixed_len);
    } else {
        memset(res, ' ', fixed_len);
    }
    res[fixed_len] = '\0';
    return res;
}


