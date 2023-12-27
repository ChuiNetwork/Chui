/// This is the build script for the `bloom` crate.
/// It checks the Rust compiler version and sets the appropriate configuration flags based on the channel.
/// If the channel is Stable or Beta, it sets the `RUSTC_WITHOUT_SPECIALIZATION` flag.
/// If the channel is Nightly or Dev, it sets the `RUSTC_WITH_SPECIALIZATION` flag.
/// Additionally, if the channel is Dev, it also sets the `RUSTC_NEEDS_PROC_MACRO_HYGIENE` flag.
extern crate rustc_version;
use rustc_version::{version_meta, Channel};

fn main() {
    // Copied and adapted from
    // https://github.com/Kimundi/rustc-version-rs/blob/1d692a965f4e48a8cb72e82cda953107c0d22f47/README.md#example
    // Licensed under Apache-2.0 + MIT
    match version_meta().unwrap().channel {
        Channel::Stable => {
            println!("cargo:rustc-cfg=RUSTC_WITHOUT_SPECIALIZATION");
        }
        Channel::Beta => {
            println!("cargo:rustc-cfg=RUSTC_WITHOUT_SPECIALIZATION");
        }
        Channel::Nightly => {
            println!("cargo:rustc-cfg=RUSTC_WITH_SPECIALIZATION");
        }
        Channel::Dev => {
            println!("cargo:rustc-cfg=RUSTC_WITH_SPECIALIZATION");
            // See https://github.com/solana-labs/solana/issues/11055
            // We may be running the custom `rust-bpf-builder` toolchain,
            // which currently needs `#![feature(proc_macro_hygiene)]` to
            // be applied.
            println!("cargo:rustc-cfg=RUSTC_NEEDS_PROC_MACRO_HYGIENE");
        }
    }
}
