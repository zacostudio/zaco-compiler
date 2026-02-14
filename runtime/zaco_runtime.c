/**
 * Zaco Runtime Library
 *
 * Minimal runtime for memory management (reference counting),
 * string operations, and basic I/O for the Zaco TypeScript compiler.
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <math.h>
#include <ctype.h>
#include <time.h>
#include <setjmp.h>
#include <pthread.h>
#include <unistd.h>

/* ========== Memory Layout ==========
 * Every heap-allocated object has a header:
 * [ref_count: i64][size: i64][data...]
 * Header is 16 bytes, data starts at offset 16.
 */

#define HEADER_SIZE 16
#define RC_OFFSET   0
#define SIZE_OFFSET  8

/* ========== Allocation ========== */

void* zaco_alloc(int64_t size) {
    void* ptr = calloc(1, HEADER_SIZE + size);
    if (!ptr) {
        fprintf(stderr, "zaco: out of memory\n");
        exit(1);
    }
    // Initialize ref count to 1
    *((int64_t*)ptr) = 1;
    *((int64_t*)((char*)ptr + SIZE_OFFSET)) = size;
    // Return pointer to data (past header)
    return (char*)ptr + HEADER_SIZE;
}

void zaco_free(void* data_ptr) {
    if (!data_ptr) return;
    void* real_ptr = (char*)data_ptr - HEADER_SIZE;
    free(real_ptr);
}

/* ========== Reference Counting ========== */

void zaco_rc_inc(void* data_ptr) {
    if (!data_ptr) return;
    int64_t* rc = (int64_t*)((char*)data_ptr - HEADER_SIZE);
    (*rc)++;
}

void zaco_rc_dec(void* data_ptr) {
    if (!data_ptr) return;
    int64_t* rc = (int64_t*)((char*)data_ptr - HEADER_SIZE);
    (*rc)--;
    if (*rc <= 0) {
        zaco_free(data_ptr);
    }
}

int64_t zaco_rc_get(void* data_ptr) {
    if (!data_ptr) return 0;
    int64_t* rc = (int64_t*)((char*)data_ptr - HEADER_SIZE);
    return *rc;
}

/* ========== String Operations ========== */

void* zaco_str_new(const char* s) {
    int64_t len = strlen(s);
    void* ptr = zaco_alloc(len + 1);
    memcpy(ptr, s, len + 1);
    return ptr;
}

void* zaco_str_concat(void* a, void* b) {
    if (!a && !b) return zaco_str_new("");
    if (!a) { zaco_rc_inc(b); return b; }
    if (!b) { zaco_rc_inc(a); return a; }

    int64_t len_a = strlen((char*)a);
    int64_t len_b = strlen((char*)b);
    void* result = zaco_alloc(len_a + len_b + 1);
    memcpy(result, a, len_a);
    memcpy((char*)result + len_a, b, len_b + 1);
    return result;
}

int64_t zaco_str_len(void* s) {
    if (!s) return 0;
    return (int64_t)strlen((char*)s);
}

int64_t zaco_str_eq(void* a, void* b) {
    if (a == b) return 1;
    if (!a || !b) return 0;
    return strcmp((char*)a, (char*)b) == 0 ? 1 : 0;
}

/* ========== Number to String ========== */

void* zaco_i64_to_str(int64_t n) {
    char buf[32];
    snprintf(buf, sizeof(buf), "%lld", (long long)n);
    return zaco_str_new(buf);
}

void* zaco_f64_to_str(double n) {
    char buf[64];
    if (floor(n) == n && fabs(n) < 1e15) {
        snprintf(buf, sizeof(buf), "%.0f", n);
    } else {
        snprintf(buf, sizeof(buf), "%g", n);
    }
    return zaco_str_new(buf);
}

/* ========== Console I/O ========== */

void zaco_print_str(void* s) {
    if (s) {
        printf("%s", (char*)s);
    }
}

void zaco_print_i64(int64_t n) {
    printf("%lld", (long long)n);
}

void zaco_print_f64(double n) {
    if (floor(n) == n && fabs(n) < 1e15) {
        printf("%.0f", n);
    } else {
        printf("%g", n);
    }
}

void zaco_print_bool(int64_t b) {
    printf("%s", b ? "true" : "false");
}

void zaco_println_str(void* s) {
    zaco_print_str(s);
    printf("\n");
}

void zaco_println_i64(int64_t n) {
    zaco_print_i64(n);
    printf("\n");
}

/* ========== Array Operations ========== */

typedef struct {
    int64_t length;
    int64_t capacity;
    int64_t elem_size;
    void*   data;
} ZacoArray;

void* zaco_array_new(int64_t elem_size, int64_t initial_capacity) {
    ZacoArray* arr = (ZacoArray*)zaco_alloc(sizeof(ZacoArray));
    arr->length = 0;
    arr->capacity = initial_capacity > 0 ? initial_capacity : 8;
    arr->elem_size = elem_size;
    arr->data = zaco_alloc(arr->capacity * elem_size);
    return arr;
}

