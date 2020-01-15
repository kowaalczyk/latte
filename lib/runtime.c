#define _GNU_SOURCE

#include "stdio.h"
#include "stdlib.h"
#include "string.h"

void __func__error();
void __func__printString(const char*);

/**
 * initialize empty string with a given size
 * @param size - desired number of characters
 * @return string
 */
char* __builtin_method__str__init__(int size) {
    // allocate memory (+1) to contain 0 at the end
    char* s = (char *)malloc(size + 1);
    if (s == NULL) __func__error();

    // fill memory with zeros
    memset(s, (char)0, size+1);
    return s;
}

/**
 * concatenate strings (operator +)
 * @param left - string
 * @param right - string
 * @return - concatenated string
 */
char* __builtin_method__str__concat__(char* left, char* right) {
    // calculate length (without null character)
    size_t left_len = strlen(left);
    size_t right_len = strlen(right);
    size_t combined_len = left_len + right_len;

    char* combined = __builtin_method__str__init__(combined_len);
    memcpy(combined, left, left_len);
    memcpy(combined + left_len, right, right_len);

    return combined;
}

void* __builtin_method__array__init__(int size) {
    void* arr = malloc(size);
    if (arr == NULL) __func__error();

    // fill memory with zeros
    memset(arr, (char)0, size+1);
    return arr;
}

/// latte standard library
void __func__printInt(int i) {
    printf("%d\n", i);
}

/// latte standard library
void __func__printString(const char* str) {
    printf("%s\n", str);
}

/// latte standard library
int __func__readInt() {
    int i;
    scanf("%d\n", &i);
    return i;
}

/// latte standard library
char* __func__readString() {
    // getline will allocate memory if c = NULL and n = 0
    char* c = NULL;
    size_t n = 0;
    ssize_t r = getline(&c, &n, stdin);
    if (r < 0) __func__error();

    // getline returns a '\n' unless the EOF was reached before it, we need to clear that from the string
    if (c[r-1] == '\n') c[r-1] = (char)0;
    return c;
}

/// latte standard library
void __func__error() {
    __func__printString("runtime error");
    exit(1);
}
