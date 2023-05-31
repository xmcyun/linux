use crate::puzzle::error::WireFormatError;
use alloc::vec::Vec;
use core::mem::size_of;
use serde::de::Error as SerdeError;
use serde::de::Visitor;
use serde::{Deserialize, Deserializer};
use serde_derive::Deserialize;
mod cbor_helpers;
use crate::puzzle::error::Result;
pub(crate) use cbor_helpers::{cbor_get_array_size, cbor_size_of_list_header};
use core::fmt;
use hex::{encode_hex_iter, FromHexError};
use kernel::file;
use kernel::str::CStr;

#[derive(Deserialize, Debug)]
pub(crate) struct InodeAdditional {
    #[allow(dead_code)]
    pub(crate) xattrs: Vec<Xattr>,
    #[allow(dead_code)]
    pub(crate) symlink_target: Option<Vec<u8>>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct Xattr {
    #[allow(dead_code)]
    pub(crate) key: Vec<u8>,
    #[allow(dead_code)]
    pub(crate) val: Vec<u8>,
}

#[derive(Debug)]
pub(crate) struct MetadataBlob {
    pub(crate) mmapped_region: Vec<u8>,
    pub(crate) inode_count: usize,
}

pub(crate) const SHA256_BLOCK_SIZE: usize = 32;

fn read_one_from_slice<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T> {
    // serde complains when we leave extra bytes on the wire, which we often want to do. as a
    // hack, we create a streaming deserializer for the type we're about to read, and then only
    // read one value.
    let mut iter = serde_cbor::Deserializer::from_slice(bytes).into_iter::<T>();
    let v = iter.next().transpose()?;
    v.ok_or(WireFormatError::ValueMissing)
}

#[derive(Deserialize, Debug)]
pub(crate) struct Rootfs {
    pub(crate) metadatas: Vec<BlobRef>,
    // TODO: deserialize fs_verity_data, for the moment BTreeMap is not supported
    #[allow(dead_code)]
    pub(crate) fs_verity_data: (),
    #[allow(dead_code)]
    pub(crate) manifest_version: u64,
}

impl Rootfs {
    pub(crate) fn open(file: file::RegularFile) -> Result<Rootfs> {
        let buffer = file.read_to_end()?;
        read_one_from_slice(&buffer)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlobRefKind {
    Local,
    Other { digest: [u8; 32] },
}

const BLOB_REF_SIZE: usize = 1 /* mode */ + 32 /* digest */ + 8 /* offset */;

// TODO: should this be an ociv1 digest and include size and media type?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BlobRef {
    pub(crate) offset: u64,
    pub(crate) kind: BlobRefKind,
    pub(crate) compressed: bool,
}

const COMPRESSED_BIT: u8 = 1 << 7;

impl BlobRef {
    fn fixed_length_deserialize<E: SerdeError>(
        state: &[u8; BLOB_REF_SIZE],
    ) -> kernel::error::Result<BlobRef, E> {
        let offset = u64::from_le_bytes(state[0..8].try_into().unwrap());

        let compressed = (state[8] & COMPRESSED_BIT) != 0;
        let kind = match state[8] & !COMPRESSED_BIT {
            0 => BlobRefKind::Local,
            1 => BlobRefKind::Other {
                digest: state[9..41].try_into().unwrap(),
            },
            _ => {
                return Err(SerdeError::custom(format_args!(
                    "bad blob ref kind {}",
                    state[0]
                )))
            }
        };

        Ok(BlobRef {
            offset,
            kind,
            compressed,
        })
    }
}

impl<'de> Deserialize<'de> for BlobRef {
    fn deserialize<D>(deserializer: D) -> kernel::error::Result<BlobRef, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct BlobRefVisitor;

        impl<'de> Visitor<'de> for BlobRefVisitor {
            type Value = BlobRef;

            fn expecting(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                formatter.write_fmt(format_args!("expected {BLOB_REF_SIZE} bytes for BlobRef"))
            }

            fn visit_bytes<E>(self, v: &[u8]) -> kernel::error::Result<BlobRef, E>
            where
                E: SerdeError,
            {
                let state: [u8; BLOB_REF_SIZE] = v
                    .try_into()
                    .map_err(|_| SerdeError::invalid_length(v.len(), &self))?;
                BlobRef::fixed_length_deserialize(&state)
            }
        }

