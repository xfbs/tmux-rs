use crate::*;

use compat_rs::{strtonum, tree::rb_min};
use libc::{sscanf, strcmp, tcgetattr};

const NEW_SESSION_TEMPLATE: &CStr = c"#{session_name}:";

#[unsafe(no_mangle)]
static mut cmd_new_session_entry : cmd_entry = cmd_entry  {
	name : c"new-session".as_ptr(),
	alias : c"new".as_ptr(),

	args : args_parse::new( c"Ac:dDe:EF:f:n:Ps:t:x:Xy:", 0, -1, None),
	usage : c"[-AdDEPX] [-c start-directory] [-e environment] [-F format] [-f flags] [-n window-name] [-s session-name] [-t target-session] [-x width] [-y height] [shell-command]".as_ptr(),

	target : cmd_entry_flag::new( b't', cmd_find_type::CMD_FIND_SESSION, CMD_FIND_CANFAIL ),

	flags : CMD_STARTSERVER,
	exec : Some(cmd_new_session_exec),
    ..unsafe{zeroed()}
};

#[unsafe(no_mangle)]
static mut cmd_has_session_entry: cmd_entry = cmd_entry {
    name: c"has-session".as_ptr(),
    alias: c"has".as_ptr(),

    args: args_parse::new(c"t:", 0, 0, None),
    usage: c"[-t target-session]".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_SESSION, 0),

    flags: 0,
    exec: Some(cmd_new_session_exec),

    ..unsafe { zeroed() }
};

