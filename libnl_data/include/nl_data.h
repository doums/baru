// By Clément Dommerc

#ifndef NL_DATA_H_
#define NL_DATA_H_

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
#include <netlink/genl/genl.h>
#include <netlink/genl/ctrl.h>
#include <linux/nl80211.h>
#include <linux/if_ether.h>

#define WIRELESS_INTERFACE "wlp2s0"
#define NL80211 "nl80211"
#define WLAN_EID_SSID 0
#define WIRELESS_INFO_FLAG_HAS_ESSID (1 << 0)
#define WIRELESS_INFO_FLAG_HAS_QUALITY (1 << 1)
#define WIRELESS_ESSID_MAX_SIZE 16
#define NOISE_FLOOR_DBM (-90)
#define SIGNAL_MAX_DBM (-20)
#define WIRELESS_PREFIX_ERROR "Wireless module error"

typedef struct      s_wireless {
    unsigned int    flags;
    int             nl80211_id;
    unsigned int    if_index;
    uint8_t         bssid[ETH_ALEN];
    char            *essid;
    int             quality;
}                   t_wireless;

typedef struct  s_nl_data {
    char        *essid;
    int32_t     signal;
}               t_nl_data;

/* API */
t_nl_data       *get_data();

/* FUNCTIONS */
char    *v_strncpy(char *dest, const char *src, size_t n);
void    v_memset(void *ptr, uint8_t c, size_t size);
char    *alloc_buffer(size_t size);
void    *alloc_ptr(size_t size);

#endif /* !NL_DATA_H_ */
