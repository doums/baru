// By Clément Dommerc

#include "../include/nl_data.h"

static uint32_t     nl80211_xbm_to_percent(int32_t xbm, int32_t divisor) {
    xbm /= divisor;
    if (xbm < NOISE_FLOOR_DBM) {
        xbm = NOISE_FLOOR_DBM;
    }
    if (xbm > SIGNAL_MAX_DBM) {
        xbm = SIGNAL_MAX_DBM;
    }
    return 100 - 70 * (((float)SIGNAL_MAX_DBM - (float)xbm) / ((float)SIGNAL_MAX_DBM - (float)NOISE_FLOOR_DBM));
}

static void     find_ssid(uint8_t *ies, uint32_t ies_len, uint8_t **ssid, uint32_t *ssid_len) {
    *ssid = NULL;
    *ssid_len = 0;
    while (ies_len > 2 && ies[0] != WLAN_EID_SSID) {
        ies_len -= ies[1] + 2;
        ies += ies[1] + 2;
    }
    if (ies_len < 2 || ies_len < (uint32_t)(2 + ies[1])) {
        return;
    }
    *ssid_len = ies[1];
    *ssid = ies + 2;
}

static int      station_callback(struct nl_msg *msg, void *data) {
    int                 sig_qlty;
    t_wireless          *wireless = data;
    struct nlattr       *tb[NL80211_ATTR_MAX + 1];
    struct genlmsghdr   *gnlh = nlmsg_data(nlmsg_hdr(msg));
    int                 attrs_length;
    struct nlattr       *attrs_head;
    struct nlattr       *s_info[NL80211_STA_INFO_MAX + 1];
    static struct       nla_policy stats_policy[NL80211_STA_INFO_MAX + 1];

    attrs_head = genlmsg_attrdata(gnlh, 0);
    attrs_length = genlmsg_attrlen(gnlh, 0);
    if (nla_parse(tb, NL80211_ATTR_MAX, attrs_head, attrs_length, NULL) < 0) {
        return NL_SKIP;
    }
    if (tb[NL80211_ATTR_STA_INFO] == NULL) {
        return NL_SKIP;
    }
    if (nla_parse_nested(s_info, NL80211_STA_INFO_MAX, tb[NL80211_ATTR_STA_INFO], stats_policy)) {
        return NL_SKIP;
    }
    if (s_info[NL80211_STA_INFO_SIGNAL] != NULL) {
        wireless->flags |= WIRELESS_INFO_FLAG_HAS_QUALITY;
        sig_qlty = (int8_t)nla_get_u8(s_info[NL80211_STA_INFO_SIGNAL]);
        wireless->quality = nl80211_xbm_to_percent(sig_qlty, 1);
    }
    return NL_SKIP;
}

static int      scan_callback(struct nl_msg *msg, void *data) {
    t_wireless          *wireless = data;
    uint8_t             *ssid;
    uint32_t            ssid_len;
    uint32_t            status;
    uint8_t             *bss_ies;
    uint32_t            bss_ies_length;
    int                 attrs_length;
    struct genlmsghdr   *gnlh;
    struct nlattr       *attrs_head;
    struct nlattr       *tb[NL80211_ATTR_MAX + 1];
    struct nlattr       *bss[NL80211_BSS_MAX + 1];
    struct nla_policy   bss_policy[NL80211_BSS_MAX + 1] = {
            [NL80211_BSS_BSSID] = {.type = NLA_UNSPEC},
            [NL80211_BSS_INFORMATION_ELEMENTS] = {.type = NLA_UNSPEC},
            [NL80211_BSS_STATUS] = {.type = NLA_U32},
    };

    gnlh = nlmsg_data(nlmsg_hdr(msg));
    attrs_head = genlmsg_attrdata(gnlh, 0);
    attrs_length = genlmsg_attrlen(gnlh, 0);
    if (nla_parse(tb, NL80211_ATTR_MAX, attrs_head, attrs_length, NULL) < 0) {
        return NL_SKIP;
    }
    if (tb[NL80211_ATTR_BSS] == NULL) {
        return NL_SKIP;
    }
    if (nla_parse_nested(bss, NL80211_BSS_MAX, tb[NL80211_ATTR_BSS], bss_policy)) {
        return NL_SKIP;
    }
    if (bss[NL80211_BSS_STATUS] == NULL) {
        return NL_SKIP;
    }
    status = nla_get_u32(bss[NL80211_BSS_STATUS]);
    if (status != NL80211_BSS_STATUS_ASSOCIATED && status != NL80211_BSS_STATUS_IBSS_JOINED) {
        return NL_SKIP;
    }
    if (bss[NL80211_BSS_BSSID] == NULL) {
        return NL_SKIP;
    }

    memcpy(wireless->bssid, nla_data(bss[NL80211_BSS_BSSID]), ETH_ALEN);
    if (bss[NL80211_BSS_INFORMATION_ELEMENTS]) {
        bss_ies = nla_data(bss[NL80211_BSS_INFORMATION_ELEMENTS]);
        bss_ies_length = nla_len(bss[NL80211_BSS_INFORMATION_ELEMENTS]);
        find_ssid(bss_ies, bss_ies_length, &ssid, &ssid_len);
        if (ssid && ssid_len) {
            wireless->flags |= WIRELESS_INFO_FLAG_HAS_ESSID;
            if (ssid_len > WIRELESS_ESSID_MAX_SIZE) {
                wireless->essid = alloc_buffer(WIRELESS_ESSID_MAX_SIZE + 1);
                v_strncpy(wireless->essid, (char *)ssid, WIRELESS_ESSID_MAX_SIZE);
                wireless->essid[WIRELESS_ESSID_MAX_SIZE - 1] = '.';
            } else {
                wireless->essid = alloc_buffer(ssid_len + 1);
                v_strncpy(wireless->essid, (char *)ssid, ssid_len);
            }
        }
    }
    return NL_SKIP;
}

