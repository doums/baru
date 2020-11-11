/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

// By Clément Dommerc

#include "../include/netlink.h"

void print_and_exit(char *err) {
    fprintf(stderr, "%s: %s\n", PREFIX_ERROR, err);
    exit(EXIT_FAILURE);
}