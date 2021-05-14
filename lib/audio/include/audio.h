/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

#ifndef AUDIO_H
#define AUDIO_H

#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>
#include <errno.h>
#include <string.h>
#include <pulse/context.h>
#include <pulse/proplist.h>
#include <pulse/mainloop.h>
#include <pulse/def.h>
#include <pulse/introspect.h>
#include <pulse/subscribe.h>
#include <time.h>

#define PREFIX_ERROR "libaudio"
#define APPLICATION_NAME "baru"
#define NSEC_TO_SECOND(N) N / (long)1e9
#define MAX_NSEC 999999999
/*
 * get humanized volume from a pa_volume_t (aka uint32_t)
 * N should result from pa_cvolume_avg(pa_cvolume)
 * based on pulseaudio source code, see https://gitlab.freedesktop.org/pulseaudio/pulseaudio/-/blob/master/src/pulse/volume.c#L336
*/
#define VOLUME(N) (uint32_t)(((uint64_t)N * 100 + (uint64_t)PA_VOLUME_NORM / 2) / (uint64_t)PA_VOLUME_NORM)

static bool alive = true;

typedef struct timespec
        t_timespec;

typedef struct volume {
    uint32_t volume;
    bool mute;
} t_volume;

typedef void(*send_cb)(void *, uint32_t, bool);

typedef struct data {
    const char *name;
    bool use_default;
    t_volume volume;
    send_cb cb;
    pa_operation *op;
} t_data;

typedef struct main {
    uint32_t tick;
    bool connected;
    pa_context *context;
    pa_mainloop *mainloop;
    pa_mainloop_api *api;
    void *cb_context;
    t_timespec start;
    pa_operation *server_op;
    t_data *sink;
    t_data *source;
} t_main;

void run(uint32_t tick,
         const char *sink_name,
         const char *source_name,
         void *cb_context,
         send_cb,
         send_cb);

#endif //AUDIO_H
