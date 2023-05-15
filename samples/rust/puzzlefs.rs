// SPDX-License-Identifier: GPL-2.0

//! Rust file system sample.

use kernel::module_fs;
use kernel::prelude::*;
use kernel::{c_str, file, fs, io_buffer::IoBufferWriter, fmt, str::CString};

mod puzzle;

module_fs! {
    type: PuzzleFs,
    name: "puzzlefs",
    author: "Ariel Miculas",
    license: "GPL",
}

struct PuzzleFs;

#[derive(Debug)]
struct PuzzlefsInfo {
    base_path: CString,
}

#[vtable]
impl fs::Context<Self> for PuzzleFs {
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

impl fs::Type for PuzzleFs {
    type Context = Self;
    type INodeData = &'static [u8];
    type Data = Box<PuzzlefsInfo>;
    const SUPER_TYPE: fs::Super = fs::Super::Independent;
    const NAME: &'static CStr = c_str!("puzzlefs");
    const FLAGS: i32 = fs::flags::USERNS_MOUNT;
    const DCACHE_BASED: bool = true;

    fn fill_super(_data: (), sb: fs::NewSuperBlock<'_, Self>) -> Result<&fs::SuperBlock<Self>> {
        let base_path = CString::try_from_fmt(fmt!("hello world"))?;
        pr_info!("base_path {:?}\n", base_path);
        let sb = sb.init(
            Box::try_new(PuzzlefsInfo { base_path })?,
            &fs::SuperParams {
                magic: 0x72757374,
                ..fs::SuperParams::DEFAULT
            },
        )?;
        let root = sb.try_new_populated_root_dentry(
            &[],
            kernel::fs_entries![
                file("test1", 0o600, "abc\n".as_bytes(), FsFile),
                file("test2", 0o600, "def\n".as_bytes(), FsFile),
                char("test3", 0o600, [].as_slice(), (10, 125)),
                sock("test4", 0o755, [].as_slice()),
                fifo("test5", 0o755, [].as_slice()),
                block("test6", 0o755, [].as_slice(), (1, 1)),
                dir(
                    "dir1",
                    0o755,
                    [].as_slice(),
                    [
                        file("test1", 0o600, "abc\n".as_bytes(), FsFile),
                        file("test2", 0o600, "def\n".as_bytes(), FsFile),
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
    type FSData = Box<PuzzlefsInfo>;

    fn open(
        fs_info: &PuzzlefsInfo,
        _context: &Self::OpenData,
        _file: &file::File,
    ) -> Result<Self::Data> {
        pr_info!("got {:?}\n", fs_info);

        Ok(())
    }

    fn read(
        _data: (),
        file: &file::File,
        writer: &mut impl IoBufferWriter,
        offset: u64,
    ) -> Result<usize> {
        file::read_from_slice(
            file.inode::<PuzzleFs>().ok_or(EINVAL)?.fs_data(),
            writer,
            offset,
        )
    }
}