void zaco_array_push(void* array_ptr, void* elem) {
    ZacoArray* arr = (ZacoArray*)array_ptr;
    if (arr->length >= arr->capacity) {
        arr->capacity *= 2;
        void* new_data = zaco_alloc(arr->capacity * arr->elem_size);
        memcpy(new_data, arr->data, arr->length * arr->elem_size);
        zaco_free(arr->data);
        arr->data = new_data;
    }
    memcpy((char*)arr->data + arr->length * arr->elem_size, elem, arr->elem_size);
    arr->length++;
}

void* zaco_array_get(void* array_ptr, int64_t index) {
    ZacoArray* arr = (ZacoArray*)array_ptr;
    if (index < 0 || index >= arr->length) {
        fprintf(stderr, "zaco: array index out of bounds: %lld (length: %lld)\n",
                (long long)index, (long long)arr->length);
        exit(1);
    }
    return (char*)arr->data + index * arr->elem_size;
}

int64_t zaco_array_len(void* array_ptr) {
    ZacoArray* arr = (ZacoArray*)array_ptr;
    return arr->length;
}

/* Fix #2: Free array inner data buffer, then the array struct itself.
 * Call this instead of zaco_rc_dec for arrays, or use it when
 * the array ref count reaches 0. */
void zaco_array_destroy(void* array_ptr) {
    if (!array_ptr) return;
    ZacoArray* arr = (ZacoArray*)array_ptr;
    if (arr->data) {
        zaco_free(arr->data);
        arr->data = NULL;
    }
    zaco_free(array_ptr);
}

void zaco_array_rc_dec(void* array_ptr) {
    if (!array_ptr) return;
    int64_t* rc = (int64_t*)((char*)array_ptr - HEADER_SIZE);
    (*rc)--;
    if (*rc <= 0) {
        zaco_array_destroy(array_ptr);
    }
}

/* ========== Math Functions ========== */

double zaco_math_floor(double x) {
    return floor(x);
}

double zaco_math_ceil(double x) {
    return ceil(x);
}

double zaco_math_round(double x) {
    return round(x);
}

double zaco_math_abs(double x) {
    return fabs(x);
}

double zaco_math_sqrt(double x) {
    return sqrt(x);
}

double zaco_math_pow(double x, double y) {
    return pow(x, y);
}

double zaco_math_sin(double x) {
    return sin(x);
}

double zaco_math_cos(double x) {
    return cos(x);
}

double zaco_math_tan(double x) {
    return tan(x);
}

double zaco_math_log(double x) {
    return log(x);
}

double zaco_math_log2(double x) {
    return log2(x);
}

double zaco_math_log10(double x) {
    return log10(x);
}

double zaco_math_random() {
    static int initialized = 0;
    if (!initialized) {
        srand((unsigned int)time(NULL));
        initialized = 1;
    }
    return (double)rand() / (double)RAND_MAX;
}

double zaco_math_min(double a, double b) {
    return a < b ? a : b;
}

double zaco_math_max(double a, double b) {
    return a > b ? a : b;
}

int64_t zaco_math_trunc(double x) {
    return (int64_t)x;
}

double zaco_math_pi() {
    return M_PI;
}

double zaco_math_e() {
    return M_E;
}

/* ========== JSON Functions ========== */

// Minimal JSON parser - handles basic primitives and simple structures
void* zaco_json_parse(void* json_str) {
    if (!json_str) return NULL;

    const char* s = (const char*)json_str;
    // Skip leading whitespace
    while (*s && isspace(*s)) s++;

    // Parse strings — Fix #7: handle escape sequences properly
    if (*s == '"') {
        s++; // skip opening quote
        // First pass: calculate output length
        const char* scan = s;
        size_t out_len = 0;
        while (*scan && *scan != '"') {
            if (*scan == '\\' && *(scan+1)) {
                scan += 2; // skip escape pair
            } else {
                scan++;
            }
            out_len++;
        }
        char* result_buf = malloc(out_len + 1);
        size_t wi = 0;
        while (*s && *s != '"') {
            if (*s == '\\' && *(s+1)) {
                s++; // skip backslash
                switch (*s) {
                    case 'n':  result_buf[wi++] = '\n'; break;
                    case 't':  result_buf[wi++] = '\t'; break;
                    case 'r':  result_buf[wi++] = '\r'; break;
                    case '"':  result_buf[wi++] = '"';  break;
                    case '\\': result_buf[wi++] = '\\'; break;
                    case '/':  result_buf[wi++] = '/';  break;
                    default:   result_buf[wi++] = *s;   break;
                }
                s++;
            } else {
                result_buf[wi++] = *s++;
            }
        }
        result_buf[wi] = '\0';
        void* result = zaco_str_new(result_buf);
        free(result_buf);
        return result;
    }

    // Parse booleans
    if (strncmp(s, "true", 4) == 0) {
        return zaco_str_new("true");
    }
    if (strncmp(s, "false", 5) == 0) {
        return zaco_str_new("false");
    }

    // Parse null
    if (strncmp(s, "null", 4) == 0) {
        return zaco_str_new("null");
    }

    // Parse numbers - return as string for now
    if (*s == '-' || isdigit(*s)) {
        const char* start = s;
        if (*s == '-') s++;
        while (*s && isdigit(*s)) s++;
        if (*s == '.') {
            s++;
            while (*s && isdigit(*s)) s++;
        }
        size_t len = s - start;
        char* result_buf = malloc(len + 1);
        memcpy(result_buf, start, len);
        result_buf[len] = '\0';
        void* result = zaco_str_new(result_buf);
        free(result_buf);
        return result;
    }

    // For arrays/objects, return string representation for now
    return zaco_str_new((const char*)json_str);
}

