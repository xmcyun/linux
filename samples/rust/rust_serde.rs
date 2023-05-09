// SPDX-License-Identifier: GPL-2.0

//! Rust `serde` sample.
//!
//! It uses a data format from the `kernel` crate, as well as defining
//! one here ("local"). Then it uses both on a type that uses `serve_derive`.

use kernel::prelude::*;
use serde::Serialize;
use serde_cbor::ser::SliceWrite;
use serde_derive::{Deserialize, Serialize};

module! {
    type: RustSerde,
    name: "rust_serde",
    author: "Rust for Linux Contributors",
    description: "Rust `serde` sample",
    license: "GPL",
}

struct RustSerde;

pub mod local_data_format {
    #![allow(missing_docs)]

    mod de;
    mod error;
    mod ser;

    pub use de::{from_bytes, Deserializer};
    pub use error::{Error, Result};
    pub use ser::{to_vec, Serializer};
}

#[derive(Serialize, Deserialize, Debug)]
struct S {
    a: (),
    b: bool,
    c: bool,
    d: (),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct User {
    user_id: u32,
    password_hash: [u8; 4],
}

fn cbor_serialize() -> Result<(), serde_cbor::Error> {
    let mut buf = [0u8; 100];
    let writer = SliceWrite::new(&mut buf[..]);
    let mut ser = serde_cbor::Serializer::new(writer);
    let user = User {
        user_id: 42,
        password_hash: [1, 2, 3, 4],
    };
    user.serialize(&mut ser)?;
    let writer = ser.into_inner();
    let size = writer.bytes_written();
    let expected = [
        0xa2, 0x67, 0x75, 0x73, 0x65, 0x72, 0x5f, 0x69, 0x64, 0x18, 0x2a, 0x6d, 0x70, 0x61, 0x73,
        0x73, 0x77, 0x6f, 0x72, 0x64, 0x5f, 0x68, 0x61, 0x73, 0x68, 0x84, 0x1, 0x2, 0x3, 0x4,
    ];
    assert_eq!(&buf[..size], expected);

    crate::pr_info!("cbor serialized = {:?}", buf);

    Ok(())
}

fn cbor_deserialize() -> Result<(), serde_cbor::Error> {
    let value = [
        0xa2, 0x67, 0x75, 0x73, 0x65, 0x72, 0x5f, 0x69, 0x64, 0x18, 0x2a, 0x6d, 0x70, 0x61, 0x73,
        0x73, 0x77, 0x6f, 0x72, 0x64, 0x5f, 0x68, 0x61, 0x73, 0x68, 0x84, 0x1, 0x2, 0x3, 0x4,
    ];

    let user: User = serde_cbor::de::from_slice(&value[..])?;
    assert_eq!(
        user,
        User {
            user_id: 42,
            password_hash: [1, 2, 3, 4],
        }
    );

    crate::pr_info!("cbor deserialized = {:?}", user);
    Ok(())
}

impl kernel::Module for RustSerde {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        pr_info!("Rust serde sample (init)\n");

        let original = S {
            a: (),
            b: false,
            c: true,
            d: (),
        };
        crate::pr_info!("            original = {:?}", original);

        let serialized = kernel::test_serde::to_vec(&original).unwrap();
        crate::pr_info!("          serialized = {:?}", serialized);

        let deserialized: S = kernel::test_serde::from_bytes(&serialized).unwrap();
        crate::pr_info!("        deserialized = {:?}", deserialized);

        let serialized = local_data_format::to_vec(&deserialized).unwrap();
        crate::pr_info!("  serialized (local) = {:?}", serialized);

        let deserialized: S = local_data_format::from_bytes(&serialized).unwrap();
        crate::pr_info!("deserialized (local) = {:?}", deserialized);

        cbor_serialize().unwrap();
        cbor_deserialize().unwrap();

        Ok(RustSerde)
    }
}

impl Drop for RustSerde {
    fn drop(&mut self) {
        pr_info!("Rust serde sample (exit)\n");
    }
}
