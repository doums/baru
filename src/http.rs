// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::time::Duration;

use once_cell::sync::Lazy;
use reqwest::blocking::Client;
use tracing::error;

pub static HTTP_CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .inspect_err(|e| {
            error!("Failed to create HTTP client: {:?}", e);
        })
        .unwrap()
});
