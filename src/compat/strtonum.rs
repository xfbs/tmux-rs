use core::ffi::{CStr, c_char, c_longlong};

pub unsafe fn strtonum<T>(nptr: *const c_char, minval: T, maxval: T) -> Result<T, &'static CStr>
where
    T: Into<i64>,
    i64: TryInto<T>,
{
    let minval: i64 = minval.into();
    let maxval: i64 = maxval.into();
    if minval > maxval {
        return Err(c"invalid");
    }

    let buf = unsafe { std::slice::from_raw_parts(nptr as *const u8, libc::strlen(nptr)) };
    let s = std::str::from_utf8(buf).map_err(|_| c"invalid")?;
    let n = s.trim_start().parse::<i64>().map_err(|_| c"invalid")?;

    if n < minval {
        return Err(c"too small");
    }

    if n > maxval {
        return Err(c"too large");
    }

    match n.try_into() {
        Ok(value) => Ok(value),
        Err(_) => unreachable!("range check above should prevent this case"),
    }
}
