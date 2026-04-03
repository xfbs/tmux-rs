// Copyright (c) 2019 Nicholas Marriott <nicholas.marriott@gmail.com>
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
use std::io::Read as _;
use std::ops::BitAndAssign as _;
use std::ops::BitOrAssign as _;

use crate::xmalloc::xrecallocarray__;
use crate::*;

#[expect(unused_imports)]
#[allow(clippy::all)]
#[allow(clippy::pedantic)]
#[allow(clippy::restriction)]
mod lalrpop {
    use lalrpop_util::lalrpop_mod;
    lalrpop_mod!(pub(crate) cmd_parse);
}
use lalrpop::cmd_parse;

fn yyparse(ps: &mut cmd_parse_state) -> Result<Option<Vec<ParsedCommand>>, ()> {
    let parser = cmd_parse::LinesParser::new();

    let ps = NonNull::new(ps).unwrap();
    let lexer = lexer::Lexer::new(ps);

    match parser.parse(ps, lexer) {
        Ok(cmds) => Ok(cmds),
        Err(parse_err) => {
            log_debug!("parsing error {parse_err:?}");
            Err(())
        }
    }
}

// --- New clean types for parser output ---

pub struct ParsedCommand {
    pub line: u32,
    pub arguments: Vec<ParsedArgument>,
}

pub enum ParsedArgument {
    /// NUL-terminated C string (owned Vec<u8>).
    String(Vec<u8>),
    /// Command block from { ... } braces.
    CommandBlock(Vec<ParsedCommand>),
    /// Pre-parsed command list (reference-counted, from cmd_parse_from_arguments).
    ParsedCommands(*mut cmd_list),
}

impl Drop for ParsedArgument {
    fn drop(&mut self) {
        match self {
            ParsedArgument::String(_) => {} // Vec<u8> drops automatically
            ParsedArgument::CommandBlock(_) => {} // Vec drops recursively
            ParsedArgument::ParsedCommands(cmdlist) => unsafe {
                if !(*cmdlist).is_null() {
                    cmd_list_free(*cmdlist);
                }
            },
        }
    }
}

pub struct ElifResult {
    pub flag: bool,
    pub commands: Vec<ParsedCommand>,
}

pub struct cmd_parse_state<'a> {
    pub f: Option<&'a mut std::io::BufReader<std::fs::File>>,
    pub unget_buf: Option<i32>,

    pub buf: Option<&'a [u8]>,
    pub off: usize,

    pub condition: i32,
    pub eol: i32,
    pub eof: i32,
    pub input: Option<&'a cmd_parse_input<'a>>,
    pub escapes: u32,

    pub error: *mut u8,

    pub scope: Option<bool>,
    pub stack: Vec<bool>,
}

// --- Safe helpers for grammar actions (encapsulate NonNull dereference) ---

/// Check if the current scope is active (no scope, or scope flag is true).
pub fn ps_scope_active(ps: NonNull<cmd_parse_state>) -> bool {
    unsafe { (*ps.as_ptr()).scope.is_none_or(|flag| flag) }
}

/// Get the current line number from the parser input.
pub fn ps_current_line(ps: NonNull<cmd_parse_state>) -> u32 {
    unsafe {
        (*ps.as_ptr())
            .input
            .as_ref()
            .unwrap()
            .line
            .load(atomic::Ordering::SeqCst)
    }
}

/// Push the current scope onto the stack and set a new scope flag.
pub fn ps_push_scope(ps: NonNull<cmd_parse_state>, flag: bool) {
    unsafe {
        let ps = &mut *ps.as_ptr();
        if let Some(current) = ps.scope {
            ps.stack.push(current);
        }
        ps.scope = Some(flag);
    }
}

/// Pop the scope stack, restoring the previous scope.
pub fn ps_pop_scope(ps: NonNull<cmd_parse_state>) {
    unsafe {
        (*ps.as_ptr()).scope = (*ps.as_ptr()).stack.pop();
    }
}

/// Replace the current scope flag (for %else and %elif).
pub fn ps_set_scope(ps: NonNull<cmd_parse_state>, flag: bool) {
    unsafe {
        (*ps.as_ptr()).scope = Some(flag);
    }
}

/// Get the current scope flag (for %else inversion).
pub fn ps_scope_flag(ps: NonNull<cmd_parse_state>) -> bool {
    unsafe { (*ps.as_ptr()).scope.unwrap() }
}

/// Convert a *mut u8 C string to an owned Vec<u8> (NUL-terminated). Frees the original.
pub unsafe fn cstr_to_owned_vec(ptr: *mut u8) -> Vec<u8> {
    unsafe {
        let len = strlen(ptr) + 1; // include NUL
        let vec = std::slice::from_raw_parts(ptr, len).to_vec();
        free_(ptr);
        vec
    }
}

pub unsafe fn cmd_parse_get_error(file: Option<&str>, line: u32, error: &str) -> CString {
    match file {
        None => CString::new(error).unwrap(),
        Some(file) => CString::new(format!("{file}:{line}: {error}")).unwrap(),
    }
}

pub fn cmd_parse_print_commands(pi: &cmd_parse_input, cmdlist: &cmd_list) {
    if pi.item.is_null()
        || !pi
            .flags
            .intersects(cmd_parse_input_flags::CMD_PARSE_VERBOSE)
    {
        return;
    }

    let s = cmd_list_print(cmdlist, 0);

    unsafe {
        if let Some(file) = pi.file {
            cmdq_print!(
                pi.item,
                "{}:{}: {}",
                file,
                pi.line.load(atomic::Ordering::SeqCst),
                _s(s)
            );
        } else {
            cmdq_print!(
                pi.item,
                "{}: {}",
                pi.line.load(atomic::Ordering::SeqCst),
                _s(s)
            );
        }
        free_(s);
    }
}

pub unsafe fn cmd_parse_run_parser(
    ps: &mut cmd_parse_state,
) -> Result<Vec<ParsedCommand>, CString> {
    unsafe {
        let retval = yyparse(ps);
        ps.stack.clear();

        match retval {
            Ok(Some(cmds)) => Ok(cmds),
            Ok(None) => Ok(Vec::new()),
            Err(()) => {
                if ps.error.is_null() {
                    let pi = ps.input.as_ref().unwrap();
                    Err(cmd_parse_get_error(
                        pi.file,
                        pi.line.load(atomic::Ordering::SeqCst),
                        "syntax error",
                    ))
                } else {
                    Err(CString::from_raw(ps.error.cast()))
                }
            }
        }
    }
}

fn new_cmd_parse_state<'a>() -> Box<cmd_parse_state<'a>> {
    Box::new(cmd_parse_state {
        f: None,
        unget_buf: None,
        buf: None,
        off: 0,
        condition: 0,
        eol: 0,
        eof: 0,
        input: None,
        escapes: 0,
        error: null_mut(),
        scope: None,
        stack: Vec::new(),
    })
}

pub unsafe fn cmd_parse_do_file<'a>(
    f: &'a mut std::io::BufReader<std::fs::File>,
    pi: &'a cmd_parse_input<'a>,
) -> Result<Vec<ParsedCommand>, CString> {
    unsafe {
        let mut ps = new_cmd_parse_state();
        ps.input = Some(pi);
        ps.f = Some(f);
        cmd_parse_run_parser(&mut ps)
    }
}

pub unsafe fn cmd_parse_do_buffer<'a>(
    buf: &'a [u8],
    pi: &'a cmd_parse_input<'a>,
) -> Result<Vec<ParsedCommand>, CString> {
    unsafe {
        let mut ps = new_cmd_parse_state();
        ps.input = Some(pi);
        ps.buf = Some(buf);
        cmd_parse_run_parser(&mut ps)
    }
}

