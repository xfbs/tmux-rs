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
