# Obvious program stuff.
bin_PROGRAMS = tmux
CLEANFILES = tmux.1.mdoc tmux.1.man cmd-parse.c

# Distribution tarball options.
EXTRA_DIST = \
	CHANGES README README.ja COPYING example_tmux.conf \
	osdep-*.c mdoc2man.awk tmux.1
dist_EXTRA_tmux_SOURCES = compat/*.[ch]

# Preprocessor flags.
AM_CPPFLAGS += @XOPEN_DEFINES@ \
	-DTMUX_VERSION='"@VERSION@"' \
	-DTMUX_CONF='"$(sysconfdir)/tmux.conf:~/.tmux.conf:$$XDG_CONFIG_HOME/tmux/tmux.conf:~/.config/tmux/tmux.conf"' \
	-DTMUX_LOCK_CMD='"@DEFAULT_LOCK_CMD@"' \
	-DTMUX_TERM='"@DEFAULT_TERM@"'

# Additional object files.
LDADD = $(LIBOBJS) -L/home/collin/Git/tmux/tmux-3.5a/target/x86_64-unknown-linux-gnu/debug -ltmux_rs -lbsd
# LDADD = $(LIBOBJS) -L/home/collin/Git/tmux/tmux-3.5a/target/release -ltmux_rs

AM_CFLAGS += -fsanitize=address -fno-omit-frame-pointer -O0

# Set flags for gcc.
if IS_GCC
AM_CFLAGS += -std=gnu99
if IS_DEBUG
AM_CFLAGS += -g
AM_CFLAGS += -Wno-long-long -Wall -W -Wformat=2
AM_CFLAGS += -Wmissing-prototypes -Wstrict-prototypes -Wmissing-declarations
AM_CFLAGS += -Wwrite-strings -Wshadow -Wpointer-arith -Wsign-compare
AM_CFLAGS += -Wundef -Wbad-function-cast -Winline -Wcast-align
AM_CFLAGS += -Wdeclaration-after-statement -Wno-pointer-sign -Wno-attributes
AM_CFLAGS += -Wno-unused-result -Wno-format-y2k
if IS_DARWIN
AM_CFLAGS += -Wno-deprecated-declarations -Wno-cast-align
endif
AM_CPPFLAGS += -DDEBUG
endif
AM_CPPFLAGS += -iquote.
endif

# Set flags for Solaris.
if IS_SUNOS
if IS_GCC
AM_CPPFLAGS += -D_XPG6
else
AM_CPPFLAGS += -D_XPG4_2
endif
endif

# Set flags for Sun CC.
if IS_SUNCC
AM_CFLAGS += -erroff=E_EMPTY_DECLARATION
endif

# Set _LINUX_SOURCE_COMPAT for AIX for malloc(0).
if IS_AIX
AM_CPPFLAGS += -D_LINUX_SOURCE_COMPAT=1
endif

# Set flags for NetBSD.
if IS_NETBSD
AM_CPPFLAGS += -D_OPENBSD_SOURCE
endif

# Set flags for Haiku.
if IS_HAIKU
AM_CPPFLAGS += -D_BSD_SOURCE
endif

# List of sources.
# alerts.c
# attributes.c
# cmd-copy-mode.c
# cmd-kill-server.c
# log.c
# window.c
# xmalloc.c
# cfg.c
# regsub.c
# colour.c
# names.c
# tmux.c
# control-notify.c
# cmd.c
# client.c
# cmd-attach-session.c
# cmd-bind-key.c
# cmd-break-pane.c
# cmd-kill-session.c
# cmd-capture-pane.c
# cmd-choose-tree.c
# cmd-command-prompt.c
# style.c
# cmd-confirm-before.c
# cmd-detach-client.c
# cmd-kill-pane.c
# cmd-swap-window.c
# cmd-find-window.c
# cmd-rename-window.c
# cmd-lock-server.c
# cmd-list-buffers.c
# cmd-rename-session.c
# cmd-list-sessions.c
# cmd-respawn-window.c
# cmd-respawn-pane.c
# cmd-list-clients.c
# cmd-unbind-key.c
# cmd-show-messages.c
# cmd-show-prompt-history.c
# cmd-kill-window.c 
# cmd-load-buffer.c
# cmd-paste-buffer.c
# cmd-resize-window.c
# cmd-rotate-window.c
# cmd-set-environment.c
# cmd-save-buffer.c
# cmd-move-window.c
# cmd-list-windows.c
# cmd-set-buffer.c
# cmd-switch-client.c
# cmd-show-environment.c
# cmd-server-access.c
# cmd-list-panes.c
# cmd-swap-pane.c
# cmd-select-layout.c
# cmd-select-window.c
# cmd-display-message.c
# cmd-new-window.c
# cmd-display-menu.c
# cmd-join-pane.c
# cmd-if-shell.c
# cmd-display-panes.c
# cmd-list-keys.c
# cmd-split-window.c
# job.c
# cmd-source-file.c
# file.c
# window-clock.c
# control.c
# proc.c
# utf8-combined.c
# utf8.c
# paste.c
# hyperlinks.c
# cmd-pipe-pane.c
# cmd-send-keys.c
# server-acl.c
# cmd-set-option.c
# cmd-refresh-client.c
# cmd-select-pane.c
# cmd-show-options.c
# cmd-wait-for.c
# cmd-run-shell.c
# arguments.c
# cmd-resize-pane.c
# spawn.c
# cmd-find.c
# tty-acs.c
# layout-custom.c
# window-client.c
# grid-reader.c
# resize.c
# key-string.c
# tty-term.c
# server-fn.c
# cmd-new-session.c
# menu.c
# tty-features.c
# key-bindings.c
# input-keys.c
# window-buffer.c
# layout-set.c
# grid-view.c
# screen.c
# screen-redraw.c

# TMP moved to build.rs cc:


# Broken:
# notify.c (notify_add)
# environ.c (environ_free)
# server.c (server_loop)
#


dist_tmux_SOURCES = \
	broken.c \
	cmd-queue.c \
	format-draw.c \
	format.c \
	grid.c \
	input.c \
	layout.c \
	mode-tree.c \
	options-table.c \
	options.c \
	popup.c \
	screen-write.c \
	server-client.c \
	session.c \
	status.c \
	tty-keys.c \
	tty.c \
	window-copy.c \
	window-customize.c \
	window-tree.c \
	tmux-protocol.h \
	tmux.h \
	cmd-parse.y \
	compat.h \
	xmalloc.h
nodist_tmux_SOURCES = osdep-@PLATFORM@.c

# Add compat file for forkpty.
if NEED_FORKPTY
nodist_tmux_SOURCES += compat/forkpty-@PLATFORM@.c
endif

# Add compat file for systemd.
if HAVE_SYSTEMD
nodist_tmux_SOURCES += compat/systemd.c
endif

# Add compat file for utf8proc.
if HAVE_UTF8PROC
nodist_tmux_SOURCES += compat/utf8proc.c
endif

# Enable sixel support.
if ENABLE_SIXEL
dist_tmux_SOURCES += image.c image-sixel.c
endif

if NEED_FUZZING
check_PROGRAMS = fuzz/input-fuzzer
fuzz_input_fuzzer_LDFLAGS = $(FUZZING_LIBS)
fuzz_input_fuzzer_LDADD = $(LDADD) $(tmux_OBJECTS)
endif

# Install tmux.1 in the right format.
install-exec-hook:
	if test x@MANFORMAT@ = xmdoc; then \
		sed -e "s|@SYSCONFDIR@|$(sysconfdir)|g" $(srcdir)/tmux.1 \
			>$(srcdir)/tmux.1.mdoc; \
	else \
		sed -e "s|@SYSCONFDIR@|$(sysconfdir)|g" $(srcdir)/tmux.1| \
			$(AWK) -f $(srcdir)/mdoc2man.awk >$(srcdir)/tmux.1.man; \
	fi
	$(mkdir_p) $(DESTDIR)$(mandir)/man1
	$(INSTALL_DATA) $(srcdir)/tmux.1.@MANFORMAT@ \
		$(DESTDIR)$(mandir)/man1/tmux.1
