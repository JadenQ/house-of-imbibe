//! Shared library for the `house-of-imbibe` package.
//!
//! Currently only exposes `pixelart`, a thin wrapper around the
//! PixelLab REST API and a MiniMax-M3 vision call (for the
//! `image → vision → text → PixelLab` pipeline validated in spike-0,
//! see `.scratch/issues/0009`).
//!
//! Both `src/bin/image2pixel.rs` (CLI demo) and `src/bin/pixelart_server.rs`
//! (HTTP test server) use this module. NO business code outside the package
//! should reach into `reqwest` directly — keep provider swaps possible.

pub mod pixelart;