pub fn cmd_parse_log_commands(cmds: &[ParsedCommand], prefix: &str) {
    unsafe {
        for (i, cmd) in cmds.iter().enumerate() {
            for (j, arg) in cmd.arguments.iter().enumerate() {
                match arg {
                    ParsedArgument::String(string) => {
                        log_debug!("{} {}:{}: {}", prefix, i, j, _s(string.as_ptr()));
                    }
                    ParsedArgument::CommandBlock(commands) => {
                        let sub = format!("{} {}:{}", prefix, i, j);
                        cmd_parse_log_commands(commands, &sub);
                    }
                    ParsedArgument::ParsedCommands(cmdlist) => {
                        let s = cmd_list_print(&**cmdlist, 0);
                        log_debug!("{} {}:{}: {}", prefix, i, j, _s(s));
                        free_(s);
                    }
                }
            }
        }
    }
}

pub unsafe fn cmd_parse_expand_alias<'a>(
    cmd: &mut ParsedCommand,
    pi: &'a cmd_parse_input<'a>,
    pr: &mut cmd_parse_result,
) -> bool {
    let __func__ = "cmd_parse_expand_alias";
    unsafe {
        if pi
            .flags
            .intersects(cmd_parse_input_flags::CMD_PARSE_NOALIAS)
        {
            return false;
        }
        *pr = Err(CString::default());

        if cmd.arguments.is_empty() {
            *pr = Ok(cmd_list_new());
            return true;
        }
        let ParsedArgument::String(name) = &cmd.arguments[0] else {
            *pr = Ok(cmd_list_new());
            return true;
        };

        let alias = cmd_get_alias(name.as_ptr());
        if alias.is_null() {
            return false;
        }
        log_debug!(
            "{}: {} alias {} = {}",
            __func__,
            pi.line.load(atomic::Ordering::SeqCst),
            _s(name.as_ptr()),
            _s(alias)
        );

        let result = cmd_parse_do_buffer(
            std::slice::from_raw_parts(alias.cast(), libc::strlen(alias)),
            pi,
        );
        free_(alias);
        let mut cmds = match result {
            Ok(cmds) => cmds,
            Err(cause) => {
                *pr = Err(cause);
                return true;
            }
        };

        if cmds.is_empty() {
            *pr = Ok(cmd_list_new());
            return true;
        }

        // Remove the alias name (first argument) and append remaining
        // arguments to the last expanded command.
        let remaining: Vec<ParsedArgument> = cmd.arguments.drain(1..).collect();
        if let Some(last) = cmds.last_mut() {
            last.arguments.extend(remaining);
        }
        cmd_parse_log_commands(&cmds, __func__);

        (&pi.flags).bitor_assign(cmd_parse_input_flags::CMD_PARSE_NOALIAS);
        cmd_parse_build_commands(&cmds, pi, pr);
        (&pi.flags).bitand_assign(!cmd_parse_input_flags::CMD_PARSE_NOALIAS);
        true
    }
}

pub unsafe fn cmd_parse_build_command(
    cmd: &ParsedCommand,
    pi: &cmd_parse_input,
    pr: &mut cmd_parse_result,
) {
    unsafe {
        let mut values: *mut args_value = null_mut();
        let mut count: u32 = 0;
        *pr = cmd_parse_result::Err(CString::default());

        // Check alias first — needs a mutable copy of arguments
        {
            let mut alias_cmd = ParsedCommand {
                line: cmd.line,
                arguments: cmd.arguments.iter().map(|arg| match arg {
                    ParsedArgument::String(s) => ParsedArgument::String(s.clone()),
                    ParsedArgument::CommandBlock(_) => {
                        // Can't cheaply clone command blocks for alias check.
                        // Aliases only match on first string arg, so this path
                        // won't be reached during alias lookup.
                        ParsedArgument::CommandBlock(Vec::new())
                    }
                    ParsedArgument::ParsedCommands(cmdlist) => {
                        (**cmdlist).references += 1;
                        ParsedArgument::ParsedCommands(*cmdlist)
                    }
                }).collect(),
            };
            if cmd_parse_expand_alias(&mut alias_cmd, pi, pr) {
                return;
            }
            // Alias expansion didn't happen — drop the copy
        }

        'out: {
            for arg in cmd.arguments.iter() {
                values = xrecallocarray__::<args_value>(values, count as usize, count as usize + 1)
                    .as_ptr();
                match arg {
                    ParsedArgument::String(string) => {
                        (*values.add(count as usize)).type_ = args_type::ARGS_STRING;
                        (*values.add(count as usize)).union_.string = xstrdup(string.as_ptr()).as_ptr();
                    }
                    ParsedArgument::CommandBlock(commands) => {
                        cmd_parse_build_commands(commands, pi, pr);
                        match *pr {
                            Err(_) => break 'out,
                            Ok(cmdlist) => {
                                (*values.add(count as _)).type_ = args_type::ARGS_COMMANDS;
                                (*values.add(count as _)).union_.cmdlist = cmdlist;
                            }
                        }
                    }
                    ParsedArgument::ParsedCommands(cmdlist) => {
                        (*values.add(count as _)).type_ = args_type::ARGS_COMMANDS;
                        (*values.add(count as _)).union_.cmdlist = *cmdlist;
                        (*(*values.add(count as _)).union_.cmdlist).references += 1;
                    }
                }
                count += 1;
            }

            match cmd_parse(
                values,
                count,
                pi.file,
                pi.line.load(atomic::Ordering::SeqCst),
            ) {
                Ok(add) => {
                    let cmdlist = cmd_list_new();
                    *pr = Ok(cmdlist);
                    cmd_list_append(cmdlist, add);
                }
                Err(cause) => {
                    *pr = Err(cmd_parse_get_error(
                        pi.file,
                        pi.line.load(atomic::Ordering::SeqCst),
                        &cause,
                    ));
                    break 'out;
                }
            }
        }
        // out:
        for idx in 0..count {
            args_free_value(values.add(idx as usize));
        }
        free_(values);
    }
}

pub unsafe fn cmd_parse_build_commands(
    cmds: &[ParsedCommand],
    pi: &cmd_parse_input,
    pr: &mut cmd_parse_result,
) {
    unsafe {
        let mut line = u32::MAX;
        let mut current: *mut cmd_list = null_mut();

        *pr = Err(CString::default());

        if cmds.is_empty() {
            *pr = Ok(cmd_list_new());
            return;
        }
        cmd_parse_log_commands(cmds, "cmd_parse_build_commands");

        let result = cmd_list_new();
        for cmd in cmds.iter() {
            if !pi
                .flags
                .intersects(cmd_parse_input_flags::CMD_PARSE_ONEGROUP)
                && cmd.line != line
            {
                if !current.is_null() {
                    cmd_parse_print_commands(pi, &*current);
                    cmd_list_move(result, current);
                    cmd_list_free(current);
                }
                current = cmd_list_new();
            }
            if current.is_null() {
                current = cmd_list_new();
            }
            line = cmd.line;
            pi.line.store(cmd.line, atomic::Ordering::SeqCst);

            cmd_parse_build_command(cmd, pi, pr);
            match pr {
                Err(_err) => {
                    cmd_list_free(result);
                    cmd_list_free(current);
                    return;
                }
                Ok(cmdlist) => {
                    cmd_list_append_all(current, *cmdlist);
                    cmd_list_free(*cmdlist);
                }
            }
        }

        if !current.is_null() {
            cmd_parse_print_commands(pi, &*current);
            cmd_list_move(result, current);
            cmd_list_free(current);
        }

        let s = cmd_list_print(result, 0);
        log_debug!("cmd_parse_build_commands: {}", _s(s));
        free_(s);

        *pr = Ok(result);
    }
}

