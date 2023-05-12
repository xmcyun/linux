pub(crate) const fn cbor_size_of_list_header(size: usize) -> usize {
    match size {
        0..=23 => 1,
        24..=255 => 2,
        256..=65535 => 3,
        65536..=4294967295 => 4,
        _ => 8,
    }
}
