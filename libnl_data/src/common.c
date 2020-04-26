// By Clément Dommerc

#include "../include/nl_data.h"

void    v_memset(void *ptr, uint8_t c, size_t size) {
    size_t  i = -1;

    while (++i < size) {
        ((uint8_t *)ptr)[i] = c;
    }
}

char    *v_strncpy(char *dest, const char *src, size_t n) {
    size_t  i = -1;

    if (!dest || !src) {
        return NULL;
    }
    while (++i < n && src[i]) {
        dest[i] = src[i];
    }
    while (i < n) {
        dest[i++] = 0;
    }
    return dest;
}

char    *alloc_buffer(size_t size) {
    char    *buffer;

    if ((buffer = malloc(sizeof(char) * size)) == NULL) {
        printf("Call to 'malloc()' failed: %s\n", strerror(errno));
        exit(EXIT_FAILURE);
    }
    v_memset(buffer, 0, size);
    return buffer;
}

void    *alloc_ptr(size_t size) {
    void    *ptr;

    if ((ptr = malloc(size)) == NULL) {
        printf("Call to 'malloc()' failed: %s\n", strerror(errno));
        exit(EXIT_FAILURE);
    }
    return ptr;
}