pub unsafe fn cmd_parse_from_file<'a>(
    f: &'a mut std::io::BufReader<std::fs::File>,
    pi: Option<&'a cmd_parse_input<'a>>,
) -> cmd_parse_result {
    unsafe {
        let input: cmd_parse_input = zeroed();
        let pi = pi.unwrap_or(&input);

        let cmds = cmd_parse_do_file(f, pi)?;
        let mut pr = Err(CString::default());
        cmd_parse_build_commands(&cmds, pi, &mut pr);
        pr
    }
}

pub unsafe fn cmd_parse_from_string(s: &str, pi: Option<&cmd_parse_input>) -> cmd_parse_result {
    unsafe {
        let input: cmd_parse_input = cmd_parse_input::default();
        let pi = pi.unwrap_or(&input);

        (&pi.flags).bitor_assign(cmd_parse_input_flags::CMD_PARSE_ONEGROUP);
        cmd_parse_from_buffer(s.as_bytes(), Some(pi))
    }
}

pub unsafe fn cmd_parse_and_append(
    s: &str,
    pi: Option<&cmd_parse_input>,
    c: *mut client,
    state: *mut cmdq_state,
    error: *mut *mut u8,
) -> cmd_parse_status {
    unsafe {
        match cmd_parse_from_string(s, pi) {
            Err(err) => {
                if !error.is_null() {
                    *error = err.into_raw().cast();
                }
                cmd_parse_status::CMD_PARSE_ERROR
            }
            Ok(cmdlist) => {
                let item = cmdq_get_command(cmdlist, state);
                cmdq_append(c, item);
                cmd_list_free(cmdlist);
                cmd_parse_status::CMD_PARSE_SUCCESS
            }
        }
    }
}

pub unsafe fn cmd_parse_from_buffer(buf: &[u8], pi: Option<&cmd_parse_input>) -> cmd_parse_result {
    unsafe {
        let input: cmd_parse_input = zeroed();
        let pi = pi.unwrap_or(&input);

        if buf.is_empty() {
            return Ok(cmd_list_new());
        }

        let cmds = cmd_parse_do_buffer(buf, pi)?;
        let mut pr = Err(CString::default());
        cmd_parse_build_commands(&cmds, pi, &mut pr);
        pr
    }
}

pub unsafe fn cmd_parse_from_arguments(
    values: *mut args_value,
    count: u32,
    pi: Option<&mut cmd_parse_input>,
) -> cmd_parse_result {
    unsafe {
        let mut input: cmd_parse_input = zeroed();
        let pi = pi.unwrap_or(&mut input);
        let mut cmds: Vec<ParsedCommand> = Vec::new();

        let mut current = ParsedCommand {
            line: pi.line.load(atomic::Ordering::SeqCst),
            arguments: Vec::new(),
        };

        for i in 0..count {
            let mut end = false;
            if (*values.add(i as usize)).type_ == args_type::ARGS_STRING {
                let src = (*values.add(i as usize)).union_.string;
                let len = strlen(src);
                let mut bytes: Vec<u8> = std::slice::from_raw_parts(src, len).to_vec();
                if !bytes.is_empty() && *bytes.last().unwrap() == b';' {
                    bytes.pop(); // remove ';'
                    if !bytes.is_empty() && *bytes.last().unwrap() == b'\\' {
                        *bytes.last_mut().unwrap() = b';';
                    } else {
                        end = true;
                    }
                }
                bytes.push(b'\0'); // NUL terminate
                if !end || bytes.len() > 1 {
                    current.arguments.push(ParsedArgument::String(bytes));
                }
            } else if (*values.add(i as usize)).type_ == args_type::ARGS_COMMANDS {
                let cmdlist = (*values.add(i as usize)).union_.cmdlist;
                (*cmdlist).references += 1;
                current
                    .arguments
                    .push(ParsedArgument::ParsedCommands(cmdlist));
            } else {
                fatalx("unknown argument type");
            }
            if end {
                cmds.push(current);
                current = ParsedCommand {
                    line: pi.line.load(atomic::Ordering::SeqCst),
                    arguments: Vec::new(),
                };
            }
        }
        if !current.arguments.is_empty() {
            cmds.push(current);
        }

        let mut pr = Err(CString::default());
        cmd_parse_build_commands(&cmds, pi, &mut pr);
        pr
    }
}

mod lexer {
    use crate::cmd_parse_state;

    pub struct Lexer<'a> {
        ps: std::ptr::NonNull<cmd_parse_state<'a>>,
    }
    impl<'a> Lexer<'a> {
        pub fn new(ps: std::ptr::NonNull<cmd_parse_state<'a>>) -> Self {
            Lexer { ps }
        }
    }

    #[derive(Clone, Debug)]
    pub enum Tok {
        Newline,
        Semicolon,
        LeftBrace,
        RightBrace,

        Error,
        Hidden,
        If,
        Else,
        Elif,
        Endif,

        Format(Vec<u8>),
        Token(Vec<u8>),
        Equals(Vec<u8>),
    }
    impl std::fmt::Display for Tok {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Tok::Newline => write!(f, "\\n"),
                Tok::Semicolon => write!(f, ";"),
                Tok::LeftBrace => write!(f, "{{"),
                Tok::RightBrace => write!(f, "}}"),
                Tok::Error => write!(f, "%error"),
                Tok::Hidden => write!(f, "%hidden"),
                Tok::If => write!(f, "%if"),
                Tok::Else => write!(f, "%else"),
                Tok::Elif => write!(f, "%elif"),
                Tok::Endif => write!(f, "%endif"),
                Tok::Format(v) => {
                    write!(f, "format({})", unsafe {
                        crate::_s(v.as_ptr())
                    })
                }
                Tok::Token(v) => write!(f, "token({})", unsafe {
                    crate::_s(v.as_ptr())
                }),
                Tok::Equals(v) => {
                    write!(f, "equals({})", unsafe {
                        crate::_s(v.as_ptr())
                    })
                }
            }
        }
    }

    #[derive(Debug)]
    pub enum LexicalError {
        // Not possible
    }
    type Loc = usize;
    impl Iterator for Lexer<'_> {
        type Item = Result<(Loc, Tok, Loc), LexicalError>;

        fn next(&mut self) -> Option<Result<(Loc, Tok, Loc), LexicalError>> {
            unsafe { super::yylex_(&mut *self.ps.as_ptr()).map(|tok| Ok((0, tok, 0))) }
        }
    }
}

macro_rules! yyerror {
   ($ps:expr, $fmt:literal $(, $args:expr)* $(,)?) => {
        crate::cmd_parse::yyerror_(&mut *$ps, format_args!($fmt $(, $args)*))
    };
}
unsafe fn yyerror_(ps: &mut cmd_parse_state, args: std::fmt::Arguments) -> i32 {
    unsafe {
        if !ps.error.is_null() {
            return 0;
        }

        let pi = ps.input.as_mut().unwrap();

        let error = args.to_string();

        ps.error = cmd_parse_get_error(pi.file, pi.line.load(atomic::Ordering::SeqCst), &error)
            .into_raw()
            .cast();
        0
    }
}

fn yylex_is_var(ch: u8, first: bool) -> bool {
    if ch == b'=' || (first && ch.is_ascii_digit()) {
        false
    } else {
        ch.is_ascii_alphanumeric() || ch == b'_'
    }
}

fn yylex_append(buf: &mut Vec<u8>, add: &[u8]) {
    if add.len() > usize::MAX - 1 || buf.len() > usize::MAX - 1 - add.len() {
        fatalx("buffer is too big");
    }
    buf.extend_from_slice(add);
}

fn yylex_append1(buf: &mut Vec<u8>, add: u8) {
    yylex_append(buf, &[add]);
}

