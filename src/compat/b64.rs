use core::ffi::c_char;
use core::mem::MaybeUninit;

// https://www.rfc-editor.org/rfc/rfc4648

pub unsafe extern "C" fn b64_ntop(
    src: *const u8,
    srclength: usize,
    target: *mut c_char,
    targsize: usize,
) -> i32 {
    let src = unsafe { std::slice::from_raw_parts(src, srclength) };
    let dst = unsafe { std::slice::from_raw_parts_mut(target.cast::<MaybeUninit<u8>>(), targsize) };

    match ntop(src, dst) {
        Ok(out) => (out.len() - 1) as i32,
        Err(_) => -1,
    }
}

/// skips all whitespace anywhere.
/// converts characters, four at a time, starting at (or after)
/// src from base - 64 numbers into three 8 bit bytes in the target area.
/// it returns the number of data bytes stored at the target, or -1 on error.
pub unsafe extern "C" fn b64_pton(src: *const c_char, target: *mut u8, targsize: usize) -> i32 {
    let srclength: usize = unsafe { libc::strlen(src) };
    let src = unsafe { std::slice::from_raw_parts(src.cast::<u8>(), srclength) };
    let dst = unsafe { std::slice::from_raw_parts_mut(target.cast::<MaybeUninit<u8>>(), targsize) };

    match pton(src, dst) {
        Ok(out) => (out.len() - 1) as i32,
        Err(_) => -1,
    }
}

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const REVERSE: [u8; 80] = const {
    let mut tmp: [u8; 80] = [u8::MAX; 80];

    let mut i: u8 = 0;
    while i < ALPHABET.len() as u8 {
        tmp[(ALPHABET[i as usize] - b'+') as usize] = i;
        i += 1;
    }

    tmp
};

fn pton<'out>(src: &'_ [u8], dst: &'out mut [MaybeUninit<u8>]) -> Result<&'out mut [u8], ()> {
    // dst must be at least 3/4 of src, and room for NUL byte
    if src.len().div_ceil(4) * 3 + 1 > dst.len() {
        return Err(());
    }

    let mut i = 0;
    let mut it = src.iter().cloned().filter(|b| !b.is_ascii_whitespace());

    while let Some(ch) = it.next() {
        let chunk: [u8; 4] = [
            ch,
            it.next().ok_or(())?,
            it.next().ok_or(())?,
            it.next().ok_or(())?, // TODO consider special handling for missing =
        ];

        for g in chunk {
            if !matches!(g, b'A'..=b'Z' | b'a'..=b'z' | b'+' | b'/') {
                return Err(());
            }
        }

        let a = REVERSE[(chunk[0] - b'+') as usize];
        let b = REVERSE[(chunk[1] - b'+') as usize];
        let c = REVERSE[(chunk[2] - b'+') as usize];
        let d = REVERSE[(chunk[3] - b'+') as usize];

        //        a                 b                 c                 d
        // X X 0 0 0 0 0 0 | X X 0 0 0 0 0 0 | X X 0 0 0 0 0 0 | X X 0 0 0 0 0 0
        //
        // ( a << 2  ) ( b >> 4 )    (b<<4) (    c >> 2    )     (c<<4)(      d       )
        // 0  0  0  0  0  0  0  0  |  0  0  0  0  0  0  0  0  |  0  0  0  0  0  0  0  0
        //
        dst[i] = MaybeUninit::new((a << 2) | (b >> 4));
        dst[i + 1] = MaybeUninit::new((b << 4) | (c >> 2));
        dst[i + 2] = MaybeUninit::new((c << 4) | d);
        i += 3;
    }

    dst[i] = MaybeUninit::new(0);
    Ok(unsafe { std::slice::from_raw_parts_mut(dst.as_mut_ptr().cast::<u8>(), i) })
}

fn ntop<'out>(src: &'_ [u8], dst: &'out mut [MaybeUninit<u8>]) -> Result<&'out mut [u8], ()> {
    const MASK: u8 = 0b00111111;
    if dst.len() < src.len().div_ceil(3) * 4 + 1 {
        return Err(());
    }

    let mut i = 0;
    let mut it = src.chunks_exact(3);

    for chunk in &mut it {
        dst[i] = MaybeUninit::new(ALPHABET[((chunk[0] >> 2) & MASK) as usize]);
        dst[i + 1] =
            MaybeUninit::new(ALPHABET[(((chunk[0] << 4) | (chunk[1] >> 4)) & MASK) as usize]);
        dst[i + 2] =
            MaybeUninit::new(ALPHABET[(((chunk[1] << 2) | (chunk[2] >> 6)) & MASK) as usize]);
        dst[i + 3] = MaybeUninit::new(ALPHABET[(chunk[2] & MASK) as usize]);
        i += 4;
    }

    let chunk = it.remainder();
    match chunk.len() {
        0 => (),
        1 => {
            dst[i] = MaybeUninit::new(ALPHABET[((chunk[0] >> 2) & MASK) as usize]);
            dst[i + 1] = MaybeUninit::new(ALPHABET[((chunk[0] << 4) & MASK) as usize]);
            dst[i + 2] = MaybeUninit::new(b'=');
            dst[i + 3] = MaybeUninit::new(b'=');
            i += 4;
        }
        2 => {
            dst[i] = MaybeUninit::new(ALPHABET[((chunk[0] >> 2) & MASK) as usize]);
            dst[i + 1] =
                MaybeUninit::new(ALPHABET[(((chunk[0] << 4) | (chunk[1] >> 4)) & MASK) as usize]);
            dst[i + 2] = MaybeUninit::new(ALPHABET[((chunk[1] << 2) & MASK) as usize]);
            dst[i + 3] = MaybeUninit::new(b'=');
            i += 4;
        }
        _ => unreachable!(),
    }

    dst[i] = MaybeUninit::new(b'\0');
    Ok(unsafe { std::slice::from_raw_parts_mut(dst.as_mut_ptr().cast::<u8>(), i) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_b64_pton_valid() {
        let input = c"TWFu";
        let mut output = [0u8; 3];
        let expected = [77, 97, 110];

        unsafe {
            let result = b64_pton(input.as_ptr(), output.as_mut_ptr(), output.len());
            assert_eq!(result, 3);
            assert_eq!(&output, &expected);
        }
    }

    #[test]
    fn test_b64_pton_invalid() {
        let input = c"****";
        let mut output = [0u8; 3];

        unsafe {
            let result = b64_pton(input.as_ptr(), output.as_mut_ptr(), output.len());
            assert_eq!(result, -1);
        }
    }

    #[test]
    fn test_b64_pton_partial() {
        let input = c"TWE=";
        let mut output = [0u8; 2];
        let expected = [77, 97];

        unsafe {
            let result = b64_pton(input.as_ptr(), output.as_mut_ptr(), output.len());
            assert_eq!(result, 2);
            assert_eq!(&output, &expected);
        }
    }
}
