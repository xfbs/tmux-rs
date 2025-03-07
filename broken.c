#include <sys/ioctl.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/un.h>
#include <sys/wait.h>

#include <errno.h>
#include <fcntl.h>
#include <fnmatch.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <termios.h>
#include <time.h>
#include <unistd.h>

#include "tmux.h"

RB_HEAD(environ, environ_entry);
int environ_cmp(struct environ_entry *, struct environ_entry *);
RB_GENERATE_STATIC(environ, environ_entry, entry, environ_cmp);

void environ_free(struct environ *env) {
  struct environ_entry *envent, *envent1;

  RB_FOREACH_SAFE(envent, environ, env, envent1) {
    RB_REMOVE(environ, env, envent);
    free(envent->name);
    free(envent->value);
    free(envent);
  }
  free(env);
}


enum cmd_retval notify_callback(struct cmdq_item *item, void *data);
struct notify_entry {
  const char *name;
  struct cmd_find_state fs;
  struct format_tree *formats;

  struct client *client;
  struct session *session;
  struct window *window;
  int pane;
  const char *pbname;
};

void notify_add(const char *name, struct cmd_find_state *fs, struct client *c, struct session *s, struct window *w, struct window_pane *wp, const char *pbname);
void notify_add(const char *name, struct cmd_find_state *fs, struct client *c, struct session *s, struct window *w, struct window_pane *wp, const char *pbname) {
  struct notify_entry *ne;
  struct cmdq_item *item;

  item = cmdq_running(NULL);
  if (item != NULL && (cmdq_get_flags(item) & CMDQ_STATE_NOHOOKS)) {
    return;
  }

  ne = xcalloc(1, sizeof *ne);
  ne->name = xstrdup(name);

  ne->client = c;
  ne->session = s;
  ne->window = w;
  ne->pane = (wp != NULL ? (int)wp->id : -1);
  ne->pbname = (pbname != NULL ? xstrdup(pbname) : NULL);

  ne->formats = format_create(NULL, NULL, 0, FORMAT_NOJOBS);
  format_add(ne->formats, "hook", "%s", name);
  if (c != NULL) {
    format_add(ne->formats, "hook_client", "%s", c->name);
  }
  if (s != NULL) {
    format_add(ne->formats, "hook_session", "$%u", s->id);
    format_add(ne->formats, "hook_session_name", "%s", s->name);
  }
  if (w != NULL) {
    format_add(ne->formats, "hook_window", "@%u", w->id);
    format_add(ne->formats, "hook_window_name", "%s", w->name);
  }
  if (wp != NULL) {
    format_add(ne->formats, "hook_pane", "%%%d", wp->id);
  }
  format_log_debug(ne->formats, __func__);

  if (c != NULL) {
    c->references++;
  }
  if (s != NULL) {
    session_add_ref(s, __func__);
  }
  if (w != NULL) {
    window_add_ref(w, __func__);
  }

  cmd_find_copy_state(&ne->fs, fs);
  if (ne->fs.s != NULL) {
    session_add_ref(ne->fs.s, __func__);
  }

  cmdq_append(NULL, cmdq_get_callback(notify_callback, ne));
}

extern int server_exit;
int server_loop(void);
int server_loop(void) {
  struct client *c;
  u_int items;

  current_time = time(NULL);

  do {
    items = cmdq_next(NULL);
    TAILQ_FOREACH(c, &clients, entry) {
      if (c->flags & CLIENT_IDENTIFIED) {
        items += cmdq_next(c);
      }
    }
  } while (items != 0);

  server_client_loop();

  if (!options_get_number(global_options, "exit-empty") && !server_exit) {
    return 0;
  }

  if (!options_get_number(global_options, "exit-unattached")) {
    if (!RB_EMPTY(&sessions)) {
      return 0;
    }
  }

  TAILQ_FOREACH(c, &clients, entry) {
    if (c->session != NULL) {
      return 0;
    }
  }

  cmd_wait_for_flush();
  if (!TAILQ_EMPTY(&clients)) {
    return 0;
  }

  if (job_still_running()) {
    return 0;
  }

  return 1;
}