fn yylex_getc1(ps: &mut cmd_parse_state) -> i32 {
    let ch;
    if let Some(f) = ps.f.as_mut() {
        if let Some(c) = ps.unget_buf.take() {
            return c;
        }
        let mut buf: [u8; 1] = [0];
        match f.read(&mut buf) {
            Ok(count) => {
                assert!(count == 0 || count == 1, "unexpected read size");
                if count == 0 {
                    ch = libc::EOF;
                } else {
                    ch = buf[0] as i32;
                }
            }
            Err(_) => {
                ch = libc::EOF;
            }
        }
    } else if ps.off == ps.buf.unwrap().len() {
        ch = libc::EOF;
    } else {
        ch = ps.buf.unwrap()[ps.off] as i32;
        ps.off += 1;
    }

    ch
}

fn yylex_ungetc(ps: &mut cmd_parse_state, ch: i32) {
    if let Some(_f) = ps.f.as_mut() {
        ps.unget_buf = Some(ch);
    } else if ps.off > 0 && ch != libc::EOF {
        ps.off -= 1;
    }
}

fn yylex_getc(ps: &mut cmd_parse_state) -> i32 {
    if ps.escapes != 0 {
        ps.escapes -= 1;
        return '\\' as i32;
    }
    loop {
        let ch = yylex_getc1(ps);
        if ch == '\\' as i32 {
            ps.escapes += 1;
            continue;
        }
        if ch == '\n' as i32 && ps.escapes % 2 == 1 {
            ps.input
                .as_mut()
                .unwrap()
                .line
                .fetch_add(1, atomic::Ordering::SeqCst);
            ps.escapes -= 1;
            continue;
        }

        if ps.escapes != 0 {
            yylex_ungetc(ps, ch);
            ps.escapes -= 1;
            return '\\' as i32;
        }
        return ch;
    }
}

unsafe fn yylex_get_word(ps: &mut cmd_parse_state, mut ch: i32) -> Vec<u8> {
    unsafe {
        let mut buf = Vec::new();

        loop {
            yylex_append1(&mut buf, ch as u8);
            ch = yylex_getc(ps);
            if ch == libc::EOF || !libc::strchr(c!(" \t\n"), ch).is_null() {
                break;
            }
        }
        yylex_ungetc(ps, ch);

        buf.push(b'\0');
        // log_debug("%s: %s", __func__, buf.as_ptr());
        buf
    }
}

use lexer::Tok;

unsafe fn yylex_(ps: &mut cmd_parse_state) -> Option<Tok> {
    unsafe {
        if ps.eol != 0 {
            ps.input
                .as_mut()
                .unwrap()
                .line
                .fetch_add(1, atomic::Ordering::SeqCst);
        }
        ps.eol = 0;

        let condition = ps.condition;
        ps.condition = 0;

        loop {
            let mut ch = yylex_getc(ps);

            if ch == libc::EOF {
                // Ensure every file or string is terminated by a
                // newline. This keeps the parser simpler and avoids
                // having to add a newline to each string.
                if ps.eof != 0 {
                    break;
                }
                ps.eof = 1;
                return Some(Tok::Newline);
            }

            if ch == ' ' as i32 || ch == '\t' as i32 {
                // Ignore whitespace.
                continue;
            }

            if ch == '\r' as i32 {
                // Treat \r\n as \n.
                ch = yylex_getc(ps);
                if ch != '\n' as i32 {
                    yylex_ungetc(ps, ch);
                    ch = '\r' as i32;
                }
            }
            if ch == '\n' as i32 {
                // End of line. Update the line number.
                ps.eol = 1;
                return Some(Tok::Newline);
            }

            if ch == ';' as i32 {
                return Some(Tok::Semicolon);
            }
            if ch == '{' as i32 {
                return Some(Tok::LeftBrace);
            }
            if ch == '}' as i32 {
                return Some(Tok::RightBrace);
            }

            if ch == '#' as i32 {
                // #{ after a condition opens a format; anything else
                // is a comment, ignore up to the end of the line.
                let mut next = yylex_getc(ps);
                if condition != 0 && next == '{' as i32 {
                    match yylex_format(ps) {
                        None => return Some(Tok::Error),
                        Some(yylval_token) => return Some(Tok::Format(yylval_token)),
                    }
                }
                while next != '\n' as i32 && next != libc::EOF {
                    next = yylex_getc(ps);
                }
                if next == '\n' as i32 {
                    ps.input
                        .as_mut()
                        .unwrap()
                        .line
                        .fetch_add(1, atomic::Ordering::SeqCst);
                    return Some(Tok::Newline);
                }
                continue;
            }

            if ch == '%' as i32 {
                // % is a condition unless it is all % or all numbers,
                // then it is a token.
                let yylval_token = yylex_get_word(ps, '%' as i32);
                let all_pct_or_digit = yylval_token.iter()
                    .take_while(|&&b| b != b'\0')
                    .all(|&b| b == b'%' || b.is_ascii_digit());
                if all_pct_or_digit {
                    return Some(Tok::Token(yylval_token));
                }
                ps.condition = 1;
                if yylval_token.as_slice() == b"%hidden\0" {
                    return Some(Tok::Hidden);
                }
                if yylval_token.as_slice() == b"%if\0" {
                    return Some(Tok::If);
                }
                if yylval_token.as_slice() == b"%else\0" {
                    return Some(Tok::Else);
                }
                if yylval_token.as_slice() == b"%elif\0" {
                    return Some(Tok::Elif);
                }
                if yylval_token.as_slice() == b"%endif\0" {
                    return Some(Tok::Endif);
                }
                return Some(Tok::Error);
            }

            // Otherwise this is a token.
            let buf = match yylex_token(ps, ch) {
                None => return Some(Tok::Error),
                Some(buf) => buf,
            };

            if buf.contains(&b'=') && !buf.is_empty() && yylex_is_var(buf[0], true) {
                let mut i = 1;
                while i < buf.len() && buf[i] != b'=' {
                    if !yylex_is_var(buf[i], false) {
                        break;
                    }
                    i += 1;
                }
                if i < buf.len() && buf[i] == b'=' {
                    return Some(Tok::Equals(buf));
                }
            }
            return Some(Tok::Token(buf));
        }

        None
    }
}

unsafe fn yylex_format(ps: &mut cmd_parse_state) -> Option<Vec<u8>> {
    let mut brackets = 1;
    let mut buf = Vec::new();

    'error: {
        yylex_append(&mut buf, b"#{");
        loop {
            let mut ch = yylex_getc(ps);
            if ch == libc::EOF || ch == '\n' as i32 {
                break 'error;
            }
            if ch == '#' as i32 {
                ch = yylex_getc(ps);
                if ch == libc::EOF || ch == '\n' as i32 {
                    break 'error;
                }
                if ch == '{' as i32 {
                    brackets += 1;
                }
                yylex_append1(&mut buf, b'#');
            } else if (ch == '}' as i32)
                && brackets != 0
                && ({
                    brackets -= 1;
                    brackets == 0
                })
            {
                yylex_append1(&mut buf, ch as u8);
                break;
            }
            yylex_append1(&mut buf, ch as u8);
        }
        if brackets != 0 {
            break 'error;
        }

        buf.push(b'\0');
        // log_debug("%s: %s", __func__, buf.as_ptr());
        return Some(buf);
    } // error:

    None
}

