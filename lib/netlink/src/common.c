/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

#include "../include/netlink.h"

void print_and_exit(char *err) {
    fprintf(stderr, "%s: %s\n", PREFIX_ERROR, err);
    exit(EXIT_FAILURE);
}

void *alloc_mem(size_t i) {
    void *p;

    if ((p = malloc(i)) == NULL) {
        print_and_exit("malloc failed");
    }
    memset(p, 0, i);
    return p;
}