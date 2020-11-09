/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

// By Clément Dommerc

#include "../include/netlink.h"

char    *alloc_buffer(size_t size) {
    char    *buffer;

    if ((buffer = malloc(sizeof(char) * size)) == NULL) {
        printf("Call to 'malloc()' failed: %s\n", strerror(errno));
        exit(EXIT_FAILURE);
    }
    memset(buffer, 0, size);
    return buffer;
}