// This contents of this file is taken from puzzlefs.rs (the userspace implementation)
// It is named inode.rs instead puzzlefs.rs since the root of this kernel module already has that name

use crate::puzzle::error::Result;
use crate::puzzle::error::WireFormatError;
use crate::puzzle::oci::Image;
use crate::puzzle::types as format;
use crate::puzzle::types::Digest;
use crate::puzzle::types::{FileChunk, Ino, InodeAdditional, MetadataBlob};
use alloc::vec::Vec;
use core::cmp::min;
use kernel::mount::Vfsmount;
use kernel::prelude::{ENOENT, ENOTDIR};
use kernel::str::CStr;

#[derive(Debug)]
pub(crate) struct Inode {
    pub(crate) inode: format::Inode,
    pub(crate) mode: InodeMode,
    #[allow(dead_code)]
    pub(crate) additional: Option<InodeAdditional>,
}

#[derive(Debug)]
pub(crate) struct PuzzleFS {
    pub(crate) oci: Image,
    layers: Vec<format::MetadataBlob>,
}

impl PuzzleFS {
    pub(crate) fn open(oci_root_dir: &CStr, rootfs_path: &CStr) -> Result<PuzzleFS> {
        let vfs_mount = Vfsmount::new_private_mount(oci_root_dir)?;
        let oci = Image::open(vfs_mount)?;
        let rootfs = oci.open_rootfs_blob(rootfs_path)?;

        let layers =
            Vec::from_iter_fallible(rootfs.metadatas.iter().map(|md| -> Result<MetadataBlob> {
                let digest = Digest::try_from(md)?;
                oci.open_metadata_blob(&digest)
            }))?
            .process_results()?;

        Ok(PuzzleFS { oci, layers })
    }

    pub(crate) fn find_inode(&self, ino: u64) -> Result<Inode> {
        for layer in self.layers.iter() {
            if let Some(inode) = layer.find_inode(ino)? {
                return Inode::new(layer, inode);
            }
        }
        Err(WireFormatError::from_errno(ENOENT))
    }
}

impl Inode {
    fn new(layer: &MetadataBlob, inode: format::Inode) -> Result<Inode> {
        let mode = match inode.mode {
            format::InodeMode::Reg { offset } => {
                let chunks = layer.read_file_chunks(offset)?;
                InodeMode::File { chunks }
            }
            format::InodeMode::Dir { offset } => {
                // TODO: implement something like collect_fallible (since try_collect already exists with another purpose)
                let mut entries = Vec::from_iter_fallible(
                    layer
                        .read_dir_list(offset)?
                        .entries
                        .iter()
                        .map(|de| (de.name.try_clone().unwrap(), de.ino)),
                )?;
                // Unstable sort is used because it avoids memory allocation
                // There should not be two directories with the same name, so stable sort doesn't have any advantage
                entries.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));
                InodeMode::Dir { entries }
            }
            _ => InodeMode::Other,
        };

        let additional = inode
            .additional
            .map(|additional_ref| layer.read_inode_additional(&additional_ref))
            .transpose()?;

        Ok(Inode {
            inode,
            mode,
            additional,
        })
    }
}

#[derive(Debug)]
pub(crate) enum InodeMode {
    File { chunks: Vec<FileChunk> },
    Dir { entries: Vec<(Vec<u8>, Ino)> },
    Other,
}

pub(crate) fn file_read(
    oci: &Image,
    inode: &Inode,
    offset: usize,
    data: &mut [u8],
) -> Result<usize> {
    let chunks = match &inode.mode {
        InodeMode::File { chunks } => chunks,
        _ => return Err(WireFormatError::from_errno(ENOTDIR)),
    };

    // TODO: fix all this casting...
    let end = offset + data.len();

    let mut file_offset = 0;
    let mut buf_offset = 0;
    for chunk in chunks {
        // have we read enough?
        if file_offset > end {
            break;
        }

        // should we skip this chunk?
        if file_offset + (chunk.len as usize) < offset {
            file_offset += chunk.len as usize;
            continue;
        }

        let addl_offset = if offset > file_offset {
            offset - file_offset
        } else {
            0
        };

        // ok, need to read this chunk; how much?
        let left_in_buf = data.len() - buf_offset;
        let to_read = min(left_in_buf, chunk.len as usize - addl_offset);

        let start = buf_offset;
        let finish = start + to_read;
        file_offset += addl_offset;

        // how many did we actually read?
        let n = oci.fill_from_chunk(chunk.blob, addl_offset as u64, &mut data[start..finish])?;
        file_offset += n;
        buf_offset += n;
    }

    // discard any extra if we hit EOF
    Ok(buf_offset)
}
