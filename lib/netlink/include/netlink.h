/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

// Wireless code written by Clément Dommerc

#ifndef NETLINK_H
#define NETLINK_H

#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <sys/types.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <string.h>
#include <time.h>
#include <stdarg.h>
#include <dirent.h>
#include <regex.h>
#include <stdbool.h>
#include <limits.h>
#include <errno.h>
#include <net/if.h>
#include <netlink/netlink.h>
#include <sys/socket.h>
#include <netlink/genl/genl.h>
#include <netlink/genl/ctrl.h>
#include <linux/nl80211.h>
#include <linux/if_ether.h>
#include <netlink/socket.h>
#include <netlink/cache.h>
#include <netlink/route/link.h>
#include <netlink/route/addr.h>

#define NL80211 "nl80211"
#define WLAN_EID_SSID 0
#define WIRELESS_INFO_FLAG_HAS_ESSID (1 << 0)
#define WIRELESS_INFO_FLAG_HAS_QUALITY (1 << 1)
#define WIRELESS_ESSID_MAX_SIZE 16
#define NOISE_FLOOR_DBM (-90)
#define SIGNAL_MAX_DBM (-20)
#define PREFIX_ERROR "libnetlink error"
#define BUF_SIZE 1024

typedef struct      s_wireless {
    unsigned int    flags;
    int             nl80211_id;
    unsigned int    if_index;
    char            *if_name;
    uint8_t         bssid[ETH_ALEN];
    char            *essid;
    int             quality;
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

t_wireless_data get_wireless_data(char *interface);
t_wired_data    get_wired_data(char *interface);

/* HELPERS */
char    *alloc_buffer(size_t size);

#endif // NETLINK_H
