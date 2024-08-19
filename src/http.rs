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