// Minimal JSON stringifier - handles basic primitives
void* zaco_json_stringify(void* value) {
    if (!value) {
        return zaco_str_new("null");
    }

    // For now, assume value is a string and just quote it
    // More sophisticated handling would check type
    const char* s = (const char*)value;

    // Check if it's already a JSON primitive (number, boolean, null)
    if (strcmp(s, "true") == 0 || strcmp(s, "false") == 0 || strcmp(s, "null") == 0) {
        return zaco_str_new(s);
    }

    // Check if it's a number
    char* endptr;
    strtod(s, &endptr);
    if (*endptr == '\0' && *s != '\0') {
        return zaco_str_new(s);
    }

    // Otherwise, quote it as a string with proper escaping
    size_t len = strlen(s);

    // First pass: calculate escaped length
    size_t escaped_len = 0;
    for (size_t i = 0; i < len; i++) {
        switch (s[i]) {
            case '"':  escaped_len += 2; break; /* \" */
            case '\\': escaped_len += 2; break; /* \\ */
            case '\n': escaped_len += 2; break; /* \n */
            case '\t': escaped_len += 2; break; /* \t */
            case '\r': escaped_len += 2; break; /* \r */
            case '\b': escaped_len += 2; break; /* \b */
            case '\f': escaped_len += 2; break; /* \f */
            default:   escaped_len += 1; break;
        }
    }

    // Second pass: build escaped string
    char* buf = malloc(escaped_len + 3); // quotes + null
    buf[0] = '"';
    size_t pos = 1;
    for (size_t i = 0; i < len; i++) {
        switch (s[i]) {
            case '"':  buf[pos++] = '\\'; buf[pos++] = '"';  break;
            case '\\': buf[pos++] = '\\'; buf[pos++] = '\\'; break;
            case '\n': buf[pos++] = '\\'; buf[pos++] = 'n';  break;
            case '\t': buf[pos++] = '\\'; buf[pos++] = 't';  break;
            case '\r': buf[pos++] = '\\'; buf[pos++] = 'r';  break;
            case '\b': buf[pos++] = '\\'; buf[pos++] = 'b';  break;
            case '\f': buf[pos++] = '\\'; buf[pos++] = 'f';  break;
            default:   buf[pos++] = s[i]; break;
        }
    }
    buf[pos++] = '"';
    buf[pos] = '\0';
    void* result = zaco_str_new(buf);
    free(buf);
    return result;
}

/* ========== Enhanced Console Functions ========== */

void zaco_console_error_str(void* s) {
    if (s) {
        fprintf(stderr, "%s", (char*)s);
    }
}

void zaco_console_error_i64(int64_t n) {
    fprintf(stderr, "%lld", (long long)n);
}

void zaco_console_error_f64(double n) {
    if (floor(n) == n && fabs(n) < 1e15) {
        fprintf(stderr, "%.0f", n);
    } else {
        fprintf(stderr, "%g", n);
    }
}

void zaco_console_error_bool(int64_t b) {
    fprintf(stderr, "%s", b ? "true" : "false");
}

void zaco_console_errorln(void* s) {
    zaco_console_error_str(s);
    fprintf(stderr, "\n");
}

void zaco_console_warn_str(void* s) {
    if (s) {
        fprintf(stderr, "%s", (char*)s);
    }
}

void zaco_console_warn_i64(int64_t n) {
    fprintf(stderr, "%lld", (long long)n);
}

void zaco_console_warnln(void* s) {
    zaco_console_warn_str(s);
    fprintf(stderr, "\n");
}

void zaco_console_debug_str(void* s) {
    if (s) {
        fprintf(stdout, "%s", (char*)s);
    }
}

void zaco_console_debug_i64(int64_t n) {
    fprintf(stdout, "%lld", (long long)n);
}

void zaco_console_debug_f64(double n) {
    if (floor(n) == n && fabs(n) < 1e15) {
        fprintf(stdout, "%.0f", n);
    } else {
        fprintf(stdout, "%g", n);
    }
}

void zaco_console_debug_bool(int64_t b) {
    fprintf(stdout, "%s", b ? "true" : "false");
}