unsafe fn yylex_token_variable(ps: &mut cmd_parse_state, buf: &mut Vec<u8>) -> bool {
    unsafe {
        let mut namelen: usize = 0;
        let mut name: [u8; 1024] = [0; 1024];
        const SIZEOF_NAME: usize = 1024;
        let mut brackets = 0;

        let mut ch = yylex_getc(ps);
        if ch == libc::EOF {
            return false;
        }
        if ch == '{' as i32 {
            brackets = 1;
        } else {
            if !yylex_is_var(ch as u8, true) {
                yylex_append1(buf, b'$');
                yylex_ungetc(ps, ch);
                return true;
            }
            name[namelen] = ch as u8;
            namelen += 1;
        }

        loop {
            ch = yylex_getc(ps);
            if brackets != 0 && ch == '}' as i32 {
                break;
            }
            if ch == libc::EOF || !yylex_is_var(ch as u8, false) {
                if brackets == 0 {
                    yylex_ungetc(ps, ch);
                    break;
                }
                yyerror!(ps, "invalid environment variable");
                return false;
            }
            if namelen == SIZEOF_NAME - 2 {
                yyerror!(ps, "environment variable is too long");
                return false;
            }
            name[namelen] = ch as u8;
            namelen += 1;
        }
        name[namelen] = b'\0';

        let envent = environ_find_raw(&*GLOBAL_ENVIRON, (&raw const name).cast());
        if let Some(envent) = envent {
            if let Some(ref value) = envent.value {
                // log_debug("%s: %s -> %s", __func__, name, value);
                yylex_append(buf, value);
            }
        }
        true
    }
}

unsafe fn yylex_token_tilde(ps: &mut cmd_parse_state, buf: &mut Vec<u8>) -> bool {
    unsafe {
        let mut home: *const u8 = null();
        let mut namelen: usize = 0;
        let mut name: [u8; 1024] = [0; 1024];
        const SIZEOF_NAME: usize = 1024;

        loop {
            let ch = yylex_getc(ps);
            if ch == libc::EOF || !libc::strchr(c!("/ \t\n\"'"), ch).is_null() {
                yylex_ungetc(ps, ch);
                break;
            }
            if namelen == SIZEOF_NAME - 2 {
                yyerror!(ps, "user name is too long");
                return false;
            }
            name[namelen] = ch as u8;
            namelen += 1;
        }
        name[namelen] = b'\0';

        if name[0] == b'\0' {
            let envent = environ_find_raw(&*GLOBAL_ENVIRON, c!("HOME"));
            if let Some(envent) = envent {
                if let Some(ref value) = envent.value {
                    if !value.is_empty() {
                        home = value.as_ptr();
                    }
                }
            }
            if home.is_null() {
                if let Some(pw) = NonNull::new(libc::getpwuid(libc::getuid())) {
                    home = (*pw.as_ptr()).pw_dir.cast();
                }
            }
        } else if let Some(pw) = NonNull::new(libc::getpwnam((&raw const name).cast())) {
            home = (*pw.as_ptr()).pw_dir.cast();
        }
        if home.is_null() {
            return false;
        }

        // log_debug("%s: ~%s -> %s", __func__, name, home);
        let home_len = strlen(home);
        yylex_append(buf, core::slice::from_raw_parts(home, home_len));
        true
    }
}

unsafe fn yylex_token(ps: &mut cmd_parse_state, mut ch: i32) -> Option<Vec<u8>> {
    unsafe {
        #[derive(Copy, Clone, Eq, PartialEq)]
        enum State {
            Start,
            None,
            DoubleQuotes,
            SingleQuotes,
        }

        let mut state = State::None;
        let mut last = State::Start;

        let mut buf = Vec::new();

        'error: {
            'aloop: loop {
                'next: {
                    'skip: {
                        // EOF or \n are always the end of the token.
                        if ch == libc::EOF {
                            // log_debug("%s: end at EOF", __func__);
                            break 'aloop;
                        }
                        if state == State::None && ch == '\r' as i32 {
                            ch = yylex_getc(ps);
                            if ch != '\n' as i32 {
                                yylex_ungetc(ps, ch);
                                ch = '\r' as i32;
                            }
                        }
                        if state == State::None && ch == '\n' as i32 {
                            // log_debug("%s: end at EOL", __func__);
                            break 'aloop;
                        }

                        // Whitespace or ; or } ends a token unless inside quotes.
                        if state == State::None && (ch == ' ' as i32 || ch == '\t' as i32) {
                            // log_debug("%s: end at WS", __func__);
                            break 'aloop;
                        }
                        if state == State::None && (ch == ';' as i32 || ch == '}' as i32) {
                            // log_debug("%s: end at %c", __func__, ch);
                            break 'aloop;
                        }

                        // Spaces and comments inside quotes after \n are removed but
                        // the \n is left.
                        if ch == '\n' as i32 && state != State::None {
                            yylex_append1(&mut buf, b'\n');
                            while ({
                                ch = yylex_getc(ps);
                                ch == b' ' as i32
                            }) || ch == '\t' as i32
                            {}
                            if ch != '#' as i32 {
                                continue 'aloop;
                            }
                            ch = yylex_getc(ps);
                            if !libc::strchr(c!(",#{}:"), ch).is_null() {
                                yylex_ungetc(ps, ch);
                                ch = '#' as i32;
                            } else {
                                while {
                                    ch = yylex_getc(ps);
                                    ch != '\n' as i32 && ch != libc::EOF
                                } { /* nothing */ }
                            }
                            continue 'aloop;
                        }

                        // \ ~ and $ are expanded except in single quotes.
                        if ch == '\\' as i32 && state != State::SingleQuotes {
                            if !yylex_token_escape(ps, &mut buf) {
                                break 'error;
                            }
                            break 'skip;
                        }
                        if ch == '~' as i32 && last != state && state != State::SingleQuotes {
                            if !yylex_token_tilde(ps, &mut buf) {
                                break 'error;
                            }
                            break 'skip;
                        }
                        if ch == '$' as i32 && state != State::SingleQuotes {
                            if !yylex_token_variable(ps, &mut buf) {
                                break 'error;
                            }
                            break 'skip;
                        }
                        if ch == '}' as i32 && state == State::None {
                            break 'error; /* unmatched (matched ones were handled) */
                        }

                        // ' and " starts or end quotes (and is consumed).
                        if ch == '\'' as i32 {
                            if state == State::None {
                                state = State::SingleQuotes;
                                break 'next;
                            }
                            if state == State::SingleQuotes {
                                state = State::None;
                                break 'next;
                            }
                        }
                        if ch == b'"' as i32 {
                            if state == State::None {
                                state = State::DoubleQuotes;
                                break 'next;
                            }
                            if state == State::DoubleQuotes {
                                state = State::None;
                                break 'next;
                            }
                        }

                        // Otherwise add the character to the buffer.
                        yylex_append1(&mut buf, ch as u8);
                    }
                    // skip:
                    last = state;
                }
                // next:
                ch = yylex_getc(ps);
            }
            yylex_ungetc(ps, ch);

            buf.push(b'\0');
            // log_debug("%s: %s", __func__, buf.as_ptr());
            return Some(buf);
        } // error:

        None
    }
}

