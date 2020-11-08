/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

#include "../include/sound.h"

static void sig_handler(int signum) {
    (void)signum;
    alive = false;
}

void print_and_exit(char *err) {
    fprintf(stderr, "%s: %s, %s\n", PREFIX_ERROR, err, strerror(errno));
    exit(EXIT_FAILURE);
}

void context_state_cb(pa_context *context, void *data) {
    pa_context_state_t  state;

    state = pa_context_get_state(context);
    if (state == PA_CONTEXT_READY) {
        ((t_data *)data)->connected = true;
    } else if (state == PA_CONTEXT_FAILED) {
        print_and_exit("context fails to connect");
    }
}

void sink_info_cb(pa_context *context, const pa_sink_info *info, int eol, void *data) {
    t_data      *d = data;

    (void)context;
    if (info != NULL && eol == 0) {
        d->sink_volume.mute = info->mute;
        d->sink_volume.volume = VOLUME(pa_cvolume_avg(&info->volume));
        (*d->sink_cb)(d->cb_context, d->sink_volume.volume, d->sink_volume.mute);
    }
}

void source_info_cb(pa_context *context, const pa_source_info *info, int eol, void *data) {
    t_data      *d = data;

    (void)context;
    if (info != NULL && eol == 0) {
        d->source_volume.mute = info->mute;
        d->source_volume.volume = VOLUME(pa_cvolume_avg(&info->volume));
        (*d->source_cb)(d->cb_context, d->source_volume.volume, d->source_volume.mute);
    }
}

void subscription_cb(pa_context *context, pa_subscription_event_type_t t, uint32_t idx, void *data) {
    t_data  *d;

    (void)context;
    (void)idx;
    d = data;
    if ((t & PA_SUBSCRIPTION_EVENT_FACILITY_MASK) == PA_SUBSCRIPTION_EVENT_SINK) {
        pa_context_get_sink_info_by_index(d->context, d->sink_index, sink_info_cb, data);
    } else if ((t & PA_SUBSCRIPTION_EVENT_FACILITY_MASK) == PA_SUBSCRIPTION_EVENT_SOURCE) {
        pa_context_get_source_info_by_index(d->context, d->source_index, source_info_cb, data);
    }
}

void abs_time_tick(t_timespec *start, t_timespec *end, uint32_t tick) {
    long int    sec;
    long int    nsec;

    sec = start->tv_sec + (long int) NSEC_TO_SECOND(tick);
    nsec = start->tv_nsec + (long int)tick;
    if (nsec > MAX_NSEC) {
        end->tv_sec = sec + 1;
        end->tv_nsec = nsec - MAX_NSEC;
    } else {
        end->tv_sec = sec;
        end->tv_nsec = nsec;
    }
}

void iterate(t_data *data) {
    t_timespec  tick;

    if (clock_gettime(CLOCK_REALTIME, &data->start) == -1) {
        print_and_exit("clock_gettime fails");
    }
    abs_time_tick(&data->start, &tick, data->tick);
    if (pa_mainloop_iterate(data->mainloop, 0, NULL) < 0) {
        print_and_exit("pa_mainloop_iterate fails");
    }
    clock_nanosleep(CLOCK_REALTIME, TIMER_ABSTIME, &tick, NULL);
}

int run(uint32_t tick, uint32_t sink_index, uint32_t source_index, void *cb_context, send_sink_cb sink_cb, send_source_cb source_cb) {
    pa_proplist     *proplist;
    t_data          data;
    t_sigaction     sa;

    memset(&sa, 0, sizeof(t_sigaction));
    sa.sa_handler = sig_handler;
    sigemptyset(&sa.sa_mask);
    if (sigaction(SIGINT, &sa, NULL) == -1) {
        print_and_exit("sigaction fails");
    }
    if (sigaction(SIGTERM, &sa, NULL) == -1) {
        print_and_exit("sigaction fails");
    }

    data.tick = tick;
    data.sink_index = sink_index;
    data.source_index = source_index;
    data.connected = false;
    data.cb_context = cb_context;
    data.sink_cb = sink_cb;
    data.source_cb = source_cb;
    data.mainloop = pa_mainloop_new();
    data.api = pa_mainloop_get_api(data.mainloop);
    proplist = pa_proplist_new();

    // context creation
    if (pa_proplist_sets(proplist, PA_PROP_APPLICATION_NAME, APPLICATION_NAME) != 0) {
        print_and_exit("pa_proplist_sets fails");
    }
    data.context = pa_context_new_with_proplist(data.api, APPLICATION_NAME, proplist);

    // context connection to the sever
    pa_context_set_state_callback(data.context, context_state_cb, &data);
    if (pa_context_connect(data.context, NULL, PA_CONTEXT_NOFAIL, NULL) < 0) {
        print_and_exit("pa_context_connect fails");
    }
    while(data.connected == false) {
        if (pa_mainloop_iterate(data.mainloop, 0, NULL) < 0) {
            print_and_exit("pa_mainloop_iterate fails");
        }
    }

    // initial introspection
    pa_context_get_sink_info_by_index(data.context, data.sink_index, sink_info_cb, &data);
    pa_context_get_source_info_by_index(data.context, data.source_index, source_info_cb, &data);

    // subscription introspection
    pa_context_subscribe(data.context, PA_SUBSCRIPTION_MASK_SINK | PA_SUBSCRIPTION_MASK_SOURCE, NULL, NULL);
    pa_context_set_subscribe_callback(data.context, subscription_cb, &data);

    // iterate main loop
    while(alive) {
        iterate(&data);
    }

    // close connection and free
    pa_context_disconnect(data.context);
    pa_mainloop_free(data.mainloop);
    printf("quit gracefully\n");
    return 0;
}
