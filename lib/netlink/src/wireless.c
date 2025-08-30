/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

// By Clément Dommerc

#include <netlink/genl/genl.h>
#include <netlink/genl/ctrl.h>
#include <net/if.h>
#include <errno.h>

#include "../include/netlink.h"

// Based on NetworkManager/src/platform/wifi/wifi-utils-nl80211.c
static uint32_t nl80211_xbm_to_percent(int32_t xbm) {
    xbm = CLAMP(xbm, NOISE_FLOOR_DBM, SIGNAL_MAX_DBM);
    return 100 - 70 * (((float) SIGNAL_MAX_DBM - (float) xbm) / ((float) SIGNAL_MAX_DBM - (float) NOISE_FLOOR_DBM));
}

// Based on NetworkManager/src/platform/wifi/wifi-utils-nl80211.c
static void find_ssid(uint8_t *ies, uint32_t ies_len, uint8_t **ssid, uint32_t *ssid_len) {
    while (ies_len > 2 && ies[0] != EID_SSID) {
        ies_len -= ies[1] + 2;
        ies += ies[1] + 2;
    }
    if (ies_len < 2 || ies_len < (uint32_t) (2 + ies[1])) {
        return;
    }
    *ssid_len = ies[1];
    *ssid = ies + 2;
}

void resolve_essid(t_wireless *wireless, const struct nlattr *attr) {
    uint32_t bss_ies_len = nla_len(attr);
    uint8_t *bss_ies = nla_data(attr);
    uint8_t *ssid = nullptr;
    uint32_t ssid_len = 0;

    find_ssid(bss_ies, bss_ies_len, &ssid, &ssid_len);
    if (ssid_len > ESSID_MAX_SIZE) {
        ssid_len = ESSID_MAX_SIZE;
    }
    if (ssid) {
        wireless->essid = alloc_mem(sizeof(char) * (ssid_len + 1));
        wireless->essid_found = true;
        strncpy(wireless->essid, (char *) ssid, ssid_len);
    }
}

static int station_cb(struct nl_msg *msg, void *data) {
    t_wireless *wireless = data;
    struct nlattr *tb[NL80211_ATTR_MAX + 1];
    struct genlmsghdr *gnlh = nlmsg_data(nlmsg_hdr(msg));
    struct nlattr *attr = genlmsg_attrdata(gnlh, 0);
    int attrlen = genlmsg_attrlen(gnlh, 0);
    struct nlattr *s_info[NL80211_STA_INFO_MAX + 1];
    static struct nla_policy stats_policy[NL80211_STA_INFO_MAX + 1];

    if (nla_parse(tb, NL80211_ATTR_MAX, attr, attrlen, nullptr) < 0) {
        return NL_SKIP;
    }
    if (tb[NL80211_ATTR_STA_INFO] == nullptr) {
        return NL_SKIP;
    }
    if (nla_parse_nested(s_info, NL80211_STA_INFO_MAX, tb[NL80211_ATTR_STA_INFO], stats_policy) < 0) {
        return NL_SKIP;
    }
    if (s_info[NL80211_STA_INFO_SIGNAL] != nullptr) {
        wireless->signal_found = true;
        wireless->signal = nl80211_xbm_to_percent((int8_t) nla_get_u8(s_info[NL80211_STA_INFO_SIGNAL]));
    }
    return NL_SKIP;
}

static int scan_cb(struct nl_msg *msg, void *data) {
    t_wireless *wireless = data;
    uint32_t status;
    struct genlmsghdr *gnlh = nlmsg_data(nlmsg_hdr(msg));
    struct nlattr *attr = genlmsg_attrdata(gnlh, 0);
    int attrlen = genlmsg_attrlen(gnlh, 0);
    struct nlattr *tb[NL80211_ATTR_MAX + 1];
    struct nlattr *bss[NL80211_BSS_MAX + 1];
    struct nla_policy bss_policy[NL80211_BSS_MAX + 1] = {
            [NL80211_BSS_BSSID] = {.type = NLA_UNSPEC},
            [NL80211_BSS_INFORMATION_ELEMENTS] = {.type = NLA_UNSPEC},
            [NL80211_BSS_STATUS] = {.type = NLA_U32},
    };

    if (nla_parse(tb, NL80211_ATTR_MAX, attr, attrlen, nullptr) < 0) {
        return NL_SKIP;
    }
    if (tb[NL80211_ATTR_BSS] == nullptr) {
        return NL_SKIP;
    }
    if (nla_parse_nested(bss, NL80211_BSS_MAX, tb[NL80211_ATTR_BSS], bss_policy) < 0) {
        return NL_SKIP;
    }
    if (bss[NL80211_BSS_STATUS] == nullptr) {
        return NL_SKIP;
    }
    status = nla_get_u32(bss[NL80211_BSS_STATUS]);
    if (status != NL80211_BSS_STATUS_ASSOCIATED && status != NL80211_BSS_STATUS_IBSS_JOINED) {
        return NL_SKIP;
    }
    if (bss[NL80211_BSS_BSSID] == nullptr) {
        return NL_SKIP;
    }
    memcpy(wireless->bssid, nla_data(bss[NL80211_BSS_BSSID]), ETH_ALEN);
    if (bss[NL80211_BSS_INFORMATION_ELEMENTS]) {
        resolve_essid(wireless, bss[NL80211_BSS_INFORMATION_ELEMENTS]);
    }
    return NL_SKIP;
}