void zaco_console_debugln(void* s) {
    zaco_console_debug_str(s);
    fprintf(stdout, "\n");
}

/* ========== String Methods ========== */

void* zaco_str_slice(void* s, int64_t start, int64_t end) {
    if (!s) return zaco_str_new("");

    int64_t len = strlen((char*)s);

    // Handle negative indices
    if (start < 0) start = len + start;
    if (end < 0) end = len + end;

    // Clamp to valid range
    if (start < 0) start = 0;
    if (end < 0) end = 0;
    if (start > len) start = len;
    if (end > len) end = len;
    if (start > end) start = end;

    int64_t slice_len = end - start;
    /* Fix #13: single allocation via zaco_alloc */
    void* result = zaco_alloc(slice_len + 1);
    memcpy(result, (char*)s + start, slice_len);
    ((char*)result)[slice_len] = '\0';
    return result;
}

void* zaco_str_to_upper(void* s) {
    if (!s) return zaco_str_new("");

    int64_t len = strlen((char*)s);
    /* Fix #13: single allocation */
    void* result = zaco_alloc(len + 1);
    for (int64_t i = 0; i < len; i++) {
        ((char*)result)[i] = toupper(((char*)s)[i]);
    }
    ((char*)result)[len] = '\0';
    return result;
}

void* zaco_str_to_lower(void* s) {
    if (!s) return zaco_str_new("");

    int64_t len = strlen((char*)s);
    /* Fix #13: single allocation */
    void* result = zaco_alloc(len + 1);
    for (int64_t i = 0; i < len; i++) {
        ((char*)result)[i] = tolower(((char*)s)[i]);
    }
    ((char*)result)[len] = '\0';
    return result;
}

void* zaco_str_trim(void* s) {
    if (!s) return zaco_str_new("");

    const char* str = (const char*)s;
    const char* start = str;

    // Trim leading whitespace
    while (*start && isspace(*start)) start++;

    if (*start == '\0') return zaco_str_new("");

    // Trim trailing whitespace
    const char* end = str + strlen(str) - 1;
    while (end > start && isspace(*end)) end--;

    int64_t len = end - start + 1;
    /* Fix #13: single allocation */
    void* result = zaco_alloc(len + 1);
    memcpy(result, start, len);
    ((char*)result)[len] = '\0';
    return result;
}

int64_t zaco_str_index_of(void* s, void* search) {
    if (!s || !search) return -1;

    const char* found = strstr((char*)s, (char*)search);
    if (!found) return -1;
    return (int64_t)(found - (char*)s);
}

int64_t zaco_str_includes(void* s, void* search) {
    return zaco_str_index_of(s, search) >= 0 ? 1 : 0;
}

void* zaco_str_replace(void* s, void* search, void* replace) {
    if (!s || !search) {
        if (s) {
            zaco_rc_inc(s);
            return s;
        }
        return zaco_str_new("");
    }

    const char* str = (const char*)s;
    const char* search_str = (const char*)search;
    const char* replace_str = replace ? (const char*)replace : "";

    const char* found = strstr(str, search_str);
    if (!found) {
        zaco_rc_inc(s);
        return s;
    }

    int64_t search_len = strlen(search_str);
    int64_t replace_len = strlen(replace_str);
    int64_t prefix_len = found - str;
    int64_t suffix_len = strlen(found + search_len);
    int64_t total_len = prefix_len + replace_len + suffix_len;

    char* buf = malloc(total_len + 1);
    memcpy(buf, str, prefix_len);
    memcpy(buf + prefix_len, replace_str, replace_len);
    memcpy(buf + prefix_len + replace_len, found + search_len, suffix_len);
    buf[total_len] = '\0';

    void* result = zaco_str_new(buf);
    free(buf);
    return result;
}

void* zaco_str_split(void* s, void* separator) {
    if (!s) {
        return zaco_array_new(sizeof(void*), 0);
    }

    const char* str = (const char*)s;
    const char* sep = separator ? (const char*)separator : "";
    int64_t sep_len = strlen(sep);

    ZacoArray* result = (ZacoArray*)zaco_array_new(sizeof(void*), 4);

    if (sep_len == 0) {
        // Split every character
        int64_t len = strlen(str);
        for (int64_t i = 0; i < len; i++) {
            char buf[2] = {str[i], '\0'};
            void* elem = zaco_str_new(buf);
            zaco_array_push(result, &elem);
        }
        return result;
    }

    const char* current = str;
    const char* found;

    while ((found = strstr(current, sep)) != NULL) {
        int64_t len = found - current;
        char* buf = malloc(len + 1);
        memcpy(buf, current, len);
        buf[len] = '\0';
        void* elem = zaco_str_new(buf);
        free(buf);
        zaco_array_push(result, &elem);
        current = found + sep_len;
    }

    // Add remaining part
    void* elem = zaco_str_new(current);
    zaco_array_push(result, &elem);

    return result;
}

