#![cfg_attr(feature = "nightly", feature(doc_cfg))]

#[cfg(feature = "async")]
#[cfg_attr(feature = "nightly", doc(cfg(feature = "async")))]
pub mod async_drain;
pub mod console;
