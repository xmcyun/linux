// SPDX-License-Identifier: GPL-2.0

//! Rust file system sample.

use kernel::prelude::*;
use kernel::{c_str, file, fs, io_buffer::IoBufferWriter};
use kernel::module_fs;

module_fs! {
    type: RustFs,
    name: "rwrapfs",
    author: "xieminsheng",
    license: "GPL",
}

struct RustFs;

#[vtable]
impl fs::Context<Self> for RustFs {
    type Data = ();

    kernel::define_fs_params! {(),
        {flag, "flag", |_, v| { pr_info!("flag passed-in: {v}\n"); Ok(()) } },
        {flag_no, "flagno", |_, v| { pr_info!("flagno passed-in: {v}\n"); Ok(()) } },
        {bool, "bool", |_, v| { pr_info!("bool passed-in: {v}\n"); Ok(()) } },
        {u32, "u32", |_, v| { pr_info!("u32 passed-in: {v}\n"); Ok(()) } },
        {u32oct, "u32oct", |_, v| { pr_info!("u32oct passed-in: {v}\n"); Ok(()) } },
        {u32hex, "u32hex", |_, v| { pr_info!("u32hex passed-in: {v}\n"); Ok(()) } },
        {s32, "s32", |_, v| { pr_info!("s32 passed-in: {v}\n"); Ok(()) } },
        {u64, "u64", |_, v| { pr_info!("u64 passed-in: {v}\n"); Ok(()) } },
        {string, "string", |_, v| { pr_info!("string passed-in: {v}\n"); Ok(()) } },
        {enum, "enum", [("first", 10), ("second", 20)], |_, v| {
            pr_info!("enum passed-in: {v}\n"); Ok(()) }
        },
    }

    fn try_new() -> Result {
        Ok(())
    }
}

impl fs::Type for RustFs {
    type Context = Self;
    type INodeData = &'static [u8];
    const SUPER_TYPE: fs::Super = fs::Super::Independent;
    const NAME: &'static CStr = c_str!("rwrapfs");
    const FLAGS: i32 = fs::flags::USERNS_MOUNT;
    const DCACHE_BASED: bool = true;

    fn fill_super(_data: (), sb: fs::NewSuperBlock<'_, Self>) -> Result<&fs::SuperBlock<Self>> {
        let sb = sb.init(
            (),
            &fs::SuperParams {
                magic: 0x08041234,
                ..fs::SuperParams::DEFAULT
            },
        )?;
        let root = sb.try_new_populated_root_dentry(
            &[],
            kernel::fs_entries![
                file("vivo_file1", 0o600, "vivo_file1\n".as_bytes(), FsFile),
                file("vivo_file2", 0o600, "vivo_file2\n".as_bytes(), FsFile),
                // char("test3", 0o600, [].as_slice(), (10, 125)),
                // sock("test4", 0o755, [].as_slice()),
                // fifo("test5", 0o755, [].as_slice()),
                // block("test6", 0o755, [].as_slice(), (1, 1)),
                dir(
                    "vivo_dir1",
                    0o755,
                    [].as_slice(),
                    [
                        file("vivo_file3", 0o600, "vivo_file3\n".as_bytes(), FsFile),
                        file("vivo_file4", 0o600, "vivo_file4\n".as_bytes(), FsFile),
                    ]
                ),
            ],
        )?;
        let sb = sb.init_root(root)?;
        Ok(sb)
    }
}

struct FsFile;

#[vtable]
impl file::Operations for FsFile {
    type OpenData = &'static [u8];

    fn open(_context: &Self::OpenData, _file: &file::File) -> Result<Self::Data> {
        Ok(())
    }

    fn read(
        _data: (),
        file: &file::File,
        writer: &mut impl IoBufferWriter,
        offset: u64,
    ) -> Result<usize> {
        file::read_from_slice(
            file.inode::<RustFs>().ok_or(EINVAL)?.fs_data(),
            writer,
            offset,
        )
    }
}