static int send_for_station(t_wireless *wireless) {
    struct nl_msg *msg = nullptr;
    int err;

    if ((err = nl_socket_modify_cb(wireless->socket, NL_CB_VALID, NL_CB_CUSTOM, station_cb, wireless)) < 0) {
        printf("%s, station nl_socket_modify_cb failed, %s\n", PREFIX_ERROR, nl_geterror(err));
        return -1;
    }
    if ((msg = nlmsg_alloc()) == nullptr) {
        printf("%s, station nlmsg_alloc failed\n", PREFIX_ERROR);
        return -1;
    }
    if (genlmsg_put(msg, NL_AUTO_PORT, NL_AUTO_SEQ, wireless->nl80211_id, 0, NLM_F_DUMP, NL80211_CMD_GET_STATION, 0) ==
        nullptr) {
        printf("%s, station genlmsg_put failed\n", PREFIX_ERROR);
        nlmsg_free(msg);
        return -1;
    }
    if ((err = nla_put_u32(msg, NL80211_ATTR_IFINDEX, wireless->if_index)) < 0) {
        printf("%s, station nla_put_u32 failed, %s\n", PREFIX_ERROR, nl_geterror(err));

        nlmsg_free(msg);
        return -1;
    }
    if ((err = nla_put(msg, NL80211_ATTR_MAC, 6, wireless->bssid)) < 0) {
        printf("%s, station nla_put failed, %s\n", PREFIX_ERROR, nl_geterror(err));
        nlmsg_free(msg);
        return -1;
    }
    if ((err = nl_send_sync(wireless->socket, msg)) < 0) {
        printf("%s, station nl_send_sync failed, %s\n", PREFIX_ERROR, nl_geterror(err));
        return -1;
    }
    return 0;
}

static int send_for_scan(t_wireless *wireless) {
    struct nl_msg *msg;
    int err;

    if ((err = nl_socket_modify_cb(wireless->socket, NL_CB_VALID, NL_CB_CUSTOM, scan_cb, wireless)) < 0) {
        printf("%s, scan nl_socket_modify_cb failed, %s\n", PREFIX_ERROR, nl_geterror(err));
        return -1;
    }
    if ((msg = nlmsg_alloc()) == nullptr) {
        printf("%s, scan nlmsg_alloc failed\n", PREFIX_ERROR);
        return -1;
    }
    if (genlmsg_put(msg, NL_AUTO_PORT, NL_AUTO_SEQ, wireless->nl80211_id, 0, NLM_F_DUMP, NL80211_CMD_GET_SCAN, 0) ==
        nullptr) {
        printf("%s, scan genlmsg_put failed\n", PREFIX_ERROR);
        nlmsg_free(msg);
        return -1;
    }
    if ((err = nla_put_u32(msg, NL80211_ATTR_IFINDEX, wireless->if_index)) < 0) {
        printf("%s, scan nla_put_u32 failed, %s\n", PREFIX_ERROR, nl_geterror(err));
        nlmsg_free(msg);
        return -1;
    }
    if ((err = nl_send_sync(wireless->socket, msg)) < 0) {
        printf("%s, scan nl_send_sync failed, %s\n", PREFIX_ERROR, nl_geterror(err));
        return -1;
    }
    return 0;
}

t_wireless_data *get_wireless_data(const char *interface) {
    t_wireless wireless;
    t_wireless_data *data;

    wireless.essid_found = false;
    wireless.signal_found = false;
    memset(&wireless, 0, sizeof(t_wireless));
    wireless.if_name = interface;
    wireless.socket = nl_socket_alloc();
    if (wireless.socket == nullptr) {
        print_and_exit("nl_socket_alloc failed\n");
    }
    if (genl_connect(wireless.socket) != 0) {
        nl_socket_free(wireless.socket);
        fprintf(stderr, "%s: genl_connect failed\n", PREFIX_ERROR);
        return nullptr;
    }
    if ((wireless.nl80211_id = genl_ctrl_resolve(wireless.socket, NL80211)) < 0) {
        fprintf(stderr, "%s: genl_ctrl_resolve failed; %s\n", PREFIX_ERROR, nl_geterror(wireless.nl80211_id));
        nl_socket_free(wireless.socket);
        return nullptr;
    }
    if ((wireless.if_index = if_nametoindex(wireless.if_name)) == 0) {
        fprintf(stderr, "%s: if_nametoindex failed, %s\n", PREFIX_ERROR, strerror(errno));
        nl_socket_free(wireless.socket);
        return nullptr;
    }
    if (send_for_scan(&wireless) < 0 || send_for_station(&wireless) < 0) {
        nl_socket_free(wireless.socket);
        return nullptr;
    }
    data = alloc_mem(sizeof(t_wireless_data));
    data->signal = -1;
    if (wireless.signal_found == true) {
        data->signal = wireless.signal;
    }
    if (wireless.essid_found == true) {
        data->essid = wireless.essid;
    }
    nl_socket_free(wireless.socket);
    return data;
}

void free_data(void *data) {
    if (data != nullptr) {
        free(data);
    }
}