int64_t zaco_str_starts_with(void* s, void* prefix) {
    if (!s || !prefix) return 0;

    const char* str = (const char*)s;
    const char* pre = (const char*)prefix;
    int64_t pre_len = strlen(pre);

    return strncmp(str, pre, pre_len) == 0 ? 1 : 0;
}

int64_t zaco_str_ends_with(void* s, void* suffix) {
    if (!s || !suffix) return 0;

    const char* str = (const char*)s;
    const char* suf = (const char*)suffix;
    int64_t str_len = strlen(str);
    int64_t suf_len = strlen(suf);

    if (suf_len > str_len) return 0;

    return strcmp(str + str_len - suf_len, suf) == 0 ? 1 : 0;
}

void* zaco_str_char_at(void* s, int64_t index) {
    if (!s) return zaco_str_new("");

    int64_t len = strlen((char*)s);
    if (index < 0 || index >= len) return zaco_str_new("");

    char buf[2] = {((char*)s)[index], '\0'};
    return zaco_str_new(buf);
}

void* zaco_str_repeat(void* s, int64_t count) {
    if (!s || count <= 0) return zaco_str_new("");

    int64_t len = strlen((char*)s);
    /* Fix #8: overflow check before multiplication */
    if (len == 0) return zaco_str_new("");
    if (count > INT64_MAX / len) return zaco_str_new(""); /* overflow */

    int64_t total_len = len * count;
    /* Fix #13: Use zaco_alloc directly instead of malloc→zaco_str_new→free */
    void* result = zaco_alloc(total_len + 1);
    for (int64_t i = 0; i < count; i++) {
        memcpy((char*)result + i * len, s, len);
    }
    ((char*)result)[total_len] = '\0';
    return result;
}

void* zaco_str_pad_start(void* s, int64_t target_len, void* pad_str) {
    /* Fix #3: When s is null, create a proper managed empty string
     * instead of using a string literal (which has no header and
     * would cause UB when zaco_rc_inc tries to write to ptr-16). */
    int need_free_s = 0;
    if (!s) { s = zaco_str_new(""); need_free_s = 1; }

    int64_t current_len = strlen((char*)s);
    if (current_len >= target_len) {
        if (!need_free_s) zaco_rc_inc(s);
        /* if need_free_s, s already has rc=1, just return it */
        return s;
    }

    const char* pad = pad_str ? (const char*)pad_str : " ";
    int64_t pad_len = strlen(pad);
    if (pad_len == 0) {
        if (!need_free_s) zaco_rc_inc(s);
        return s;
    }

    int64_t fill_len = target_len - current_len;
    /* Fix #13: Use zaco_alloc directly instead of malloc→zaco_str_new→free */
    void* result = zaco_alloc(target_len + 1);

    int64_t pos = 0;
    while (pos < fill_len) {
        int64_t copy_len = fill_len - pos < pad_len ? fill_len - pos : pad_len;
        memcpy((char*)result + pos, pad, copy_len);
        pos += copy_len;
    }

    memcpy((char*)result + fill_len, s, current_len);
    ((char*)result)[target_len] = '\0';

    if (need_free_s) zaco_free(s);
    return result;
}

void* zaco_str_pad_end(void* s, int64_t target_len, void* pad_str) {
    /* Fix #3: same as pad_start */
    int need_free_s = 0;
    if (!s) { s = zaco_str_new(""); need_free_s = 1; }

    int64_t current_len = strlen((char*)s);
    if (current_len >= target_len) {
        if (!need_free_s) zaco_rc_inc(s);
        return s;
    }

    const char* pad = pad_str ? (const char*)pad_str : " ";
    int64_t pad_len = strlen(pad);
    if (pad_len == 0) {
        if (!need_free_s) zaco_rc_inc(s);
        return s;
    }

    int64_t fill_len = target_len - current_len;
    /* Fix #13: Use zaco_alloc directly */
    void* result = zaco_alloc(target_len + 1);

    memcpy(result, s, current_len);

    int64_t pos = current_len;
    while (pos < target_len) {
        int64_t copy_len = target_len - pos < pad_len ? target_len - pos : pad_len;
        memcpy((char*)result + pos, pad, copy_len);
        pos += copy_len;
    }

    ((char*)result)[target_len] = '\0';

    if (need_free_s) zaco_free(s);
    return result;
}

/* ========== Array Methods ========== */

void* zaco_array_slice(void* arr, int64_t start, int64_t end) {
    if (!arr) return zaco_array_new(sizeof(void*), 0);

    ZacoArray* array = (ZacoArray*)arr;
    int64_t len = array->length;

    // Handle negative indices
    if (start < 0) start = len + start;
    if (end < 0) end = len + end;

    // Clamp to valid range
    if (start < 0) start = 0;
    if (end < 0) end = 0;
    if (start > len) start = len;
    if (end > len) end = len;
    if (start > end) start = end;

    int64_t slice_len = end - start;
    ZacoArray* result = (ZacoArray*)zaco_array_new(array->elem_size, slice_len);

    for (int64_t i = 0; i < slice_len; i++) {
        void* elem = (char*)array->data + (start + i) * array->elem_size;
        zaco_array_push(result, elem);
    }

    return result;
}

