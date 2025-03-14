use compat_rs::strlcat;
use libc::{strcasecmp, strcmp};

use crate::*;

unsafe extern "C" {
    // pub fn tty_add_features(_: *mut c_int, _: *const c_char, _: *const c_char);
    // pub fn tty_get_features(_: c_int) -> *const c_char;
    // pub fn tty_apply_features(_: *mut tty_term, _: c_int) -> c_int;
    // pub fn tty_default_features(_: *mut c_int, _: *const c_char, _: c_uint);
}

unsafe impl Sync for tty_feature {}
#[repr(C)]
struct tty_feature {
    name: SyncCharPtr,
    capabilities: *const SyncCharPtr,
    flags: i32,
}
impl tty_feature {
    const fn new(name: SyncCharPtr, capabilities: &'static [SyncCharPtr], flags: i32) -> Self {
        Self {
            name,
            capabilities: capabilities.as_ptr(),
            flags,
        }
    }
}

unsafe impl Sync for SyncCharPtr {}
#[repr(transparent)]
#[derive(Copy, Clone)]
struct SyncCharPtr(*const c_char);
impl SyncCharPtr {
    const fn new(value: &'static CStr) -> Self { Self(value.as_ptr()) }
    const fn null() -> Self { Self(null()) }
    const fn as_ptr(&self) -> *const c_char { self.0 }
}

static tty_feature_title_capabilities: &[SyncCharPtr] = &[
    SyncCharPtr::new(c"tsl=\\E]0;"), /* should be using TS really */
    SyncCharPtr::new(c"fsl=\\a"),
    SyncCharPtr::null(),
];
static tty_feature_title: tty_feature = tty_feature::new(SyncCharPtr::new(c"title"), tty_feature_title_capabilities, 0);

/// Terminal has OSC 7 working directory.
static tty_feature_osc7_capabilities: &[SyncCharPtr] = &[
    SyncCharPtr::new(c"Swd=\\E]7;"),
    SyncCharPtr::new(c"fsl=\\a"),
    SyncCharPtr::null(),
];
static tty_feature_osc7: tty_feature = tty_feature::new(SyncCharPtr::new(c"osc7"), tty_feature_osc7_capabilities, 0);

/// Terminal has mouse support.
static tty_feature_mouse_capabilities: &[SyncCharPtr] = &[SyncCharPtr::new(c"kmous=\\E[M"), SyncCharPtr::null()];
static tty_feature_mouse: tty_feature = tty_feature::new(SyncCharPtr::new(c"mouse"), tty_feature_mouse_capabilities, 0);

/// Terminal can set the clipboard with OSC 52.
static tty_feature_clipboard_capabilities: &[SyncCharPtr] =
    &[SyncCharPtr::new(c"Ms=\\E]52;%p1%s;%p2%s\\a"), SyncCharPtr::null()];
static tty_feature_clipboard: tty_feature =
    tty_feature::new(SyncCharPtr::new(c"clipboard"), tty_feature_clipboard_capabilities, 0);

// #if defined (__OpenBSD__) || (defined(NCURSES_VERSION_MAJOR) && (NCURSES_VERSION_MAJOR > 5 ||  (NCURSES_VERSION_MAJOR == 5 && NCURSES_VERSION_MINOR > 8)))

/// Terminal supports OSC 8 hyperlinks.
#[cfg(feature = "hyperlinks")]
static tty_feature_hyperlinks_capabilities: &[SyncCharPtr] = &[
    SyncCharPtr::new(c"*:Hls=\\E]8;%?%p1%l%tid=%p1%s%;;%p2%s\\E\\\\"),
    SyncCharPtr::null(),
];
#[cfg(not(feature = "hyperlinks"))]
static tty_feature_hyperlinks_capabilities: &[SyncCharPtr] = &[SyncCharPtr::null()];
static tty_feature_hyperlinks: tty_feature =
    tty_feature::new(SyncCharPtr::new(c"hyperlinks"), tty_feature_hyperlinks_capabilities, 0);

/// Terminal supports RGB colour. This replaces setab and setaf also since
/// terminals with RGB have versions that do not allow setting colours from the
/// 256 palette.
static tty_feature_rgb_capabilities: &[SyncCharPtr] = &[
    SyncCharPtr::new(c"AX"),
    SyncCharPtr::new(c"setrgbf=\\E[38;2;%p1%d;%p2%d;%p3%dm"),
    SyncCharPtr::new(c"setrgbb=\\E[48;2;%p1%d;%p2%d;%p3%dm"),
    SyncCharPtr::new(c"setab=\\E[%?%p1%{8}%<%t4%p1%d%e%p1%{16}%<%t10%p1%{8}%-%d%e48;5;%p1%d%;m"),
    SyncCharPtr::new(c"setaf=\\E[%?%p1%{8}%<%t3%p1%d%e%p1%{16}%<%t9%p1%{8}%-%d%e38;5;%p1%d%;m"),
    SyncCharPtr::null(),
];
static tty_feature_rgb: tty_feature = tty_feature::new(
    SyncCharPtr::new(c"RGB"),
    tty_feature_rgb_capabilities,
    TERM_256COLOURS | TERM_RGBCOLOURS,
);

/// Terminal supports 256 colours.
static tty_feature_256_capabilities: &[SyncCharPtr] = &[
    SyncCharPtr::new(c"AX"),
    SyncCharPtr::new(c"setab=\\E[%?%p1%{8}%<%t4%p1%d%e%p1%{16}%<%t10%p1%{8}%-%d%e48;5;%p1%d%;m"),
    SyncCharPtr::new(c"setaf=\\E[%?%p1%{8}%<%t3%p1%d%e%p1%{16}%<%t9%p1%{8}%-%d%e38;5;%p1%d%;m"),
    SyncCharPtr::null(),
];
static tty_feature_256: tty_feature =
    tty_feature::new(SyncCharPtr::new(c"256"), tty_feature_256_capabilities, TERM_256COLOURS);

/// Terminal supports overline.
static tty_feature_overline_capabilities: &[SyncCharPtr] = &[SyncCharPtr::new(c"Smol=\\E[53m"), SyncCharPtr::null()];
static tty_feature_overline: tty_feature =
    tty_feature::new(SyncCharPtr::new(c"overline"), tty_feature_overline_capabilities, 0);

/// Terminal supports underscore styles.
static tty_feature_usstyle_capabilities: &[SyncCharPtr] = &[
    SyncCharPtr::new(c"Smulx=\\E[4::%p1%dm"),
    SyncCharPtr::new(c"Setulc=\\E[58::2::%p1%{65536}%/%d::%p1%{256}%/%{255}%&%d::%p1%{255}%&%d%;m"),
    SyncCharPtr::new(c"Setulc1=\\E[58::5::%p1%dm"),
    SyncCharPtr::new(c"ol=\\E[59m"),
    SyncCharPtr::null(),
];
static tty_feature_usstyle: tty_feature =
    tty_feature::new(SyncCharPtr::new(c"usstyle"), tty_feature_usstyle_capabilities, 0);

/// Terminal supports bracketed paste.
static tty_feature_bpaste_capabilities: &[SyncCharPtr] = &[
    SyncCharPtr::new(c"Enbp=\\E[?2004h"),
    SyncCharPtr::new(c"Dsbp=\\E[?2004l"),
    SyncCharPtr::null(),
];
static tty_feature_bpaste: tty_feature =
    tty_feature::new(SyncCharPtr::new(c"bpaste"), tty_feature_bpaste_capabilities, 0);

/// Terminal supports focus reporting.
static tty_feature_focus_capabilities: &[SyncCharPtr] = &[
    SyncCharPtr::new(c"Enfcs=\\E[?1004h"),
    SyncCharPtr::new(c"Dsfcs=\\E[?1004l"),
    SyncCharPtr::null(),
];
static tty_feature_focus: tty_feature = tty_feature::new(SyncCharPtr::new(c"focus"), tty_feature_focus_capabilities, 0);

/// Terminal supports cursor styles.
static tty_feature_cstyle_capabilities: &[SyncCharPtr] = &[
    SyncCharPtr::new(c"Ss=\\E[%p1%d q"),
    SyncCharPtr::new(c"Se=\\E[2 q"),
    SyncCharPtr::null(),
];
static tty_feature_cstyle: tty_feature =
    tty_feature::new(SyncCharPtr::new(c"cstyle"), tty_feature_cstyle_capabilities, 0);

/// Terminal supports cursor colours.
static tty_feature_ccolour_capabilities: &[SyncCharPtr] = &[
    SyncCharPtr::new(c"Cs=\\E]12;%p1%s\\a"),
    SyncCharPtr::new(c"Cr=\\E]112\\a"),
    SyncCharPtr::null(),
];
static tty_feature_ccolour: tty_feature =
    tty_feature::new(SyncCharPtr::new(c"ccolour"), tty_feature_ccolour_capabilities, 0);

/// Terminal supports strikethrough.
static tty_feature_strikethrough_capabilities: &[SyncCharPtr] =
    &[SyncCharPtr::new(c"smxx=\\E[9m"), SyncCharPtr::null()];
static tty_feature_strikethrough: tty_feature = tty_feature::new(
    SyncCharPtr::new(c"strikethrough"),
    tty_feature_strikethrough_capabilities,
    0,
);

/// Terminal supports synchronized updates.
static tty_feature_sync_capabilities: &[SyncCharPtr] = &[
    SyncCharPtr::new(c"Sync=\\E[?2026%?%p1%{1}%-%tl%eh%;"),
    SyncCharPtr::null(),
];
static tty_feature_sync: tty_feature = tty_feature::new(SyncCharPtr::new(c"sync"), tty_feature_sync_capabilities, 0);

/// Terminal supports extended keys.
static tty_feature_extkeys_capabilities: &[SyncCharPtr] = &[
    SyncCharPtr::new(c"Eneks=\\E[>4;2m"),
    SyncCharPtr::new(c"Dseks=\\E[>4m"),
    SyncCharPtr::null(),
];
static tty_feature_extkeys: tty_feature =
    tty_feature::new(SyncCharPtr::new(c"extkeys"), tty_feature_extkeys_capabilities, 0);

/// Terminal supports DECSLRM margins.
static tty_feature_margins_capabilities: &[SyncCharPtr] = &[
    SyncCharPtr::new(c"Enmg=\\E[?69h"),
    SyncCharPtr::new(c"Dsmg=\\E[?69l"),
    SyncCharPtr::new(c"Clmg=\\E[s"),
    SyncCharPtr::new(c"Cmg=\\E[%i%p1%d;%p2%ds"),
    SyncCharPtr::null(),
];
static tty_feature_margins: tty_feature = tty_feature::new(
    SyncCharPtr::new(c"margins"),
    tty_feature_margins_capabilities,
    TERM_DECSLRM,
);

/// Terminal supports DECFRA rectangle fill.
static tty_feature_rectfill_capabilities: &[SyncCharPtr] = &[SyncCharPtr::new(c"Rect"), SyncCharPtr::null()];
static tty_feature_rectfill: tty_feature = tty_feature::new(
    SyncCharPtr::new(c"rectfill"),
    tty_feature_rectfill_capabilities,
    TERM_DECFRA,
);

/// Use builtin function keys only.
static tty_feature_ignorefkeys_capabilities: &[SyncCharPtr] = &[
    SyncCharPtr::new(c"kf0@"),
    SyncCharPtr::new(c"kf1@"),
    SyncCharPtr::new(c"kf2@"),
    SyncCharPtr::new(c"kf3@"),
    SyncCharPtr::new(c"kf4@"),
    SyncCharPtr::new(c"kf5@"),
    SyncCharPtr::new(c"kf6@"),
    SyncCharPtr::new(c"kf7@"),
    SyncCharPtr::new(c"kf8@"),
    SyncCharPtr::new(c"kf9@"),
    SyncCharPtr::new(c"kf10@"),
    SyncCharPtr::new(c"kf11@"),
    SyncCharPtr::new(c"kf12@"),
    SyncCharPtr::new(c"kf13@"),
    SyncCharPtr::new(c"kf14@"),
    SyncCharPtr::new(c"kf15@"),
    SyncCharPtr::new(c"kf16@"),
    SyncCharPtr::new(c"kf17@"),
    SyncCharPtr::new(c"kf18@"),
    SyncCharPtr::new(c"kf19@"),
    SyncCharPtr::new(c"kf20@"),
    SyncCharPtr::new(c"kf21@"),
    SyncCharPtr::new(c"kf22@"),
    SyncCharPtr::new(c"kf23@"),
    SyncCharPtr::new(c"kf24@"),
    SyncCharPtr::new(c"kf25@"),
    SyncCharPtr::new(c"kf26@"),
    SyncCharPtr::new(c"kf27@"),
    SyncCharPtr::new(c"kf28@"),
    SyncCharPtr::new(c"kf29@"),
    SyncCharPtr::new(c"kf30@"),
    SyncCharPtr::new(c"kf31@"),
    SyncCharPtr::new(c"kf32@"),
    SyncCharPtr::new(c"kf33@"),
    SyncCharPtr::new(c"kf34@"),
    SyncCharPtr::new(c"kf35@"),
    SyncCharPtr::new(c"kf36@"),
    SyncCharPtr::new(c"kf37@"),
    SyncCharPtr::new(c"kf38@"),
    SyncCharPtr::new(c"kf39@"),
    SyncCharPtr::new(c"kf40@"),
    SyncCharPtr::new(c"kf41@"),
    SyncCharPtr::new(c"kf42@"),
    SyncCharPtr::new(c"kf43@"),
    SyncCharPtr::new(c"kf44@"),
    SyncCharPtr::new(c"kf45@"),
    SyncCharPtr::new(c"kf46@"),
    SyncCharPtr::new(c"kf47@"),
    SyncCharPtr::new(c"kf48@"),
    SyncCharPtr::new(c"kf49@"),
    SyncCharPtr::new(c"kf50@"),
    SyncCharPtr::new(c"kf51@"),
    SyncCharPtr::new(c"kf52@"),
    SyncCharPtr::new(c"kf53@"),
    SyncCharPtr::new(c"kf54@"),
    SyncCharPtr::new(c"kf55@"),
    SyncCharPtr::new(c"kf56@"),
    SyncCharPtr::new(c"kf57@"),
    SyncCharPtr::new(c"kf58@"),
    SyncCharPtr::new(c"kf59@"),
    SyncCharPtr::new(c"kf60@"),
    SyncCharPtr::new(c"kf61@"),
    SyncCharPtr::new(c"kf62@"),
    SyncCharPtr::new(c"kf63@"),
    SyncCharPtr::null(),
];

static tty_feature_ignorefkeys: tty_feature = tty_feature::new(
    SyncCharPtr::new(c"ignorefkeys"),
    tty_feature_ignorefkeys_capabilities,
    0,
);

/// Terminal has sixel capability.
static tty_feature_sixel_capabilities: &[SyncCharPtr] = &[SyncCharPtr::new(c"Sxl"), SyncCharPtr::null()];
static tty_feature_sixel: tty_feature =
    tty_feature::new(SyncCharPtr::new(c"sixel"), tty_feature_sixel_capabilities, TERM_SIXEL);

/// Available terminal features.
static tty_features: [&tty_feature; 20] = [
    &tty_feature_256,
    &tty_feature_bpaste,
    &tty_feature_ccolour,
    &tty_feature_clipboard,
    &tty_feature_hyperlinks,
    &tty_feature_cstyle,
    &tty_feature_extkeys,
    &tty_feature_focus,
    &tty_feature_ignorefkeys,
    &tty_feature_margins,
    &tty_feature_mouse,
    &tty_feature_osc7,
    &tty_feature_overline,
    &tty_feature_rectfill,
    &tty_feature_rgb,
    &tty_feature_sixel,
    &tty_feature_strikethrough,
    &tty_feature_sync,
    &tty_feature_title,
    &tty_feature_usstyle,
];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tty_add_features(feat: *mut i32, s: *const c_char, separators: *const c_char) {
    unsafe {
        log_debug(c"adding terminal features %s".as_ptr(), s);

        let copy = xstrdup(s).as_ptr();
        let loop_ = copy;
        let mut next = null_mut();
        let mut tf: *const tty_feature = null();
        let mut loop_ = null_mut();

        while ({
            next = strsep(&raw mut loop_, separators);
            !next.is_null()
        }) {
            let mut i = 0;
            for j in 0..tty_features.len() {
                i = j;
                tf = tty_features[i] as *const _;
                if (strcasecmp((*tf).name.as_ptr(), next) == 0) {
                    break;
                }
            }
            if (i == tty_features.len()) {
                log_debug(c"unknown terminal feature: %s".as_ptr(), next);
                break;
            }
            if !(*feat) & (1 << i) != 0 {
                log_debug(c"adding terminal feature: %s".as_ptr(), (*tf).name);
                (*feat) |= (1 << i);
            }
        }
        free_(copy);
    }
}

#[warn(static_mut_refs)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tty_get_features(feat: i32) -> *const c_char {
    static mut s_buf: [MaybeUninit<c_char>; 512] = [MaybeUninit::uninit(); 512];
    unsafe {
        let s: *mut c_char = s_buf.as_mut_ptr().cast();
        // const struct tty_feature *tf;

        *s = b'\0' as c_char;
        for i in 0..tty_features.len() {
            if (!feat & (1 << i)) != 0 {
                continue;
            }
            let tf = tty_features[i];

            strlcat(s, (*tf).name.as_ptr(), 512);
            strlcat(s, c",".as_ptr(), 512);
        }
        if (*s != b'\0' as c_char) {
            *s.add(strlen(s) - 1) = b'\0' as c_char;
        }

        s
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tty_apply_features(term: *mut tty_term, feat: i32) -> i32 {
    if (feat == 0) {
        return 0;
    }

    unsafe {
        log_debug(c"applying terminal features: %s".as_ptr(), tty_get_features(feat));

        for i in 0..tty_features.len() {
            if (((*term).features & (1 << i) != 9) || (!feat & (1 << i)) != 0) {
                continue;
            }
            let tf = tty_features[i];

            log_debug(c"applying terminal feature: %s".as_ptr(), (*tf).name);
            if (!(*tf).capabilities.is_null()) {
                let mut capability = (*tf).capabilities;
                while (!(*capability).as_ptr().is_null()) {
                    log_debug(c"adding capability: %s".as_ptr(), *capability);
                    tty_term_apply(term, (*capability).as_ptr(), 1);
                    capability = capability.add(1);
                }
            }
            (*term).flags |= (*tf).flags;
        }
        if (((*term).features | feat) == (*term).features) {
            return 0;
        }
        (*term).features |= feat;
    }

    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tty_default_features(feat: *mut i32, name: *const c_char, version: u32) {
    struct entry {
        name: &'static CStr,
        version: u32,
        features: &'static str,
    }
    macro_rules! TTY_FEATURES_BASE_MODERN_XTERM {
        () => {
            "256,RGB,bpaste,clipboard,mouse,strikethrough,title"
        };
    }

    // TODO note version isn't init in the C code
    #[rustfmt::skip]
    static table: &[entry] = &[
        entry { name: c"mintty", features: concat!( TTY_FEATURES_BASE_MODERN_XTERM!(), ",ccolour,cstyle,extkeys,margins,overline,usstyle\0"), version: 0, },
        entry { name: c"tmux", features: concat!( TTY_FEATURES_BASE_MODERN_XTERM!(), ",ccolour,cstyle,focus,overline,usstyle,hyperlinks\0"), version: 0, },
        entry { name: c"rxvt-unicode", features: "256,bpaste,ccolour,cstyle,mouse,title,ignorefkeys\0", version: 0, },
        entry { name: c"iTerm2", features: concat!( TTY_FEATURES_BASE_MODERN_XTERM!(), ",cstyle,extkeys,margins,usstyle,sync,osc7,hyperlinks\0"), version: 0, },
        // xterm also supports DECSLRM and DECFRA, but they can be
        // disabled so not set it here - they will be added if
        // secondary DA shows VT420.
        entry { name: c"XTerm", features: concat!(TTY_FEATURES_BASE_MODERN_XTERM!(), ",ccolour,cstyle,extkeys,focus\0"), version: 0, },
    ];

    unsafe {
        for e in table {
            if strcmp(e.name.as_ptr(), name) != 0 {
                continue;
            }
            if version != 0 && version < e.version {
                continue;
            }
            tty_add_features(feat, e.features.as_ptr().cast(), c",".as_ptr());
        }
    }
}
