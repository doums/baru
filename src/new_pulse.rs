// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::pulse::CallbackContext;

pub type Callback = extern "C" fn(*const CallbackContext, u32, bool);

#[link(name = "sound", kind = "static")]
extern "C" {
    pub fn run(
        tick: u32,
        sink_index: u32,
        source_index: u32,
        cb_context: *const CallbackContext,
        sink_cb: Callback,
        source_cb: Callback,
    ) -> u32;
}

pub fn pulse_run(
    tick: u32,
    sink_index: u32,
    source_index: u32,
    callback_context: &CallbackContext,
    sink_cb: Callback,
    source_cb: Callback,
) {
    let context_ptr: *const CallbackContext = callback_context;
    unsafe {
        run(
            tick,
            sink_index,
            source_index,
            context_ptr,
            sink_cb,
            source_cb,
        );
    }
}
