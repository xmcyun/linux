use core::fmt::{self, Display};

// TODO use String in error types (when it's available from the kernel)
// TODO cannot derive Debug because serde_cbor::Error doesn't support it
// make -f ./scripts/Makefile.modpost
// # MODPOST Module.symvers
//    scripts/mod/modpost   -E    -o Module.symvers -T modules.order vmlinux.o
// ERROR: modpost: "_RNvXs0_NtCsalintqihKyV_10serde_cbor5errorNtB5_5ErrorNtNtCs3yuwAp0waWO_4core3fmt5Debug3fmt" [samples/rust/puzzlefs.ko] undefined!
// make[1]: *** [scripts/Makefile.modpost:136: Module.symvers] Error 1
// make: *** [Makefile:1990: modpost] Error 2

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
            WireFormatError::KernelError(_) => f.write_str("Kernel error"),
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