unsafe extern "C" {
    // fn cmd_new_session_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval;
}

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_new_session_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    let __func__ = c"cmd_new_session_exec".as_ptr();

    unsafe {
        let mut args = cmd_get_args(self_);
        let mut current = cmdq_get_current(item);
        let mut target = cmdq_get_target(item);
        let mut c = cmdq_get_client(item);
        let mut s = null_mut();
        let mut as_ = null_mut();
        let mut groupwith = null_mut();
        let mut env: *mut environ = null_mut();
        let mut oo = null_mut();
        let mut tio: termios = zeroed();
        let mut tiop = null_mut();
        let mut sg: *mut session_group = null_mut();
        let mut errstr: *const c_char = null();
        let mut group: *const c_char = null();
        let mut tmp: *const c_char = null();
        let mut cause = null_mut();
        let mut cwd = null_mut();
        let mut cp = null_mut();
        let mut newname = null_mut();
        let mut name = null_mut();
        let mut prefix = null_mut();
        let mut detached = false;
        let mut already_attached = false;
        let mut is_control = false;
        let mut sx = 0u32;
        let mut sy = 0u32;
        let mut dsx = 0u32;
        let mut dsy = 0u32;
        let mut count = args_count(args);
        let mut sc: spawn_context = zeroed();
        let mut retval = cmd_retval::CMD_RETURN_NORMAL;
        let mut av: *mut args_value;

        'fail: {
            if cmd_get_entry(self_) == &raw mut cmd_has_session_entry {
                /*
                 * cmd_find_target() will fail if the session cannot be found,
                 * so always return success here.
                 */
                return cmd_retval::CMD_RETURN_NORMAL;
            }

            if (args_has_(args, 't') && (count != 0 || args_has_(args, 'n'))) {
                cmdq_error(item, c"command or window name given with target".as_ptr());
                return cmd_retval::CMD_RETURN_ERROR;
            }

            tmp = args_get_(args, 's');
            if (!tmp.is_null()) {
                name = format_single(item, tmp, c, null_mut(), null_mut(), null_mut());
                newname = session_check_name(name);
                if (newname.is_null()) {
                    cmdq_error(item, c"invalid session: %s".as_ptr(), name);
                    free_(name);
                    return cmd_retval::CMD_RETURN_ERROR;
                }
                free_(name);
            }
            if (args_has_(args, 'A')) {
                as_ = if (!newname.is_null()) {
                    session_find(newname)
                } else {
                    (*target).s
                };
                if (!as_.is_null()) {
                    retval = cmd_attach_session(
                        item,
                        (*as_).name,
                        args_has(args, b'D'),
                        args_has(args, b'X'),
                        0,
                        null(),
                        args_has(args, b'E'),
                        args_get(args, b'f'),
                    );
                    free_(newname);
                    return retval;
                }
            }
            if (!newname.is_null() && !session_find(newname).is_null()) {
                cmdq_error(item, c"duplicate session: %s".as_ptr(), newname);
                break 'fail;
            }

            /* Is this going to be part of a session group? */
            group = args_get_(args, 't');
            if (!group.is_null()) {
                groupwith = (*target).s;
                sg = if (groupwith.is_null()) {
                    session_group_find(group)
                } else {
                    session_group_contains(groupwith)
                };
                if (!sg.is_null()) {
                    prefix = xstrdup((*sg).name).as_ptr();
                } else if (!groupwith.is_null()) {
                    prefix = xstrdup((*groupwith).name).as_ptr();
                } else {
                    prefix = session_check_name(group);
                    if (prefix.is_null()) {
                        cmdq_error(item, c"invalid session group: %s".as_ptr(), group);
                        break 'fail;
                    }
                }
            }

            /* Set -d if no client. */
            detached = args_has_(args, 'd');
            if (c.is_null()) {
                detached = true;
            } else if ((*c).flags.intersects(client_flag::CONTROL)) {
                is_control = true;
            }

            /* Is this client already attached? */
            already_attached = false;
            if (!c.is_null() && !(*c).session.is_null()) {
                already_attached = true;
            }

            /* Get the new session working directory. */
            tmp = args_get_(args, 'c');
            cwd = if (!tmp.is_null()) {
                format_single(item, tmp, c, null_mut(), null_mut(), null_mut())
            } else {
                xstrdup(server_client_get_cwd(c, null_mut())).as_ptr()
            };

            /*
             * If this is a new client, check for nesting and save the termios
             * settings (part of which is used for new windows in this session).
             *
             * tcgetattr() is used rather than using tty.tio since if the client is
             * detached, tty_open won't be called. It must be done before opening
             * the terminal as that calls tcsetattr() to prepare for tmux taking
             * over.
             */
            if !detached && !already_attached && (*c).fd != -1 && !(*c).flags.intersects(client_flag::CONTROL) {
                if (server_client_check_nested(cmdq_get_client(item)) != 0) {
                    cmdq_error(
                        item,
                        c"sessions should be nested with care, unset $TMUX to force".as_ptr(),
                    );
                    break 'fail;
                }
                if (tcgetattr((*c).fd, &raw mut tio) != 0) {
                    fatal(c"tcgetattr failed".as_ptr());
                }
                tiop = &raw mut tio;
            } else {
                tiop = null_mut();
            }

            /* Open the terminal if necessary. */
            if (!detached && !already_attached) {
                if (server_client_open(c, &raw mut cause) != 0) {
                    cmdq_error(item, c"open terminal failed: %s".as_ptr(), cause);
                    free_(cause);
                    break 'fail;
                }
            }

            /* Get default session size. */
            dsx = if (args_has_(args, 'x')) {
                tmp = args_get_(args, 'x');
                if (strcmp(tmp, c"-".as_ptr()) == 0) {
                    if !c.is_null() { (*c).tty.sx } else { 80 }
                } else {
                    let dsx_ = strtonum(tmp, 1, u16::MAX as i64, &raw mut errstr) as u32;
                    if (!errstr.is_null()) {
                        cmdq_error(item, c"width %s".as_ptr(), errstr);
                        break 'fail;
                    }
                    dsx_
                }
            } else {
                80
            };

            dsy = if (args_has_(args, 'y')) {
                tmp = args_get_(args, 'y');
                if (strcmp(tmp, c"-".as_ptr()) == 0) {
                    if (!c.is_null()) { (*c).tty.sy } else { 24 }
                } else {
                    let dsy_ = strtonum(tmp, 1, u16::MAX as i64, &raw mut errstr) as u32;
                    if (!errstr.is_null()) {
                        cmdq_error(item, c"height %s".as_ptr(), errstr);
                        break 'fail;
                    }
                    dsy_
                }
            } else {
                24
            };

            // sx = 0;
            // sy = 0;
            /* Find new session size. */
            if (!detached && !is_control) {
                sx = (*c).tty.sx;
                sy = (*c).tty.sy;
                if (sy > 0 && options_get_number(global_s_options, c"status".as_ptr()) != 0) {
                    sy -= 1;
                }
            } else {
                tmp = options_get_string(global_s_options, c"default-size".as_ptr());
                if (sscanf(tmp, c"%ux%u".as_ptr(), &sx, &sy) != 2) {
                    sx = dsx;
                    sy = dsy;
                } else {
                    if (args_has_(args, 'x')) {
                        sx = dsx;
                    }
                    if (args_has_(args, 'y')) {
                        sy = dsy;
                    }
                }
            }
            if (sx == 0) {
                sx = 1;
            }
            if (sy == 0) {
                sy = 1;
            }

            /* Create the new session. */
            oo = options_create(global_s_options);
            if (args_has_(args, 'x') || args_has_(args, 'y')) {
                if (!args_has_(args, 'x')) {
                    dsx = sx;
                }
                if (!args_has_(args, 'y')) {
                    dsy = sy;
                }
                options_set_string(oo, c"default-size".as_ptr(), 0, c"%ux%u".as_ptr(), dsx, dsy);
            }
            env = environ_create().as_ptr();
            if (!c.is_null() && !args_has_(args, 'E')) {
                environ_update(global_s_options, (*c).environ, env);
            }
            av = args_first_value(args, b'e');
            while (!av.is_null()) {
                environ_put(env, (*av).union_.string, 0);
                av = args_next_value(av);
            }
            s = session_create(prefix, newname, cwd, env, oo, tiop);

            /* Spawn the initial window. */
            sc.item = item;
            sc.s = s;
            if (!detached) {
                sc.tc = c;
            }

            sc.name = args_get_(args, 'n');
            args_to_vector(args, &raw mut sc.argc, &raw mut sc.argv);

            sc.idx = -1;
            sc.cwd = args_get_(args, 'c');

            sc.flags = 0;

            if (spawn_window(&raw mut sc, &raw mut cause).is_null()) {
                session_destroy(s, 0, __func__);
                cmdq_error(item, c"create window failed: %s".as_ptr(), cause);
                free_(cause);
                break 'fail;
            }

            /*
             * If a target session is given, this is to be part of a session group,
             * so add it to the group and synchronize.
             */
            if (!group.is_null()) {
                if (sg.is_null()) {
                    if (!groupwith.is_null()) {
                        sg = session_group_new((*groupwith).name);
                        session_group_add(sg, groupwith);
                    } else {
                        sg = session_group_new(group);
                    }
                }
                session_group_add(sg, s);
                session_group_synchronize_to(s);
                session_select(s, (*rb_min::<winlink, _>(&raw mut (*s).windows)).idx);
            }
            notify_session(c"session-created".as_ptr(), s);

            /*
             * Set the client to the new session. If a command client exists, it is
             * taking this session and needs to get MSG_READY and stay around.
             */
            if (!detached) {
                if (args_has_(args, 'f')) {
                    server_client_set_flags(c, args_get_(args, 'f'));
                }
                if (!already_attached) {
                    if !(*c).flags.intersects(client_flag::CONTROL) {
                        proc_send((*c).peer, msgtype::MSG_READY, -1, null(), 0);
                    }
                } else if (!(*c).session.is_null()) {
                    (*c).last_session = (*c).session;
                }
                server_client_set_session(c, s);
                if (!cmdq_get_flags(item) & CMDQ_STATE_REPEAT != 0) {
                    server_client_set_key_table(c, null_mut());
                }
            }

            /* Print if requested. */
            if (args_has_(args, 'P')) {
                let mut template: *const c_char = args_get_(args, 'F');
                if (template.is_null()) {
                    template = NEW_SESSION_TEMPLATE.as_ptr();
                }
                cp = format_single(item, template, c, s, (*s).curw, null_mut());
                cmdq_print(item, c"%s".as_ptr(), cp);
                free_(cp);
            }

            if (!detached) {
                (*c).flags |= client_flag::ATTACHED;
            }
            if (!args_has_(args, 'd')) {
                cmd_find_from_session(current, s, 0);
            }

            let mut fs: MaybeUninit<cmd_find_state> = MaybeUninit::<cmd_find_state>::uninit(); //TODO use uninit;
            cmd_find_from_session(fs.as_mut_ptr(), s, 0);
            cmdq_insert_hook(s, item, fs.as_mut_ptr(), c"after-new-session".as_ptr());

            if (cfg_finished != 0) {
                cfg_show_causes(s);
            }

            if (!sc.argv.is_null()) {
                cmd_free_argv(sc.argc, sc.argv);
            }
            free_(cwd);
            free_(newname);
            free_(prefix);
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        //fail:
        if (!sc.argv.is_null()) {
            cmd_free_argv(sc.argc, sc.argv);
        }

        free_(cwd);
        free_(newname);
        free_(prefix);
        cmd_retval::CMD_RETURN_ERROR
    }
}