        deserializer.deserialize_bytes(BlobRefVisitor)
    }
}

impl MetadataBlob {
    pub(crate) fn new(mut f: file::RegularFile) -> Result<MetadataBlob> {
        let inodes_count = cbor_get_array_size(&mut f)? as usize;
        let mmapped_region = f.read_to_end()?;
        Ok(MetadataBlob {
            mmapped_region,
            inode_count: inodes_count,
        })
    }

    pub(crate) fn seek_ref(&mut self, r: &BlobRef) -> Result<u64> {
        match r.kind {
            BlobRefKind::Other { .. } => Err(WireFormatError::SeekOtherError),
            BlobRefKind::Local => Ok(r.offset),
        }
    }

    pub(crate) fn read_file_chunks(&mut self, offset: u64) -> Result<Vec<FileChunk>> {
        read_one_from_slice::<FileChunkList>(&self.mmapped_region[offset as usize..])
            .map(|cl| cl.chunks)
    }

    pub(crate) fn read_dir_list(&mut self, offset: u64) -> Result<DirList> {
        read_one_from_slice(&self.mmapped_region[offset as usize..])
    }

    pub(crate) fn read_inode_additional(&mut self, r: &BlobRef) -> Result<InodeAdditional> {
        let offset = self.seek_ref(r)? as usize;
        read_one_from_slice(&self.mmapped_region[offset..])
    }

