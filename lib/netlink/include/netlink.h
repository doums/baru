/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

// Wireless code written by Clément Dommerc

#ifndef NETLINK_H
#define NETLINK_H

#include <netlink/netlink.h>
#include <linux/nl80211.h>
#include <linux/if_ether.h>

#define NL80211 "nl80211"
#define EID_SSID 0
#define NOISE_FLOOR_DBM (-90)
#define SIGNAL_MAX_DBM (-20)
#define PREFIX_ERROR "libnetlink"
#define BUF_SIZE 1024
#define ESSID_MAX_SIZE 1024
#define CLAMP(x, l, h) x < l ? l : \
                        x > h ? h : x

typedef struct      s_wireless {
    bool            essid_found;
    bool            signal_found;
    int             nl80211_id;
    unsigned int    if_index;
    const char      *if_name;
    uint8_t         bssid[ETH_ALEN];
    char            *essid;
    int             signal;
    struct nl_sock  *socket;
}                   t_wireless;

/* API */
typedef struct  s_wireless_data {
    char        *essid;
    int32_t     signal;
}               t_wireless_data;

typedef struct  s_wired_data {
    bool        is_carrying;
    bool        is_operational;
    bool        has_ip;
}               t_wired_data;

t_wireless_data *get_wireless_data(const char *interface);
t_wired_data    *get_wired_data(const char *interface);
void            free_data(void *data);

/* HELPER */
void            print_and_exit(char *err);
void            *alloc_mem(size_t size);

#endif // NETLINK_H