unsafe fn yylex_token_escape(ps: &mut cmd_parse_state, buf: &mut Vec<u8>) -> bool {
    unsafe {
        #[cfg(not(target_os = "macos"))]
        const SIZEOF_M: usize = libc::_SC_MB_LEN_MAX as usize;

        // TODO determine a more stable way to get this value on mac
        #[cfg(target_os = "macos")]
        const SIZEOF_M: usize = 6; // compiled and printed constant from C

        let mut tmp: u32 = 0;
        let mut s: [u8; 9] = [0; 9];
        let mut m: [u8; SIZEOF_M] = [0; SIZEOF_M];
        let size: usize;
        let type_: i32;

        'unicode: {
            let mut ch = yylex_getc(ps);

            if ch >= '4' as i32 && ch <= '7' as i32 {
                yyerror!(ps, "invalid octal escape");
                return false;
            }
            if ch >= '0' as i32 && ch <= '3' as i32 {
                let o2 = yylex_getc(ps);
                if o2 >= '0' as i32 && o2 <= '7' as i32 {
                    let o3 = yylex_getc(ps);
                    if o3 >= '0' as i32 && o3 <= '7' as i32 {
                        ch = 64 * (ch - '0' as i32) + 8 * (o2 - '0' as i32) + (o3 - '0' as i32);
                        yylex_append1(buf, ch as u8);
                        return true;
                    }
                }
                yyerror!(ps, "invalid octal escape");
                return false;
            }

            if ch == libc::EOF {
                return false;
            }

            match ch as u8 as char {
                'a' => ch = '\x07' as i32,
                'b' => ch = '\x08' as i32,
                'e' => ch = '\x1B' as i32,
                'f' => ch = '\x0C' as i32,
                's' => ch = ' ' as i32,
                'v' => ch = '\x0B' as i32,
                'r' => ch = '\r' as i32,
                'n' => ch = '\n' as i32,
                't' => ch = '\t' as i32,
                'u' => {
                    type_ = 'u' as i32;
                    size = 4;
                    break 'unicode;
                }
                'U' => {
                    type_ = 'U' as i32;
                    size = 8;
                    break 'unicode;
                }
                _ => (),
            }

            yylex_append1(buf, ch as u8);
            return true;
        } // unicode:
        let mut i = 0;
        for i_ in 0..size {
            i = i_;
            let ch = yylex_getc(ps);
            if ch == libc::EOF || ch == '\n' as i32 {
                return false;
            }
            if !(ch as u8).is_ascii_hexdigit() {
                yyerror!(ps, "invalid \\{} argument", type_ as u8 as char);
                return false;
            }
            s[i] = ch as u8;
        }
        s[i] = b'\0';

        if (size == 4 && libc::sscanf((&raw mut s).cast(), c"%4x".as_ptr(), &raw mut tmp) != 1)
            || (size == 8 && libc::sscanf((&raw mut s).cast(), c"%8x".as_ptr(), &raw mut tmp) != 1)
        {
            yyerror!(ps, "invalid \\{} argument", type_ as u8 as char);
            return false;
        }
        let mlen = wctomb((&raw mut m).cast(), tmp as i32);
        if mlen <= 0 || mlen > SIZEOF_M as i32 {
            yyerror!(ps, "invalid \\{} argument", type_ as u8 as char);
            return false;
        }
        yylex_append(buf, &m[..mlen as usize]);

        true
    }
}

// # Notes:
//
// <https://matklad.github.io/2020/04/13/simple-but-powerful-pratt-parsing.html>
// <https://github.com/lalrpop/lalrpop/blob/master/README.md>

#[cfg(test)]
mod tests {
    use super::*;

    /// Initialize global state needed by the parser (options, etc.).
    /// Safe to call multiple times — uses Once internally.
    unsafe fn init_globals() {
        use std::sync::Once;
        static INIT: Once = Once::new();
        INIT.call_once(|| unsafe {
            use crate::options_::*;
            use crate::options_table::OPTIONS_TABLE;
            use crate::tmux::{GLOBAL_OPTIONS, GLOBAL_S_OPTIONS, GLOBAL_W_OPTIONS};

            GLOBAL_OPTIONS = options_create(null_mut());
            GLOBAL_S_OPTIONS = options_create(null_mut());
            GLOBAL_W_OPTIONS = options_create(null_mut());
            for oe in &OPTIONS_TABLE {
                if oe.scope & OPTIONS_TABLE_SERVER != 0 {
                    options_default(GLOBAL_OPTIONS, oe);
                }
                if oe.scope & OPTIONS_TABLE_SESSION != 0 {
                    options_default(GLOBAL_S_OPTIONS, oe);
                }
                if oe.scope & OPTIONS_TABLE_WINDOW != 0 {
                    options_default(GLOBAL_W_OPTIONS, oe);
                }
            }
        });
    }

    /// Parse a command string and return the printed representation, or the
    /// error string on failure.
    unsafe fn parse(input: &str) -> Result<String, String> {
        unsafe { init_globals(); }
        unsafe {
            match cmd_parse_from_string(input, None) {
                Ok(cmdlist) => {
                    let printed = cmd_list_print(&*cmdlist, 0);
                    let s = cstr_to_str(printed).to_string();
                    free_(printed);
                    cmd_list_free(cmdlist);
                    Ok(s)
                }
                Err(err) => {
                    Err(err.to_string_lossy().into_owned())
                }
            }
        }
    }

    // ---------------------------------------------------------------
    // Basic command parsing
    // ---------------------------------------------------------------

    #[test]
    fn simple_command() {
        unsafe {
            assert_eq!(parse("set-option -g status off"), Ok("set-option -g status off".into()));
        }
    }

    #[test]
    fn command_abbreviation() {
        unsafe {
            // tmux allows prefix matching on commands
            assert_eq!(parse("set -g status off"), Ok("set-option -g status off".into()));
        }
    }

    #[test]
    fn empty_string() {
        unsafe {
            assert_eq!(parse(""), Ok("".into()));
        }
    }

    #[test]
    fn whitespace_only() {
        unsafe {
            assert_eq!(parse("   "), Ok("".into()));
        }
    }

    #[test]
    fn comment_line() {
        unsafe {
            assert_eq!(parse("# this is a comment"), Ok("".into()));
        }
    }

    #[test]
    fn comment_after_command() {
        unsafe {
            // tmux treats # as comment start (outside quotes)
            let result = parse("set -g status off # comment");
            assert!(result.is_ok(), "expected Ok, got: {:?}", result);
        }
    }

    // ---------------------------------------------------------------
    // Multiple commands (semicolon separated)
    // ---------------------------------------------------------------

    #[test]
    fn two_commands_semicolon() {
        unsafe {
            let result = parse("set -g status off ; set -g prefix C-a");
            assert!(result.is_ok(), "expected Ok, got: {:?}", result);
            let s = result.unwrap();
            assert!(s.contains("set-option"), "expected set-option in: {}", s);
        }
    }

    #[test]
    fn multiple_commands() {
        unsafe {
            let result = parse("new-session ; new-window ; split-window");
            assert!(result.is_ok(), "expected Ok, got: {:?}", result);
        }
    }

    // ---------------------------------------------------------------
    // Quoting and escaping
    // ---------------------------------------------------------------