    pub(crate) fn find_inode(&mut self, ino: Ino) -> Result<Option<Inode>> {
        let mut left = 0;
        let mut right = self.inode_count;

        while left <= right {
            let mid = left + (right - left) / 2;
            let mid_offset = cbor_size_of_list_header(self.inode_count) + mid * INODE_WIRE_SIZE;
            let i = read_one_from_slice::<Inode>(
                &self.mmapped_region[mid_offset..mid_offset + INODE_WIRE_SIZE],
            )?;
            if i.ino == ino {
                return Ok(Some(i));
            }

            if i.ino < ino {
                left = mid + 1;
            } else {
                // don't underflow...
                if mid == 0 {
                    break;
                }
                right = mid - 1;
            };
        }

        Ok(None)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Digest([u8; SHA256_BLOCK_SIZE]);

impl Digest {
    pub(crate) fn underlying(&self) -> [u8; SHA256_BLOCK_SIZE] {
        let mut dest = [0_u8; SHA256_BLOCK_SIZE];
        dest.copy_from_slice(&self.0);
        dest
    }
}

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut hex_string =
            Vec::from_iter_fallible(encode_hex_iter(&self.underlying())).map_err(|_| fmt::Error)?;
        // append NUL character
        hex_string.try_push(0).map_err(|_| fmt::Error)?;
        let hex_string = CStr::from_bytes_with_nul(&hex_string).map_err(|_| fmt::Error)?;
        write!(f, "{}", hex_string)
    }
}

impl TryFrom<&CStr> for Digest {
    type Error = FromHexError;
    fn try_from(s: &CStr) -> kernel::error::Result<Self, Self::Error> {
        let digest = hex::decode(s)?;
        let digest: [u8; SHA256_BLOCK_SIZE] = digest
            .try_into()
            .map_err(|_| FromHexError::InvalidStringLength)?;
        Ok(Digest(digest))
    }
}

impl TryFrom<BlobRef> for Digest {
    type Error = WireFormatError;
    fn try_from(v: BlobRef) -> kernel::error::Result<Self, Self::Error> {
        match v.kind {
            BlobRefKind::Other { digest } => Ok(Digest(digest)),
            BlobRefKind::Local => Err(WireFormatError::LocalRefError),
        }
    }
}

impl TryFrom<&BlobRef> for Digest {
    type Error = WireFormatError;
    fn try_from(v: &BlobRef) -> kernel::error::Result<Self, Self::Error> {
        match v.kind {
            BlobRefKind::Other { digest } => Ok(Digest(digest)),
            BlobRefKind::Local => Err(WireFormatError::LocalRefError),
        }
    }
}

#[derive(Deserialize, Debug)]
pub(crate) struct DirEnt {
    pub(crate) ino: Ino,
    pub(crate) name: Vec<u8>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct DirList {
    // TODO: flags instead?
    #[allow(dead_code)]
    pub(crate) look_below: bool,
    pub(crate) entries: Vec<DirEnt>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct FileChunkList {
    pub(crate) chunks: Vec<FileChunk>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct FileChunk {
    pub(crate) blob: BlobRef,
    pub(crate) len: u64,
}

const INODE_MODE_SIZE: usize = 1 /* mode */ + size_of::<u64>() * 2 /* major/minor/offset */;

// InodeMode needs to have custom serialization because inodes must be a fixed size.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum InodeMode {
    Unknown,
    Fifo,
    Chr { major: u64, minor: u64 },
    Dir { offset: u64 },
    Blk { major: u64, minor: u64 },
    Reg { offset: u64 },
    Lnk,
    Sock,
    Wht,
}

pub(crate) type Ino = u64;

const INODE_SIZE: usize = size_of::<Ino>() + INODE_MODE_SIZE + 2 * size_of::<u32>() /* uid and gid */
+ size_of::<u16>() /* permissions */ + 1 /* Option<BlobRef> */ + BLOB_REF_SIZE;

pub(crate) const INODE_WIRE_SIZE: usize = cbor_size_of_list_header(INODE_SIZE) + INODE_SIZE;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Inode {
    pub(crate) ino: Ino,
    pub(crate) mode: InodeMode,
    pub(crate) uid: u32,
    pub(crate) gid: u32,
    pub(crate) permissions: u16,
    pub(crate) additional: Option<BlobRef>,
}

impl<'de> Deserialize<'de> for Inode {
    fn deserialize<D>(deserializer: D) -> kernel::error::Result<Inode, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct InodeVisitor;

        impl<'de> Visitor<'de> for InodeVisitor {
            type Value = Inode;

            fn expecting(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                formatter.write_fmt(format_args!("expected {INODE_MODE_SIZE} bytes for Inode"))
            }

            fn visit_bytes<E>(self, v: &[u8]) -> kernel::error::Result<Inode, E>
            where
                E: SerdeError,
            {
                let state: [u8; INODE_SIZE] = v
                    .try_into()
                    .map_err(|_| SerdeError::invalid_length(v.len(), &self))?;

                let mode = match state[8] {
                    0 => InodeMode::Unknown,
                    1 => InodeMode::Fifo,
                    2 => {
                        let major = u64::from_le_bytes(state[9..17].try_into().unwrap());
                        let minor = u64::from_le_bytes(state[17..25].try_into().unwrap());
                        InodeMode::Chr { major, minor }
                    }
                    4 => {
                        let offset = u64::from_le_bytes(state[9..17].try_into().unwrap());
                        InodeMode::Dir { offset }
                    }
                    6 => {
                        let major = u64::from_le_bytes(state[9..17].try_into().unwrap());
                        let minor = u64::from_le_bytes(state[17..25].try_into().unwrap());
                        InodeMode::Blk { major, minor }
                    }
                    8 => {
                        let offset = u64::from_le_bytes(state[9..17].try_into().unwrap());
                        InodeMode::Reg { offset }
                    }
                    10 => InodeMode::Lnk,
                    12 => InodeMode::Sock,
                    14 => InodeMode::Wht,
                    _ => {
                        return Err(SerdeError::custom(format_args!(
                            "bad inode mode value {}",
                            state[8]
                        )))
                    }
                };

                let additional = if state[35] > 0 {
                    Some(BlobRef::fixed_length_deserialize(
                        state[36..36 + BLOB_REF_SIZE].try_into().unwrap(),
                    )?)
                } else {
                    None
                };

                Ok(Inode {
                    // ugh there must be a nicer way to do this with arrays, which we already have
                    // from above...
                    ino: u64::from_le_bytes(state[0..8].try_into().unwrap()),
                    mode,
                    uid: u32::from_le_bytes(state[25..29].try_into().unwrap()),
                    gid: u32::from_le_bytes(state[29..33].try_into().unwrap()),
                    permissions: u16::from_le_bytes(state[33..35].try_into().unwrap()),
                    additional,
                })
            }
        }

        deserializer.deserialize_bytes(InodeVisitor)
    }
}
