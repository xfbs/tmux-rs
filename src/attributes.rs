// Copyright (c) 2009 Joshua Elsasser <josh@elsasser.org>
//
// Permission to use, copy, modify, and distribute this software for any
// purpose with or without fee is hereby granted, provided that the above
// copyright notice and this permission notice appear in all copies.
//
// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
// WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
// MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
// ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
// WHATSOEVER RESULTING FROM LOSS OF MIND, USE, DATA OR PROFITS, WHETHER
// IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING
// OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
use super::*;

pub unsafe fn attributes_tostring(attr: grid_attr) -> *const c_char {
    type buffer = [c_char; 512];
    static mut buf: buffer = unsafe { zeroed() };

    if attr.is_empty() {
        return c"none".as_ptr();
    }

    unsafe {
        #[rustfmt::skip]
        let len: isize = xsnprintf(
            &raw mut buf as _,
            size_of::<buffer>(),
            c"%s%s%s%s%s%s%s%s%s%s%s%s%s%s".as_ptr(),
            if attr.intersects(grid_attr::GRID_ATTR_CHARSET) { c"acs,".as_ptr() } else { c"".as_ptr() },
            if attr.intersects(grid_attr::GRID_ATTR_BRIGHT) { c"bright,".as_ptr() } else { c"".as_ptr() },
            if attr.intersects(grid_attr::GRID_ATTR_DIM ) { c"dim,".as_ptr() } else { c"".as_ptr() },
            if attr.intersects(grid_attr::GRID_ATTR_UNDERSCORE) { c"underscore,".as_ptr() } else { c"".as_ptr() },
            if attr.intersects(grid_attr::GRID_ATTR_BLINK) { c"blink,".as_ptr() } else { c"".as_ptr() },
            if attr.intersects(grid_attr::GRID_ATTR_REVERSE ) { c"reverse,".as_ptr() } else { c"".as_ptr() },
            if attr.intersects(grid_attr::GRID_ATTR_HIDDEN) { c"hidden,".as_ptr() } else { c"".as_ptr() },
            if attr.intersects(grid_attr::GRID_ATTR_ITALICS ) { c"italics,".as_ptr() } else { c"".as_ptr() },
            if attr.intersects(grid_attr::GRID_ATTR_STRIKETHROUGH) { c"strikethrough,".as_ptr() } else { c"".as_ptr() },
            if attr.intersects(grid_attr::GRID_ATTR_UNDERSCORE_2) { c"double-underscore,".as_ptr() } else { c"".as_ptr() },
            if attr.intersects(grid_attr::GRID_ATTR_UNDERSCORE_3) { c"curly-underscore,".as_ptr() } else { c"".as_ptr() },
            if attr.intersects(grid_attr::GRID_ATTR_UNDERSCORE_4) { c"dotted-underscore,".as_ptr() } else { c"".as_ptr() },
            if attr.intersects(grid_attr::GRID_ATTR_UNDERSCORE_5) { c"dashed-underscore,".as_ptr() } else { c"".as_ptr() },
            if attr.intersects(grid_attr::GRID_ATTR_OVERLINE) { c"overline,".as_ptr() } else { c"".as_ptr() },
        ) as isize;
        if len > 0 {
            buf[len as usize - 1] = b'\0' as c_char;
        }

        &raw mut buf as _
    }
}

#[allow(clippy::result_unit_err)]
pub unsafe fn attributes_fromstring(mut str: *const c_char) -> Result<grid_attr, ()> {
    struct table_entry {
        name: &'static CStr,
        attr: grid_attr,
    }
    #[rustfmt::skip]
    const TABLE: [table_entry; 15] = [
        table_entry { name: c"acs", attr: grid_attr::GRID_ATTR_CHARSET, },
        table_entry { name: c"bright", attr: grid_attr::GRID_ATTR_BRIGHT, },
        table_entry { name: c"bold", attr: grid_attr::GRID_ATTR_BRIGHT, },
        table_entry { name: c"dim", attr: grid_attr::GRID_ATTR_DIM, },
        table_entry { name: c"underscore", attr: grid_attr::GRID_ATTR_UNDERSCORE, },
        table_entry { name: c"blink", attr: grid_attr::GRID_ATTR_BLINK, },
        table_entry { name: c"reverse", attr: grid_attr::GRID_ATTR_REVERSE, },
        table_entry { name: c"hidden", attr: grid_attr::GRID_ATTR_HIDDEN, },
        table_entry { name: c"italics", attr: grid_attr::GRID_ATTR_ITALICS, },
        table_entry { name: c"strikethrough", attr: grid_attr::GRID_ATTR_STRIKETHROUGH, },
        table_entry { name: c"double-underscore", attr: grid_attr::GRID_ATTR_UNDERSCORE_2, },
        table_entry { name: c"curly-underscore", attr: grid_attr::GRID_ATTR_UNDERSCORE_3, },
        table_entry { name: c"dotted-underscore", attr: grid_attr::GRID_ATTR_UNDERSCORE_4, },
        table_entry { name: c"dashed-underscore", attr: grid_attr::GRID_ATTR_UNDERSCORE_5, },
        table_entry { name: c"overline", attr: grid_attr::GRID_ATTR_OVERLINE, },
    ];

    let delimiters = c" ,|".as_ptr();

    unsafe {
        if *str == b'\0' as c_char || libc::strcspn(str, delimiters) == 0 {
            return Err(());
        }
        if !libc::strchr(delimiters, *str.add(libc::strlen(str) - 1) as i32).is_null() {
            return Err(());
        }

        if libc::strcasecmp(str, c"default".as_ptr()) == 0
            || libc::strcasecmp(str, c"none".as_ptr()) == 0
        {
            return Ok(grid_attr::empty());
        }

        let mut attr = grid_attr::empty();
        loop {
            let end = libc::strcspn(str, delimiters);
            let mut i = 0;
            for j in 0..TABLE.len() {
                i = j;
                if end != TABLE[i].name.to_bytes().len() {
                    continue;
                }
                if libc::strncasecmp(str, TABLE[i].name.as_ptr(), end) == 0 {
                    attr |= TABLE[i].attr;
                    break;
                }
            }
            if i == TABLE.len() {
                return Err(());
            }
            str = str.add(end + libc::strspn(str.add(end), delimiters));

            if *str == b'\0' as c_char {
                break;
            }
        }
        Ok(attr)
    }
}
