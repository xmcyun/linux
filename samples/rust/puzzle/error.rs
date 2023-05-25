use alloc::collections::TryReserveError;
use core::ffi::c_int;
use core::fmt::{self, Display};
use kernel::prelude::{EINVAL, ENOENT, ESPIPE};

// TODO use String in error types (when it's available from the kernel)

pub(crate) enum WireFormatError {
    LocalRefError,
    SeekOtherError,
    ValueMissing,
    CBORError(serde_cbor::Error),
    KernelError(kernel::error::Error),
    TryReserveError(TryReserveError),
}

impl WireFormatError {
    pub(crate) fn to_errno(&self) -> c_int {
        match self {
            WireFormatError::LocalRefError => kernel::error::Error::to_errno(EINVAL),
            WireFormatError::SeekOtherError => kernel::error::Error::to_errno(ESPIPE),
            WireFormatError::ValueMissing => kernel::error::Error::to_errno(ENOENT),
            WireFormatError::CBORError(..) => kernel::error::Error::to_errno(EINVAL),
            WireFormatError::KernelError(e) => kernel::error::Error::to_errno(*e),
            WireFormatError::TryReserveError(_) => kernel::error::Error::to_errno(EINVAL),
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
            WireFormatError::CBORError(_) => f.write_str("CBOR error"),
            WireFormatError::KernelError(_) => f.write_str("Kernel error"),
            WireFormatError::TryReserveError(_) => f.write_str("TryReserveError"),
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
