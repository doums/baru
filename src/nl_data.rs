// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};

#[derive(Debug)]
#[repr(C)]
pub struct NlWirelessData {
    essid: *const c_char,
    signal: c_int,
}

#[derive(Debug)]
#[repr(C)]
pub struct NlWiredData {
    is_carrying: bool,
    is_operational: bool,
    has_ip: bool,
}

pub enum WirelessState {
    Disconnected,
    Connected(WirelessData),
}

pub enum WiredState {
    Disconnected,
    NotPlugged,
    Connected,
}

#[derive(Debug)]
pub struct WirelessData {
    pub essid: Option<String>,
    pub signal: Option<i32>,
}

#[link(name = "nl_data", kind = "static")]
extern "C" {
    pub fn get_wireless_data(interface: *const c_char) -> NlWirelessData;
    pub fn get_wired_data(interface: *const c_char) -> NlWiredData;
}

pub fn wireless_data(interface: &str) -> WirelessState {
    let c_interface = CString::new(interface).expect("CString::new failed");
    unsafe {
        let nl_data = get_wireless_data(c_interface.as_ptr());
        let signal_ptr = nl_data.signal;
        let essid_ptr = nl_data.essid;
        let signal = if signal_ptr == -1 {
            None
        } else {
            Some(signal_ptr)
        };
        let essid = if essid_ptr.is_null() {
            None
        } else {
            Some(CStr::from_ptr(essid_ptr).to_string_lossy().into_owned())
        };
        if signal.is_none() && essid.is_none() {
            WirelessState::Disconnected
        } else {
            WirelessState::Connected(WirelessData { signal, essid })
        }
    }
}

pub fn wired_data(interface: &str) -> WiredState {
    let c_interface = CString::new(interface).expect("CString::new failed");
    unsafe {
        let data = get_wired_data(c_interface.as_ptr());
        let is_op = data.is_operational;
        let is_carrying = data.is_carrying;
        let has_ip = data.has_ip;
        if is_carrying && is_op && has_ip {
            WiredState::Connected
        } else if is_carrying {
            WiredState::Disconnected
        } else {
            WiredState::NotPlugged
        }
    }
}
