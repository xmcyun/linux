use alloc::collections::TryReserveError;
use core::ffi::c_int;
use core::fmt::{self, Display};
use kernel::prelude::{EINVAL, ENOENT, ESPIPE};

// TODO use String in error types (when it's available from the kernel)
// TODO cannot derive Debug because serde_cbor::Error doesn't support it
// make -f ./scripts/Makefile.modpost
// # MODPOST Module.symvers
//    scripts/mod/modpost   -E    -o Module.symvers -T modules.order vmlinux.o
// ERROR: modpost: "_RNvXs0_NtCsalintqihKyV_10serde_cbor5errorNtB5_5ErrorNtNtCs3yuwAp0waWO_4core3fmt5Debug3fmt" [samples/rust/puzzlefs.ko] undefined!
// make[1]: *** [scripts/Makefile.modpost:136: Module.symvers] Error 1
// make: *** [Makefile:1990: modpost] Error 2
// Demangled symbol:
// $ rustfilt _RNvXs0_NtCsalintqihKyV_10serde_cbor5errorNtB5_5ErrorNtNtCs3yuwAp0waWO_4core3fmt5Debug3fmt
// <serde_cbor::error::Error as core::fmt::Debug>::fmt

// #[derive(Debug)]
pub(crate) enum WireFormatError {
    LocalRefError,
    SeekOtherError,
    ValueMissing,
    InvalidImageSchema,
    InvalidImageVersion,
    InvalidFsVerityData,
    CBORError(serde_cbor::Error),
    KernelError(kernel::error::Error),
    TryReserveError(TryReserveError),
    HexError(hex::FromHexError),
}

impl WireFormatError {
    pub(crate) fn to_errno(&self) -> c_int {
        match self {
            WireFormatError::LocalRefError => kernel::error::Error::to_errno(EINVAL),
            WireFormatError::SeekOtherError => kernel::error::Error::to_errno(ESPIPE),
            WireFormatError::ValueMissing => kernel::error::Error::to_errno(ENOENT),
            WireFormatError::InvalidImageSchema => kernel::error::Error::to_errno(EINVAL),
            WireFormatError::InvalidImageVersion => kernel::error::Error::to_errno(EINVAL),
            WireFormatError::InvalidFsVerityData => kernel::error::Error::to_errno(EINVAL),
            WireFormatError::CBORError(..) => kernel::error::Error::to_errno(EINVAL),
            WireFormatError::KernelError(e) => kernel::error::Error::to_errno(*e),
            WireFormatError::TryReserveError(_) => kernel::error::Error::to_errno(EINVAL),
            WireFormatError::HexError(_) => kernel::error::Error::to_errno(EINVAL),
        }
    }

    pub(crate) fn from_errno(errno: kernel::error::Error) -> Self {
        Self::KernelError(errno)
    }
}

impl Display for WireFormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WireFormatError::LocalRefError => f.write_str("cannot turn local ref into a digest"),
            WireFormatError::SeekOtherError => f.write_str("cannot seek to other blob"),
            WireFormatError::ValueMissing => f.write_str("no value present"),
            WireFormatError::InvalidImageSchema => f.write_str("invalid image schema"),
            WireFormatError::InvalidImageVersion => f.write_str("invalid image version"),
            WireFormatError::InvalidFsVerityData => f.write_str("invalid fs verity data"),
            WireFormatError::CBORError(_) => f.write_str("CBOR error"),
            WireFormatError::KernelError(e) => write!(f, "Kernel error {:?}", e.to_errno()),
            WireFormatError::TryReserveError(_) => f.write_str("TryReserveError"),
            WireFormatError::HexError(_) => f.write_str("HexError"),
        }
    }
}

pub(crate) type Result<T> = kernel::error::Result<T, WireFormatError>;

// TODO figure out how to use thiserror
#[allow(unused_qualifications)]
impl core::convert::From<serde_cbor::Error> for WireFormatError {
    #[allow(deprecated)]
    fn from(source: serde_cbor::Error) -> Self {
        WireFormatError::CBORError(source)
    }
}

#[allow(unused_qualifications)]
impl core::convert::From<kernel::error::Error> for WireFormatError {
    #[allow(deprecated)]
    fn from(source: kernel::error::Error) -> Self {
        WireFormatError::KernelError(source)
    }
}

#[allow(unused_qualifications)]
impl core::convert::From<TryReserveError> for WireFormatError {
    #[allow(deprecated)]
    fn from(source: TryReserveError) -> Self {
        WireFormatError::TryReserveError(source)
    }
}

#[allow(unused_qualifications)]
impl core::convert::From<hex::FromHexError> for WireFormatError {
    #[allow(deprecated)]
    fn from(source: hex::FromHexError) -> Self {
        WireFormatError::HexError(source)
    }
}

#[allow(unused_qualifications)]
impl core::convert::From<WireFormatError> for kernel::error::Error {
    #[allow(deprecated)]
    fn from(source: WireFormatError) -> Self {
        kernel::error::Error::from_errno(source.to_errno())
    }
}
