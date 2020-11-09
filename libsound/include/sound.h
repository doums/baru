/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

#ifndef SOUND_H
#define SOUND_H

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

#define PREFIX_ERROR "libsound"
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

typedef struct          volume {
    uint32_t            volume;
    bool                mute;
}                       t_volume;

typedef void(* send_sink_cb)(void *, uint32_t, bool);
typedef void(* send_source_cb)(void *, uint32_t, bool);

typedef struct          data {
    uint32_t            tick;
    uint32_t            sink_index;
    uint32_t            source_index;
    bool                connected;
    pa_context          *context;
    pa_mainloop         *mainloop;
    pa_mainloop_api     *api;
    t_volume            sink_volume;
    t_volume            source_volume;
    void                *cb_context;
    send_sink_cb        sink_cb;
    send_source_cb      source_cb;
    t_timespec          start;
    pa_operation        *sink_op;
    pa_operation        *source_op;
}                       t_data;

int run(uint32_t tick, uint32_t sink_index, uint32_t source_index, void *, send_sink_cb, send_source_cb);

#endif //SOUND_H