void* zaco_array_concat(void* a, void* b) {
    if (!a && !b) return zaco_array_new(sizeof(void*), 0);
    if (!a) {
        zaco_rc_inc(b);
        return b;
    }
    if (!b) {
        zaco_rc_inc(a);
        return a;
    }

    ZacoArray* arr_a = (ZacoArray*)a;
    ZacoArray* arr_b = (ZacoArray*)b;

    ZacoArray* result = (ZacoArray*)zaco_array_new(arr_a->elem_size, arr_a->length + arr_b->length);

    for (int64_t i = 0; i < arr_a->length; i++) {
        void* elem = (char*)arr_a->data + i * arr_a->elem_size;
        zaco_array_push(result, elem);
    }

    for (int64_t i = 0; i < arr_b->length; i++) {
        void* elem = (char*)arr_b->data + i * arr_b->elem_size;
        zaco_array_push(result, elem);
    }

    return result;
}

int64_t zaco_array_index_of(void* arr, void* elem) {
    if (!arr || !elem) return -1;

    ZacoArray* array = (ZacoArray*)arr;

    // For pointer-sized elements (strings, objects), compare pointers
    if (array->elem_size == sizeof(void*)) {
        void* search_ptr = *(void**)elem;
        for (int64_t i = 0; i < array->length; i++) {
            void* current_ptr = *((void**)((char*)array->data + i * array->elem_size));
            if (current_ptr == search_ptr) {
                return i;
            }
            // For strings, also check content equality
            if (search_ptr && current_ptr && zaco_str_eq(search_ptr, current_ptr)) {
                return i;
            }
        }
    } else {
        // For primitive types, compare bytes
        for (int64_t i = 0; i < array->length; i++) {
            void* current = (char*)array->data + i * array->elem_size;
            if (memcmp(current, elem, array->elem_size) == 0) {
                return i;
            }
        }
    }

    return -1;
}

void* zaco_array_join(void* arr, void* separator) {
    if (!arr) return zaco_str_new("");

    ZacoArray* array = (ZacoArray*)arr;
    if (array->length == 0) return zaco_str_new("");

    const char* sep = separator ? (const char*)separator : ",";
    int64_t sep_len = strlen(sep);

    // Calculate total length needed
    int64_t total_len = 0;
    for (int64_t i = 0; i < array->length; i++) {
        void* elem_ptr = *((void**)((char*)array->data + i * array->elem_size));
        if (elem_ptr) {
            total_len += strlen((char*)elem_ptr);
        }
        if (i < array->length - 1) {
            total_len += sep_len;
        }
    }

    char* buf = malloc(total_len + 1);
    int64_t pos = 0;

    for (int64_t i = 0; i < array->length; i++) {
        void* elem_ptr = *((void**)((char*)array->data + i * array->elem_size));
        if (elem_ptr) {
            int64_t elem_len = strlen((char*)elem_ptr);
            memcpy(buf + pos, elem_ptr, elem_len);
            pos += elem_len;
        }
        if (i < array->length - 1) {
            memcpy(buf + pos, sep, sep_len);
            pos += sep_len;
        }
    }

    buf[pos] = '\0';
    void* result = zaco_str_new(buf);
    free(buf);
    return result;
}

void zaco_array_reverse(void* arr) {
    if (!arr) return;

    ZacoArray* array = (ZacoArray*)arr;
    if (array->length <= 1) return;

    void* temp = malloc(array->elem_size);

    for (int64_t i = 0; i < array->length / 2; i++) {
        int64_t j = array->length - 1 - i;
        void* left = (char*)array->data + i * array->elem_size;
        void* right = (char*)array->data + j * array->elem_size;

        memcpy(temp, left, array->elem_size);
        memcpy(left, right, array->elem_size);
        memcpy(right, temp, array->elem_size);
    }

    free(temp);
}

void* zaco_array_pop(void* arr) {
    if (!arr) return NULL;

    ZacoArray* array = (ZacoArray*)arr;
    if (array->length == 0) return NULL;

    array->length--;

    // For pointer-sized elements, return the pointer itself
    if (array->elem_size == sizeof(void*)) {
        return *((void**)((char*)array->data + array->length * array->elem_size));
    }

    // For other types, would need to return a copy
    return NULL;
}

/* ========== Process ========== */

void zaco_exit(int64_t code) {
    exit((int)code);
}

/* ========== Exception Handling (setjmp/longjmp) ========== */

#define MAX_TRY_DEPTH 64
static jmp_buf try_stack[MAX_TRY_DEPTH];
static int try_depth = 0;
static void* current_error = NULL;

int64_t zaco_try_push() {
    if (try_depth >= MAX_TRY_DEPTH) {
        fprintf(stderr, "zaco: try/catch nesting too deep\n");
        exit(1);
    }
    return setjmp(try_stack[try_depth++]);
}