    #[test]
    fn single_quoted_string() {
        unsafe {
            assert_eq!(
                parse("set -g status-left 'hello world'"),
                Ok(r#"set-option -g status-left "hello world""#.into())
            );
        }
    }

    #[test]
    fn double_quoted_string() {
        unsafe {
            assert_eq!(
                parse(r#"set -g status-left "hello world""#),
                Ok(r#"set-option -g status-left "hello world""#.into())
            );
        }
    }

    #[test]
    fn escaped_semicolon() {
        unsafe {
            // \; should be a literal semicolon, not a command separator
            let result = parse(r"display-message 'hello \; world'");
            assert!(result.is_ok(), "expected Ok, got: {:?}", result);
        }
    }

    #[test]
    fn escaped_quotes_in_double_quotes() {
        unsafe {
            let result = parse(r#"display-message "hello \"world\"""#);
            assert!(result.is_ok(), "expected Ok, got: {:?}", result);
        }
    }

    // ---------------------------------------------------------------
    // Error cases
    // ---------------------------------------------------------------

    #[test]
    fn unknown_command() {
        unsafe {
            let result = parse("this-command-does-not-exist");
            assert!(result.is_err(), "expected Err, got: {:?}", result);
            let err = result.unwrap_err();
            assert!(err.contains("unknown command"), "unexpected error: {}", err);
        }
    }

    #[test]
    fn ambiguous_command() {
        unsafe {
            // "se" could be set-option, select-pane, select-window, etc.
            let result = parse("se");
            assert!(result.is_err(), "expected Err, got: {:?}", result);
            let err = result.unwrap_err();
            assert!(err.contains("ambiguous"), "unexpected error: {}", err);
        }
    }

    // ---------------------------------------------------------------
    // Multi-line input (via cmd_parse_from_buffer)
    // ---------------------------------------------------------------

    #[test]
    fn multiline_buffer() {
        unsafe {
            init_globals();
            let input = b"set -g status off\nnew-session\n";
            let result = cmd_parse_from_buffer(input, None);
            assert!(result.is_ok(), "expected Ok, got: {:?}", result.err());
            cmd_list_free(result.unwrap());
        }
    }

    #[test]
    fn line_continuation_backslash() {
        unsafe {
            init_globals();
            let input = b"set -g \\\nstatus off\n";
            let result = cmd_parse_from_buffer(input, None);
            assert!(result.is_ok(), "expected Ok, got: {:?}", result.err());
            let cmdlist = result.unwrap();
            let printed = cmd_list_print(&*cmdlist, 0);
            let s = cstr_to_str(printed).to_string();
            free_(printed);
            assert!(s.contains("set-option"), "expected set-option in: {}", s);
            assert!(s.contains("status"), "expected status in: {}", s);
            cmd_list_free(cmdlist);
        }
    }

    // ---------------------------------------------------------------
    // Conditional commands (if-shell)
    // ---------------------------------------------------------------

    #[test]
    fn if_shell_simple() {
        unsafe {
            let result = parse("if-shell 'true' 'set -g status off'");
            assert!(result.is_ok(), "expected Ok, got: {:?}", result);
        }
    }

    #[test]
    fn if_shell_with_else() {
        unsafe {
            let result = parse("if-shell 'true' 'set -g status on' 'set -g status off'");
            assert!(result.is_ok(), "expected Ok, got: {:?}", result);
        }
    }

    // ---------------------------------------------------------------
    // Braces (command blocks)
    // ---------------------------------------------------------------

    /// Helper for tests that use cmd_parse_from_buffer directly.
    unsafe fn parse_buffer(input: &[u8]) -> Result<String, String> {
        unsafe {
            init_globals();
            match cmd_parse_from_buffer(input, None) {
                Ok(cmdlist) => {
                    let printed = cmd_list_print(&*cmdlist, 0);
                    let s = cstr_to_str(printed).to_string();
                    free_(printed);
                    cmd_list_free(cmdlist);
                    Ok(s)
                }
                Err(err) => {
                    Err(err.to_string_lossy().into_owned())
                }
            }
        }
    }

    #[test]
    fn command_block_braces() {
        unsafe {
            let result = parse_buffer(b"if-shell 'true' {\n  set -g status off\n}\n");
            assert!(result.is_ok(), "expected Ok, got: {:?}", result);
        }
    }

    #[test]
    fn command_block_with_else() {
        unsafe {
            let result =
                parse_buffer(b"if-shell 'true' {\n  set -g status on\n} {\n  set -g status off\n}\n");
            assert!(result.is_ok(), "expected Ok, got: {:?}", result);
        }
    }

    // ---------------------------------------------------------------
    // Format strings
    // ---------------------------------------------------------------

    #[test]
    fn format_string_in_argument() {
        unsafe {
            let result = parse("display-message '#{session_name}'");
            assert!(result.is_ok(), "expected Ok, got: {:?}", result);
        }
    }

    // ---------------------------------------------------------------
    // Edge cases
    // ---------------------------------------------------------------

    #[test]
    fn trailing_semicolon() {
        unsafe {
            let result = parse("set -g status off ;");
            assert!(result.is_ok(), "expected Ok, got: {:?}", result);
        }
    }

    #[test]
    fn leading_semicolon() {
        unsafe {
            // Leading semicolon triggers a parse error
            let result = parse("; set -g status off");
            assert!(result.is_err(), "expected Err, got: {:?}", result);
        }
    }

    #[test]
    fn double_semicolon_separator() {
        unsafe {
            // ;; is a group separator in tmux
            let result = parse("set -g status off ;; new-session");
            assert!(result.is_ok(), "expected Ok, got: {:?}", result);
        }
    }

    #[test]
    fn newline_in_buffer_separates_commands() {
        unsafe {
            let result = parse_buffer(b"set -g status off\nnew-session\n");
            assert!(result.is_ok(), "expected Ok, got: {:?}", result);
        }
    }

    #[test]
    fn target_flag() {
        unsafe {
            assert_eq!(
                parse("select-window -t :1"),
                Ok("select-window -t :1".into())
            );
        }
    }

    #[test]
    fn multiple_flags() {
        unsafe {
            let result = parse("split-window -h -l 50");
            assert!(result.is_ok(), "expected Ok, got: {:?}", result);
        }
    }

    #[test]
    fn bind_key_with_command() {
        unsafe {
            let result = parse("bind-key C-a set -g status off");
            assert!(result.is_ok(), "expected Ok, got: {:?}", result);
        }
    }

    // ---------------------------------------------------------------
    // %if / %elif / %else / %endif conditionals
    // ---------------------------------------------------------------

    #[test]
    fn percent_if_true() {
        unsafe {
            let input = b"%if 1\nset -g status on\n%endif\n";
            let result = parse_buffer(input);
            assert!(result.is_ok(), "expected Ok, got: {:?}", result);
            let s = result.unwrap();
            assert!(s.contains("set-option"), "expected set-option in: {}", s);
        }
    }

    #[test]
    fn percent_if_false() {
        unsafe {
            let input = b"%if 0\nset -g status on\n%endif\n";
            let result = parse_buffer(input);
            assert!(result.is_ok(), "expected Ok, got: {:?}", result);
            // The command should be skipped when condition is false
            let s = result.unwrap();
            assert!(!s.contains("set-option"), "expected no set-option in: {}", s);
        }
    }

    #[test]
    fn percent_if_else() {
        unsafe {
            let input = b"%if 0\nset -g status on\n%else\nset -g status off\n%endif\n";
            let result = parse_buffer(input);
            assert!(result.is_ok(), "expected Ok, got: {:?}", result);
            let s = result.unwrap();
            assert!(s.contains("status off"), "expected 'status off' in: {}", s);
        }
    }

    #[test]
    fn percent_elif() {
        unsafe {
            let input = b"%if 0\nset -g status on\n%elif 1\nset -g status off\n%endif\n";
            let result = parse_buffer(input);
            assert!(result.is_ok(), "expected Ok, got: {:?}", result);
            let s = result.unwrap();
            assert!(s.contains("status off"), "expected 'status off' in: {}", s);
        }
    }

    #[test]
    fn percent_hidden() {
        unsafe {
            let input = b"%hidden set -g status off\n";
            let result = parse_buffer(input);
            // %hidden is a valid directive — may succeed or fail depending
            // on parser state, but should not panic
            let _ = result;
        }
    }

    #[test]
    fn percent_if_nested() {
        unsafe {
            let input = b"%if 1\n%if 1\nset -g status on\n%endif\n%endif\n";
            let result = parse_buffer(input);
            assert!(result.is_ok(), "expected Ok, got: {:?}", result);
            let s = result.unwrap();
            assert!(s.contains("set-option"), "expected set-option in: {}", s);
        }
    }

    // ---------------------------------------------------------------
    // Escape sequences (yylex_token_escape coverage)
    // ---------------------------------------------------------------

    #[test]
    fn escape_octal() {
        unsafe {
            // \101 = 'A' in octal
            let result = parse(r#"display-message "\101""#);
            assert!(result.is_ok(), "expected Ok, got: {:?}", result);
        }
    }

    #[test]
    fn escape_named_sequences() {
        unsafe {
            // \n = newline, \t = tab, \r = carriage return, \e = escape
            let result = parse(r#"display-message "\n\t\r\e""#);
            assert!(result.is_ok(), "expected Ok, got: {:?}", result);
        }
    }

    #[test]
    fn escape_backslash() {
        unsafe {
            let result = parse(r#"display-message "hello\\world""#);
            assert!(result.is_ok(), "expected Ok, got: {:?}", result);
        }
    }

    #[test]
    fn escape_unicode_u() {
        unsafe {
            // \u0041 = 'A'
            let result = parse(r#"display-message "\u0041""#);
            assert!(result.is_ok(), "expected Ok, got: {:?}", result);
        }
    }

    #[test]
    fn escape_unicode_upper_u() {
        unsafe {
            // \U00000041 = 'A'
            let result = parse(r#"display-message "\U00000041""#);
            assert!(result.is_ok(), "expected Ok, got: {:?}", result);
        }
    }

    #[test]
    fn escape_invalid_octal() {
        unsafe {
            // \4xx is invalid (octal must start with 0-3)
            let result = parse(r#"display-message "\4""#);
            assert!(result.is_err(), "expected Err, got: {:?}", result);
            let err = result.unwrap_err();
            assert!(err.contains("invalid octal escape"), "unexpected error: {}", err);
        }
    }

    #[test]
    fn escape_bell_formfeed_space_vtab() {
        unsafe {
            // \a = bell, \f = formfeed, \s = space, \v = vtab, \b = backspace
            let result = parse(r#"display-message "\a\b\f\s\v""#);
            assert!(result.is_ok(), "expected Ok, got: {:?}", result);
        }
    }

    // ---------------------------------------------------------------
    // yylex_format and #{...} format strings
    // ---------------------------------------------------------------

    #[test]
    fn format_in_condition() {
        unsafe {
            // %if #{...} triggers yylex_format in a condition context
            let input = b"%if #{session_name}\nset -g status on\n%endif\n";
            let result = parse_buffer(input);
            assert!(result.is_ok(), "expected Ok, got: {:?}", result);
        }
    }

    #[test]
    fn format_nested_braces() {
        unsafe {
            // #{...#{...}...} — nested format braces in lexer
            // Use a simpler nested format that doesn't need environ
            let input = b"%if #{==:1,1}\nset -g status on\n%endif\n";
            let result = parse_buffer(input);
            assert!(result.is_ok(), "expected Ok, got: {:?}", result);
        }
    }

    // ---------------------------------------------------------------
    // \r handling in lexer
    // ---------------------------------------------------------------

    #[test]
    fn carriage_return_newline() {
        unsafe {
            // \r\n should be treated as \n
            let input = b"set -g status off\r\nnew-session\r\n";
            let result = parse_buffer(input);
            assert!(result.is_ok(), "expected Ok, got: {:?}", result);
        }
    }

    #[test]
    fn standalone_carriage_return() {
        unsafe {
            // bare \r (not followed by \n)
            let input = b"set -g status off\rnew-session\n";
            let result = parse_buffer(input);
            assert!(result.is_ok(), "expected Ok, got: {:?}", result);
        }
    }

    // ---------------------------------------------------------------
    // cmd_parse_from_arguments
    // ---------------------------------------------------------------

    #[test]
    fn parse_from_arguments_simple() {
        unsafe {
            init_globals();
            // Build a simple args_value array: ["set", "-g", "status", "off"]
            let words: [&[u8]; 4] = [b"set\0", b"-g\0", b"status\0", b"off\0"];
            let values: *mut args_value = xcalloc_(4).as_ptr();
            for (i, word) in words.iter().enumerate() {
                (*values.add(i)).type_ = args_type::ARGS_STRING;
                (*values.add(i)).union_.string = xstrdup(word.as_ptr()).as_ptr();
            }
            let result = cmd_parse_from_arguments(values, 4, None);
            assert!(result.is_ok(), "expected Ok, got err");
            cmd_list_free(result.unwrap());
            args_free_values(values, 4);
            free_(values);
        }
    }

    #[test]
    fn parse_from_arguments_with_semicolon() {
        unsafe {
            init_globals();
            // ["set", "-g", "status", "off;", "new-session"] — semicolon splits commands
            let words: [&[u8]; 5] = [b"set\0", b"-g\0", b"status\0", b"off;\0", b"new-session\0"];
            let values: *mut args_value = xcalloc_(5).as_ptr();
            for (i, word) in words.iter().enumerate() {
                (*values.add(i)).type_ = args_type::ARGS_STRING;
                (*values.add(i)).union_.string = xstrdup(word.as_ptr()).as_ptr();
            }
            let result = cmd_parse_from_arguments(values, 5, None);
            assert!(result.is_ok(), "expected Ok, got err");
            cmd_list_free(result.unwrap());
            args_free_values(values, 5);
            free_(values);
        }
    }

    #[test]
    fn parse_from_arguments_escaped_semicolon() {
        unsafe {
            init_globals();
            // ["display-message", "hello\\;"] — escaped semicolon, not a separator
            let words: [&[u8]; 2] = [b"display-message\0", b"hello\\;\0"];
            let values: *mut args_value = xcalloc_(2).as_ptr();
            for (i, word) in words.iter().enumerate() {
                (*values.add(i)).type_ = args_type::ARGS_STRING;
                (*values.add(i)).union_.string = xstrdup(word.as_ptr()).as_ptr();
            }
            let result = cmd_parse_from_arguments(values, 2, None);
            assert!(result.is_ok(), "expected Ok, got err");
            cmd_list_free(result.unwrap());
            args_free_values(values, 2);
            free_(values);
        }
    }

    // ---------------------------------------------------------------
    // Command aliases
    // ---------------------------------------------------------------

    #[test]
    fn command_alias_expansion() {
        unsafe {
            init_globals();
            // Set up a command alias: "myalias" = "set -g status off"
            use crate::options_::*;
            let o = options_get_only(GLOBAL_OPTIONS, "command-alias");
            if !o.is_null() {
                let _ = options_array_set(o, 0, Some("myalias=set -g status off"), false);
                let result = parse("myalias");
                assert!(result.is_ok(), "expected Ok, got: {:?}", result);
                let s = result.unwrap();
                assert!(s.contains("set-option"), "expected set-option in: {}", s);
            }
        }
    }

    // ---------------------------------------------------------------
    // Parse errors (yyerror coverage)
    // ---------------------------------------------------------------

    #[test]
    fn unterminated_single_quote() {
        unsafe {
            // tmux treats unterminated quotes at EOF as implicitly closed
            let result = parse("display-message 'hello");
            assert!(result.is_ok(), "expected Ok, got: {:?}", result);
        }
    }

    #[test]
    fn unterminated_double_quote() {
        unsafe {
            let result = parse(r#"display-message "hello"#);
            assert!(result.is_ok(), "expected Ok, got: {:?}", result);
        }
    }

    #[test]
    fn unmatched_endif() {
        unsafe {
            let result = parse_buffer(b"%endif\n");
            assert!(result.is_err(), "expected Err, got: {:?}", result);
        }
    }

    #[test]
    fn unmatched_else() {
        unsafe {
            let result = parse_buffer(b"%else\n");
            assert!(result.is_err(), "expected Err, got: {:?}", result);
        }
    }

    #[test]
    fn missing_endif() {
        unsafe {
            let result = parse_buffer(b"%if 1\nset -g status on\n");
            assert!(result.is_err(), "expected Err, got: {:?}", result);
        }
    }

    // ---------------------------------------------------------------
    // cmd_parse_and_append (error branch)
    // ---------------------------------------------------------------

    #[test]
    fn parse_and_append_error() {
        unsafe {
            init_globals();
            let mut error: *mut u8 = null_mut();
            let status = cmd_parse_and_append(
                "this-does-not-exist",
                None,
                null_mut(),
                null_mut(),
                &raw mut error,
            );
            assert!(matches!(status, cmd_parse_status::CMD_PARSE_ERROR));
            assert!(!error.is_null());
            free_(error);
        }
    }
}
