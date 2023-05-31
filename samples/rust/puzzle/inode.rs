// This contents of this file is taken from puzzlefs.rs (the userspace implementation)
// It is named inode.rs instead puzzlefs.rs since the root of this kernel module already has that name

use crate::puzzle::error::Result;
use crate::puzzle::error::WireFormatError;
use crate::puzzle::oci::Image;
use crate::puzzle::types as format;
use crate::puzzle::types::Digest;
use crate::puzzle::types::{FileChunk, Ino, InodeAdditional, MetadataBlob};
use alloc::vec::Vec;
use kernel::mount::Vfsmount;
use kernel::prelude::{ENOENT, ENOTDIR};
use kernel::str::CStr;
use kernel::sync::Arc;

#[derive(Debug)]
pub(crate) struct Inode {
    pub(crate) inode: format::Inode,
    pub(crate) mode: InodeMode,
    #[allow(dead_code)]
    pub(crate) additional: Option<InodeAdditional>,
}

pub(crate) struct PuzzleFS {
    pub(crate) oci: Image,
    layers: Vec<format::MetadataBlob>,
}

impl PuzzleFS {
    pub(crate) fn open(vfsmount: Arc<Vfsmount>, rootfs_path: &CStr) -> Result<PuzzleFS> {
        let oci = Image::open(vfsmount)?;
        let rootfs = oci.open_rootfs_blob(rootfs_path)?;

        let layers =
            Vec::from_iter_fallible(rootfs.metadatas.iter().map(|md| -> Result<MetadataBlob> {
                let digest = Digest::try_from(md)?;
                oci.open_metadata_blob(&digest)
            }))?
            .process_results()?;

        Ok(PuzzleFS { oci, layers })
    }

    pub(crate) fn find_inode(&mut self, ino: u64) -> Result<Inode> {
        for layer in self.layers.iter_mut() {
            if let Some(inode) = layer.find_inode(ino)? {
                return Inode::new(layer, inode);
            }
        }
        Err(WireFormatError::from_errno(ENOENT))
    }
}

impl Inode {
    fn new(layer: &mut MetadataBlob, inode: format::Inode) -> Result<Inode> {
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
                        .iter_mut()
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
