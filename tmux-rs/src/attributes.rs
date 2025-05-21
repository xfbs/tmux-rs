use libc::{strcasecmp, strchr, strcspn, strlen, strncasecmp, strspn};

use super::*;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn attributes_tostring(attr: c_int) -> *const c_char {
    type buffer = [c_char; 512];
    static mut buf: buffer = unsafe { zeroed() };

    if attr == 0 {
        return c"none".as_ptr();
    }

    unsafe {
        #[rustfmt::skip]
        let len: isize = xsnprintf(
            &raw mut buf as _,
            size_of::<buffer>(),
            c"%s%s%s%s%s%s%s%s%s%s%s%s%s%s".as_ptr(),
            if attr & GRID_ATTR_CHARSET as i32 != 0 { c"acs,".as_ptr() } else { c"".as_ptr() },
            if attr & GRID_ATTR_BRIGHT as i32 != 0 { c"bright,".as_ptr() } else { c"".as_ptr() },
            if attr & GRID_ATTR_DIM as i32 != 0 { c"dim,".as_ptr() } else { c"".as_ptr() },
            if attr & GRID_ATTR_UNDERSCORE as i32 != 0 { c"underscore,".as_ptr() } else { c"".as_ptr() },
            if attr & GRID_ATTR_BLINK as i32 != 0 { c"blink,".as_ptr() } else { c"".as_ptr() },
            if attr & GRID_ATTR_REVERSE as i32 != 0 { c"reverse,".as_ptr() } else { c"".as_ptr() },
            if attr & GRID_ATTR_HIDDEN as i32 != 0 { c"hidden,".as_ptr() } else { c"".as_ptr() },
            if attr & GRID_ATTR_ITALICS as i32 != 0 { c"italics,".as_ptr() } else { c"".as_ptr() },
            if attr & GRID_ATTR_STRIKETHROUGH as i32 != 0 { c"strikethrough,".as_ptr() } else { c"".as_ptr() },
            if attr & GRID_ATTR_UNDERSCORE_2 as i32 != 0 { c"double-underscore,".as_ptr() } else { c"".as_ptr() },
            if attr & GRID_ATTR_UNDERSCORE_3 as i32 != 0 { c"curly-underscore,".as_ptr() } else { c"".as_ptr() },
            if attr & GRID_ATTR_UNDERSCORE_4 as i32 != 0 { c"dotted-underscore,".as_ptr() } else { c"".as_ptr() },
            if attr & GRID_ATTR_UNDERSCORE_5 as i32 != 0 { c"dashed-underscore,".as_ptr() } else { c"".as_ptr() },
            if attr & GRID_ATTR_OVERLINE as i32 != 0 { c"overline,".as_ptr() } else { c"".as_ptr() },
        ) as isize;
        if len > 0 {
            buf[len as usize - 1] = b'\0' as c_char;
        }

        &raw mut buf as _
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn attributes_fromstring(mut str: *const c_char) -> c_int {
    struct table_entry {
        name: *const c_char,
        attr: i32,
    }
    let delimiters = c" ,|".as_ptr();

    #[rustfmt::skip]
    let table = [
        table_entry { name: c"acs".as_ptr(), attr: GRID_ATTR_CHARSET as i32, },
        table_entry { name: c"bright".as_ptr(), attr: GRID_ATTR_BRIGHT as i32, },
        table_entry { name: c"bold".as_ptr(), attr: GRID_ATTR_BRIGHT as i32, },
        table_entry { name: c"dim".as_ptr(), attr: GRID_ATTR_DIM as i32, },
        table_entry { name: c"underscore".as_ptr(), attr: GRID_ATTR_UNDERSCORE as i32, },
        table_entry { name: c"blink".as_ptr(), attr: GRID_ATTR_BLINK as i32, },
        table_entry { name: c"reverse".as_ptr(), attr: GRID_ATTR_REVERSE as i32, },
        table_entry { name: c"hidden".as_ptr(), attr: GRID_ATTR_HIDDEN as i32, },
        table_entry { name: c"italics".as_ptr(), attr: GRID_ATTR_ITALICS as i32, },
        table_entry { name: c"strikethrough".as_ptr(), attr: GRID_ATTR_STRIKETHROUGH as i32, },
        table_entry { name: c"double-underscore".as_ptr(), attr: GRID_ATTR_UNDERSCORE_2 as i32, },
        table_entry { name: c"curly-underscore".as_ptr(), attr: GRID_ATTR_UNDERSCORE_3 as i32, },
        table_entry { name: c"dotted-underscore".as_ptr(), attr: GRID_ATTR_UNDERSCORE_4 as i32, },
        table_entry { name: c"dashed-underscore".as_ptr(), attr: GRID_ATTR_UNDERSCORE_5 as i32, },
        table_entry { name: c"overline".as_ptr(), attr: GRID_ATTR_OVERLINE as i32, },
    ];

    unsafe {
        if *str == b'\0' as c_char || libc::strcspn(str, delimiters) == 0 {
            return -1;
        }
        if !strchr(delimiters, *str.add(strlen(str) - 1) as i32).is_null() {
            return -1;
        }

        if strcasecmp(str, c"default".as_ptr()) == 0 || strcasecmp(str, c"none".as_ptr()) == 0 {
            return 0;
        }

        let mut attr = 0;
        loop {
            let end = strcspn(str, delimiters);
            let mut i = 0;
            for j in 0..table.len() {
                i = j;
                if end != strlen(table[i].name) {
                    continue;
                }
                if strncasecmp(str, table[i].name, end) == 0 {
                    attr |= table[i].attr;
                    break;
                }
            }
            if i == table.len() {
                return -1;
            }
            str = str.add(end + strspn(str.add(end), delimiters));

            if *str == b'\0' as c_char {
                break;
            }
        }
        attr
    }
}
