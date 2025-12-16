//! Mycelial WASM - Browser bindings for the mycelial network
//!
//! This crate provides WebAssembly bindings for browser-based clients.

use wasm_bindgen::prelude::*;

/// Initialize the WASM module
#[wasm_bindgen(start)]
pub fn init() {
    // Set up console error panic hook for better error messages
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

/// Example function exposed to JavaScript
#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! Welcome to the Mycelial Network.", name)
}

/// Peer connection state for browser clients
#[wasm_bindgen]
pub struct BrowserPeer {
    // TODO: Add WebSocket connection state
}

#[wasm_bindgen]
impl BrowserPeer {
    /// Create a new browser peer
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {}
    }

    /// Connect to a relay server
    pub async fn connect(&mut self, _relay_url: &str) -> Result<(), JsValue> {
        // TODO: Implement WebSocket connection
        Ok(())
    }
}

impl Default for BrowserPeer {
    fn default() -> Self {
        Self::new()
    }
}