static int      send_for_station(t_wireless *wireless, struct nl_sock *socket) {
    struct nl_msg       *msg;

    if (nl_socket_modify_cb(socket, NL_CB_VALID, NL_CB_CUSTOM, station_callback, wireless) < 0) {
        printf("%s: unable to modify station callback\n", WIRELESS_PREFIX_ERROR);
        return -NLE_RANGE;
    }
    if ((msg = nlmsg_alloc()) == NULL) {
        printf("%s: unable to alloc memory for Netlink message\n", WIRELESS_PREFIX_ERROR);
        return -1;
    }
    if (!genlmsg_put(msg, NL_AUTO_PORT, NL_AUTO_SEQ, wireless->nl80211_id, 0, NLM_F_DUMP, NL80211_CMD_GET_STATION, 0)) {
        printf("%s: unable to add header to Netlink message\n", WIRELESS_PREFIX_ERROR);
        goto error;
    }
    if (nla_put_u32(msg, NL80211_ATTR_IFINDEX, wireless->if_index) < 0) {
        printf("%s: unable to add attribute to Netlink message\n", WIRELESS_PREFIX_ERROR);
        goto error;
    }
    if (nla_put(msg, NL80211_ATTR_MAC, 6, wireless->bssid) < 0) {
        printf("%s: unable to add attribute to Netlink message\n", WIRELESS_PREFIX_ERROR);
        goto error;
    }
    if (nl_send_sync(socket, msg) < 0) {
        printf("%s: unable to send Netlink message\n", WIRELESS_PREFIX_ERROR);
        goto error;
    }
    return 0;

    error:
    nlmsg_free(msg);
    return -1;
}

static int      send_for_scan(t_wireless *wireless, struct nl_sock *socket) {
    struct nl_msg       *msg;

    if (nl_socket_modify_cb(socket, NL_CB_VALID, NL_CB_CUSTOM, scan_callback, wireless) < 0) {
        printf("%s: unable to modify scan callback\n", WIRELESS_PREFIX_ERROR);
        return -NLE_RANGE;
    }
    wireless->nl80211_id = genl_ctrl_resolve(socket, NL80211);
    if (wireless->nl80211_id < 0) {
        printf("%s: unable to resolve Netlink family\n", WIRELESS_PREFIX_ERROR);
        return -NLE_OBJ_NOTFOUND;
    }
    wireless->if_index = if_nametoindex(wireless->if_name);
    if (wireless->if_index == 0) {
        printf("%s: %s\n", WIRELESS_PREFIX_ERROR, strerror(errno));
        return -1;
    }
    if ((msg = nlmsg_alloc()) == NULL) {
        printf("%s: unable to alloc memory for Netlink message\n", WIRELESS_PREFIX_ERROR);
        return -1;
    }
    if (!genlmsg_put(msg, NL_AUTO_PORT, NL_AUTO_SEQ, wireless->nl80211_id, 0, NLM_F_DUMP, NL80211_CMD_GET_SCAN, 0)) {
        printf("%s: unable to add header to Netlink message\n", WIRELESS_PREFIX_ERROR);
        goto error;
    }
    if (nla_put_u32(msg, NL80211_ATTR_IFINDEX, wireless->if_index) < 0) {
        printf("%s: unable to add attribute to Netlink message\n", WIRELESS_PREFIX_ERROR);
        goto error;
    }
    if (nl_send_sync(socket, msg) < 0) {
        printf("%s: unable to send Netlink message\n", WIRELESS_PREFIX_ERROR);
        goto error;
    }
    return 0;

    error:
    nlmsg_free(msg);
    return -1;
}

t_nl_data        *get_data(char *interface) {
    t_wireless          wireless;
    t_nl_data           *nl_data;
    struct nl_sock      *socket;

    nl_data = alloc_ptr(sizeof(t_nl_data));
    nl_data->essid = NULL;
    nl_data->signal = -1;
    socket = nl_socket_alloc();
    if (genl_connect(socket) != 0) {
        printf("%s: unable to alloc memory for Netlink socket\n", WIRELESS_PREFIX_ERROR);
        exit(EXIT_FAILURE);
    }
    v_memset(&wireless, 0, sizeof(t_wireless));
    wireless.if_name = interface;
    if (send_for_scan(&wireless, socket) < 0 || send_for_station(&wireless, socket) < 0) {
        nl_socket_free(socket);
        exit(EXIT_FAILURE);
    }
    if (wireless.flags & WIRELESS_INFO_FLAG_HAS_QUALITY) {
        nl_data->signal = wireless.quality;
    }
    if (wireless.flags & WIRELESS_INFO_FLAG_HAS_ESSID) {
       nl_data->essid = wireless.essid;
    }

    nl_socket_free(socket);
    return nl_data;
}