void zaco_try_pop() {
    if (try_depth > 0) try_depth--;
}

void zaco_throw(void* error) {
    current_error = error;
    if (try_depth > 0) {
        try_depth--;
        longjmp(try_stack[try_depth], 1);
    }
    /* Uncaught exception */
    if (error) {
        fprintf(stderr, "Uncaught exception: %s\n", (char*)error);
    } else {
        fprintf(stderr, "Uncaught exception\n");
    }
    exit(1);
}

void* zaco_get_error() {
    return current_error;
}

void zaco_clear_error() {
    current_error = NULL;
}

/* ========== Global Number Functions ========== */

double zaco_parse_int(char* s) {
    if (!s) return 0.0 / 0.0; /* NaN */
    /* Skip leading whitespace */
    while (*s && isspace(*s)) s++;
    if (*s == '\0') return 0.0 / 0.0; /* NaN */

    char* endptr;
    double result = strtod(s, &endptr);
    if (endptr == s) return 0.0 / 0.0; /* NaN */
    return floor(result);
}

double zaco_parse_float(char* s) {
    if (!s) return 0.0 / 0.0; /* NaN */
    /* Skip leading whitespace */
    while (*s && isspace(*s)) s++;
    if (*s == '\0') return 0.0 / 0.0; /* NaN */

    char* endptr;
    double result = strtod(s, &endptr);
    if (endptr == s) return 0.0 / 0.0; /* NaN */
    return result;
}

int64_t zaco_is_nan(double n) {
    return isnan(n) ? 1 : 0;
}

int64_t zaco_is_finite(double n) {
    return isfinite(n) ? 1 : 0;
}

/* ========== Inline Array Helpers ==========
 * These work with the inline array format used by codegen:
 *   [length: i64][elem0][elem1]...
 * Each element is 8 bytes (f64 or pointer).
 */

int64_t zaco_array_length(void* arr) {
    if (!arr) return 0;
    return *((int64_t*)arr);
}

double zaco_array_get_f64(void* arr, int64_t index) {
    if (!arr) return 0.0;
    int64_t length = *((int64_t*)arr);
    if (index < 0 || index >= length) return 0.0;
    return *((double*)((char*)arr + 8 + index * 8));
}

void* zaco_array_get_ptr(void* arr, int64_t index) {
    if (!arr) return NULL;
    int64_t length = *((int64_t*)arr);
    if (index < 0 || index >= length) return NULL;
    return *((void**)((char*)arr + 8 + index * 8));
}

/* ========== Object (Key-Value Map) ========== */

typedef struct {
    char* key;
    uint64_t value_bits; /* Stores any 8-byte value via memcpy */
} ZacoObjEntry;

typedef struct {
    int64_t count;
    int64_t capacity;
    ZacoObjEntry* entries;
} ZacoObject;

static int64_t zaco_object_find(ZacoObject* obj, const char* key) {
    for (int64_t i = 0; i < obj->count; i++) {
        if (obj->entries[i].key && strcmp(obj->entries[i].key, key) == 0) {
            return i;
        }
    }
    return -1;
}

static void zaco_object_set_raw(ZacoObject* obj, const char* key, uint64_t bits) {
    int64_t idx = zaco_object_find(obj, key);
    if (idx >= 0) {
        obj->entries[idx].value_bits = bits;
        return;
    }
    if (obj->count >= obj->capacity) {
        obj->capacity *= 2;
        obj->entries = (ZacoObjEntry*)realloc(obj->entries, obj->capacity * sizeof(ZacoObjEntry));
    }
    obj->entries[obj->count].key = strdup(key);
    obj->entries[obj->count].value_bits = bits;
    obj->count++;
}

static uint64_t zaco_object_get_raw(ZacoObject* obj, const char* key) {
    int64_t idx = zaco_object_find(obj, key);
    if (idx >= 0) return obj->entries[idx].value_bits;
    return 0;
}

void* zaco_object_new(void) {
    ZacoObject* obj = (ZacoObject*)malloc(sizeof(ZacoObject));
    if (!obj) {
        fprintf(stderr, "zaco: out of memory (object)\n");
        exit(1);
    }
    obj->count = 0;
    obj->capacity = 8;
    obj->entries = (ZacoObjEntry*)calloc(obj->capacity, sizeof(ZacoObjEntry));
    return obj;
}

void zaco_object_set_str(void* o, const char* key, const char* value) {
    uint64_t bits;
    memcpy(&bits, &value, sizeof(bits));
    zaco_object_set_raw((ZacoObject*)o, key, bits);
}

void zaco_object_set_f64(void* o, const char* key, double value) {
    uint64_t bits;
    memcpy(&bits, &value, sizeof(bits));
    zaco_object_set_raw((ZacoObject*)o, key, bits);
}

