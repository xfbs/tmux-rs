% TMUX(1) BSD | BSD General Commands Manual

NAME
====

**tmux** — terminal multiplexer

SYNOPSIS
========

| **tmux** \[**-2CDlNuVv**] \[**-c** _shell-command_] \[**-f** _file_] \[**-L** _socket-name_] \[**-S** _socket-path_] \[**-T** _features_] \[_command_ \[_flags_]]

DESCRIPTION
===========

**tmux** is a terminal multiplexer: it enables a number of terminals to be created, accessed, and controlled from a single screen. **tmux** may be detached from a screen and continue running in the background, then later reattached.

When **tmux** is started, it creates a new _session_ with a single _window_ and displays it on screen.  A status line at the bottom of the screen shows information on the
current session and is used to enter interactive commands.

A session is a single collection of _pseudo terminals_ under the management of tmux.  Each session has one or more windows linked to it.  A window occupies the en‐
tire screen and may be split into rectangular panes, each of which is a separate pseudo terminal (the pty(4) manual page documents the technical details of pseudo
terminals).  Any number of *tmux* instances may connect to the same session, and any number of windows may be present in the same session.  Once all sessions are
killed, *tmux* exits.

Each session is persistent and will survive accidental disconnection (such as ssh(1) connection timeout) or intentional detaching (with the ‘C-b d’ key strokes). **tmux** may be reattached using:

	$ tmux attach

In tmux, a session is displayed on screen by a _client_ and all sessions are managed by a single _server_.  The server and each client are separate processes which communicate through a socket in _/tmp_.

The options are as follows:

**-2**

:   Force **tmux** to assume the terminal supports 256 colours.  This is equivalent to **-T** _256_.

**-C**

:   Start in control mode (see the CONTROL MODE section).  Given twice (-CC) disables echo.

**-c** _shell-command_

:   Execute shell-command using the default shell.  If necessary, the tmux server will be started to retrieve the default-shell option.  This option is for compatibility with sh(1) when tmux is used as a login shell.

**-D**

:   Do not start the tmux server as a daemon.  This also turns the exit-empty option off.  With -D, command may not be specified.

**-f** _file_

:   Specify an alternative configuration file.  By default, tmux loads the system configuration file from /etc/tmux.conf, if present, then looks for a user configuration file at ~/.tmux.conf or $XDG_CONFIG_HOME/tmux/tmux.conf.

    The configuration file is a set of tmux commands which are executed in sequence when the server is first started.  tmux loads configuration files once when the server process has started.  The source-file command may be used to load a file later.

    tmux shows any error messages from commands in configuration files in the first session created, and continues to process the rest of the configuration file.

**-L** socket-name

:   tmux stores the server socket in a directory under TMUX_TMPDIR or /tmp if it is unset.  The default socket is named default. This option allows a different socket name to be specified, allowing several independent tmux servers to be run. Unlike -S a full path is not necessary: the sockets are all created in a directory tmux-UID under the directory given by TMUX_TMPDIR or in /tmp.  The tmux-UID directory is created by tmux and must not be world readable, writable or executable.

    If the socket is accidentally removed, the SIGUSR1 signal may be sent to the tmux server process to recreate it (note that this will fail if any parent directories are missing).

**-l**

:   Behave as a login shell.  This flag currently has no effect and is for compatibility with other shells when using tmux as a login shell.

**-N**

:   Do not start the server even if the command would normally do so (for example new-session or start-server).

**-S** _socket-path_

:   Specify a full alternative path to the server socket.  If -S is specified, the default socket directory is not used and any -L flag is ignored.

**-T** _features_

:   Set terminal features for the client.  This is a comma-separated list of features.  See the terminal-features option.

**-u**

:   Write UTF-8 output to the terminal even if the first environment variable of LC_ALL, LC_CTYPE, or LANG that is set does not contain "UTF-8" or "UTF8".

**-V**

:   Report the tmux version.

**-v**

:   Request verbose logging.  Log messages will be saved into tmux-client-PID.log and tmux-server-PID.log files in the current directory, where PID is the PID of the server or client process.  If -v is specified twice, an additional tmux-out-PID.log file is generated with a copy of everything tmux writes to the terminal.

    The SIGUSR2 signal may be sent to the tmux server process to toggle logging between on (as if -v was given) and off.

_command_ [_flags_]
:   This specifies one of a set of commands used to control tmux, as described in the following sections.  If no commands are specified, the new-session command is assumed.

DEFAULT KEY BINDINGS
====================

**tmux** may be controlled from an attached client by using a key combination of a prefix key, 'C-b' (Ctrl-b) by default, followed by a command key.

The default command key bindings are:

|                    |                                                                                    |
| ------------------ | ---------------------------------------------------------------------------------- |
| C-b                | Send the prefix key (C-b) through to the application.                              |
| C-o                | Rotate the panes in the current window forwards.                                   |
| C-z                | Suspend the **tmux** client.                                                           |
| !                  | Break the current pane out of the window.                                          |
| \"                 | Split the current pane into two, top and bottom.                                   |
| #                  | List all paste buffers.                                                            |
| $                  | Rename the current session.                                                        |
| %                  | Split the current pane into two, left and right.                                   |
| &                  | Kill the current window.                                                           |
| \'                 | Prompt for a window index to select.                                               |
| (                  | Switch the attached client to the previous session.                                |
| )                  | Switch the attached client to the next session.                                    |
| ,                  | Rename the current window.                                                         |
| -                  | Delete the most recently copied buffer of text.                                    |
| .                  | Prompt for an index to move the current window.                                    |
| 0 to 9             | Select windows 0 to 9.                                                             |
| :                  | Enter the **tmux** command prompt.                                                     |
| ;                  | Move to the previously active pane.                                                |
| =                  | Choose which buffer to paste interactively from a list.                            |
| ?                  | List all key bindings.                                                             |
| D                  | Choose a client to detach.                                                         |
| L                  | Switch the attached client back to the last session.                               |
| \[                 | Enter copy mode to copy text or view the history.                                  |
| \]                 | Paste the most recently copied buffer of text.                                     |
| c                  | Create a new window.                                                               |
| d                  | Detach the current client.                                                         |
| f                  | Prompt to search for text in open windows.                                         |
| i                  | Display some information about the current window.                                 |
| l                  | Move to the previously selected window.                                            |
| m                  | Mark the current pane (see **select-pane -m**).                                        |
| M                  | Clear the marked pane.                                                             |
| n                  | Change to the next window.                                                         |
| o                  | Select the next pane in the current window.                                        |
| p                  | Change to the previous window.                                                     |
| q                  | Briefly display pane indexes.                                                      |
| r                  | Force redraw of the attached client.                                               |
| s                  | Select a new session for the attached client interactively.                        |
| t                  | Show the time.                                                                     |
| w                  | Choose the current window interactively.                                           |
| x                  | Kill the current pane.                                                             |
| z                  | Toggle zoom state of the current pane.                                             |
| {                  | Swap the current pane with the previous pane.                                      |
| }                  | Swap the current pane with the next pane.                                          |
| ~                  | Show previous messages from tmux, if any.                                          |
| Page Up            | Enter copy mode and scroll one page up.                                            |
| Up Down Left Right | Change to the pane above, below, to the left, or to the right of the current pane. |
| M-1 to M-5         | Arrange panes in one of the seven preset layouts: even-horizontal, even-vertical, main-horizontal, main-horizontal-mirrored, main-vertical, main- vertical, or tiled. |
| Space                      | Arrange the current window in the next preset layout.                      |
| M-n                        | Move to the next window with a bell or activity marker.                    |
| M-o                        | Rotate the panes in the current window backwards.                          |
| M-p                        | Move to the previous window with a bell or activity marker.                |
| C-Up C-Down C-Left C-Right | Resize the current pane in steps of one cell.                              |
| M-Up M-Down M-Left M-Right | Resize the current pane in steps of five cells.                            |


Key bindings may be changed with the **bind-key** and **unbind-key** commands.

COMMAND PARSING AND EXECUTION
=============================

**DEFAULT_HELLO_DEDICATION**

:   The default dedication if none is given. Has the highest precedence
    if a dedication is not supplied on the command line.

PARSING SYNTAX
==============

COMMANDS
========

CLIENTS AND SESSIONS
====================

WINDOWS AND PANES
=================

KEY BINDINGS
============

OPTIONS
=======

HOOKS
=====

MOUSE SUPPORT
=============

FORMATS
=======

STYLES
======

NAMES AND TITLES
================

GLOBAL AND SESSION ENVIRONMENT
==============================

STATUS LINE
===========

BUFFERS
=======

MISCELLANEOUS
=============

EXIT MESSAGES
=============

TERMINFO EXTENSIONS
===================

CONTROL MODE
============

ENVIRONMENT
===========

FILES
=====

     ~/.tmux.conf
     $XDG_CONFIG_HOME/tmux/tmux.conf
     ~/.config/tmux/tmux.conf
                        Default tmux configuration file.
     /etc/tmux.conf     System-wide configuration file.

EXAMPLES
========

SEE ALSO
========

pty(4)

AUTHORS
=======

Nicholas Marriott <nicholas.marriott@gmail.com>
