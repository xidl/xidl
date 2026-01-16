use crate::error::{XcdrError, XcdrResult};
use crate::utils::ToNeBytes;

pub(crate) trait AlignStrategy {
    fn align_to(len: usize) -> usize;
}

pub(crate) struct Align4;
pub(crate) struct Align8;
pub(crate) struct AlignCdr2;

impl AlignStrategy for Align4 {
    fn align_to(_len: usize) -> usize {
        4
    }
}

impl AlignStrategy for Align8 {
    fn align_to(_len: usize) -> usize {
        8
    }
}

impl AlignStrategy for AlignCdr2 {
    fn align_to(len: usize) -> usize {
        match len {
            8 | 16 => 4,
            _ => 8,
        }
    }
}

fn padding<S: AlignStrategy>(len: usize) -> usize {
    let align_to = S::align_to(len);
    match len % align_to {
        0 => 0usize,
        v => align_to - v,
    }
}

pub(crate) fn align_pos<S: AlignStrategy>(
    pos: &mut usize,
    len: usize,
    limit: usize,
    check_align: bool,
) -> XcdrResult<()> {
    let pad = padding::<S>(len);
    if check_align && *pos + pad > limit {
        return Err(XcdrError::BufferOverflow);
    }
    *pos += pad;
    Ok(())
}

pub(crate) fn read_aligned<S: AlignStrategy, const N: usize>(
    buf: &[u8],
    pos: &mut usize,
) -> XcdrResult<[u8; N]> {
    read_aligned_with_limit::<S, N>(buf, pos, buf.len())
}

pub(crate) fn read_aligned_with_limit<S: AlignStrategy, const N: usize>(
    buf: &[u8],
    pos: &mut usize,
    limit: usize,
) -> XcdrResult<[u8; N]> {
    let pad = padding::<S>(N);
    if *pos + pad + N > limit {
        return Err(XcdrError::BufferOverflow);
    }

    *pos += pad;
    let mut out = [0u8; N];
    out.copy_from_slice(&buf[*pos..*pos + N]);
    *pos += N;
    Ok(out)
}

pub(crate) fn write_aligned<S: AlignStrategy, T, const N: usize>(
    buf: *mut u8,
    len: usize,
    pos: &mut usize,
    do_io: bool,
    val: T,
    check_align: bool,
) -> XcdrResult<()>
where
    T: ToNeBytes<N>,
{
    let size = size_of::<T>();
    if *pos + size > len {
        return Err(XcdrError::BufferOverflow);
    }

    let pad = padding::<S>(size);
    if check_align && *pos + pad > len {
        return Err(XcdrError::BufferOverflow);
    }
    *pos += pad;

    if do_io {
        let src = &val.to_ne_bytes();
        unsafe {
            core::ptr::copy(
                core::ptr::addr_of!(*src) as *const u8,
                buf.add(*pos),
                src.len(),
            );
        }
    }

    *pos += size;
    Ok(())
}