void zaco_object_set_i64(void* o, const char* key, int64_t value) {
    uint64_t bits;
    memcpy(&bits, &value, sizeof(bits));
    zaco_object_set_raw((ZacoObject*)o, key, bits);
}

void zaco_object_set_ptr(void* o, const char* key, void* value) {
    uint64_t bits;
    memcpy(&bits, &value, sizeof(bits));
    zaco_object_set_raw((ZacoObject*)o, key, bits);
}

const char* zaco_object_get_str(void* o, const char* key) {
    uint64_t bits = zaco_object_get_raw((ZacoObject*)o, key);
    const char* result;
    memcpy(&result, &bits, sizeof(result));
    return result;
}

double zaco_object_get_f64(void* o, const char* key) {
    uint64_t bits = zaco_object_get_raw((ZacoObject*)o, key);
    double result;
    memcpy(&result, &bits, sizeof(result));
    return result;
}

int64_t zaco_object_get_i64(void* o, const char* key) {
    uint64_t bits = zaco_object_get_raw((ZacoObject*)o, key);
    int64_t result;
    memcpy(&result, &bits, sizeof(result));
    return result;
}

void* zaco_object_get_ptr(void* o, const char* key) {
    uint64_t bits = zaco_object_get_raw((ZacoObject*)o, key);
    void* result;
    memcpy(&result, &bits, sizeof(result));
    return result;
}

int64_t zaco_object_has(void* o, const char* key) {
    if (!o) return 0;
    return zaco_object_find((ZacoObject*)o, key) >= 0 ? 1 : 0;
}

void zaco_object_free(void* o) {
    if (!o) return;
    ZacoObject* obj = (ZacoObject*)o;
    for (int64_t i = 0; i < obj->count; i++) {
        free(obj->entries[i].key);
    }
    free(obj->entries);
    free(obj);
}

/* ========== Missing Console Warn Functions ========== */

void zaco_console_warn_f64(double n) {
    if (floor(n) == n && fabs(n) < 1e15) {
        fprintf(stderr, "%.0f", n);
    } else {
        fprintf(stderr, "%g", n);
    }
}

void zaco_console_warn_bool(int64_t b) {
    fprintf(stderr, "%s", b ? "true" : "false");
}

/* ========== Timer Functions (setTimeout/setInterval) ========== */

typedef struct {
    void (*callback)(void*);
    void* context;
    int64_t delay_ms;
    int is_interval;
    volatile int cancelled;
} TimerContext;

#define MAX_TIMERS 1024
static TimerContext* timer_table[MAX_TIMERS];
static int64_t next_timer_id = 1;
static pthread_mutex_t timer_mutex = PTHREAD_MUTEX_INITIALIZER;

static void* timer_thread_fn(void* arg) {
    TimerContext* tc = (TimerContext*)arg;
    do {
        usleep((useconds_t)(tc->delay_ms * 1000));
        if (!tc->cancelled) {
            tc->callback(tc->context);
        }
    } while (tc->is_interval && !tc->cancelled);
    return NULL;
}

int64_t zaco_set_timeout(void (*callback)(void*), void* context, int64_t delay_ms) {
    pthread_mutex_lock(&timer_mutex);
    int64_t id = next_timer_id++;
    if (id >= MAX_TIMERS) {
        pthread_mutex_unlock(&timer_mutex);
        return -1;
    }
    TimerContext* tc = (TimerContext*)malloc(sizeof(TimerContext));
    tc->callback = callback;
    tc->context = context;
    tc->delay_ms = delay_ms;
    tc->is_interval = 0;
    tc->cancelled = 0;
    timer_table[id] = tc;
    pthread_mutex_unlock(&timer_mutex);

    pthread_t thread;
    pthread_create(&thread, NULL, timer_thread_fn, tc);
    pthread_detach(thread);
    return id;
}

int64_t zaco_set_interval(void (*callback)(void*), void* context, int64_t delay_ms) {
    pthread_mutex_lock(&timer_mutex);
    int64_t id = next_timer_id++;
    if (id >= MAX_TIMERS) {
        pthread_mutex_unlock(&timer_mutex);
        return -1;
    }
    TimerContext* tc = (TimerContext*)malloc(sizeof(TimerContext));
    tc->callback = callback;
    tc->context = context;
    tc->delay_ms = delay_ms;
    tc->is_interval = 1;
    tc->cancelled = 0;
    timer_table[id] = tc;
    pthread_mutex_unlock(&timer_mutex);

    pthread_t thread;
    pthread_create(&thread, NULL, timer_thread_fn, tc);
    pthread_detach(thread);
    return id;
}

void zaco_clear_timeout(int64_t timer_id) {
    pthread_mutex_lock(&timer_mutex);
    if (timer_id > 0 && timer_id < MAX_TIMERS && timer_table[timer_id]) {
        timer_table[timer_id]->cancelled = 1;
    }
    pthread_mutex_unlock(&timer_mutex);
}

void zaco_clear_interval(int64_t timer_id) {
    zaco_clear_timeout(timer_id);
}
