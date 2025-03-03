/* $OpenBSD$ */

/*
 * Copyright (c) 2019 Nicholas Marriott <nicholas.marriott@gmail.com>
 *
 * Permission to use, copy, modify, and distribute this software for any
 * purpose with or without fee is hereby granted, provided that the above
 * copyright notice and this permission notice appear in all copies.
 *
 * THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
 * WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
 * MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
 * ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
 * WHATSOEVER RESULTING FROM LOSS OF MIND, USE, DATA OR PROFITS, WHETHER
 * IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING
 * OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
 */

%{

#include <sys/types.h>

#include <ctype.h>
#include <errno.h>
#include <pwd.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <wchar.h>

#include "tmux.h"

int			 yylex(void);
int			 yyparse(void);
static int printflike(1,2)	 yyerror(const char *, ...);

static char			*yylex_token(int);
static char			*yylex_format(void);

struct cmd_parse_scope {
	int				 flag;
	TAILQ_ENTRY (cmd_parse_scope)	 entry;
};

enum cmd_parse_argument_type {
	CMD_PARSE_STRING,
	CMD_PARSE_COMMANDS,
	CMD_PARSE_PARSED_COMMANDS
};

struct cmd_parse_argument {
	enum cmd_parse_argument_type	 type;
	char				*string;
	struct cmd_parse_commands	*commands;
	struct cmd_list			*cmdlist;

	TAILQ_ENTRY(cmd_parse_argument)	 entry;
};
TAILQ_HEAD(cmd_parse_arguments, cmd_parse_argument);

struct cmd_parse_command {
	u_int				 line;
	struct cmd_parse_arguments	 arguments;

	TAILQ_ENTRY(cmd_parse_command)	 entry;
};
TAILQ_HEAD(cmd_parse_commands, cmd_parse_command);

struct cmd_parse_state {
	FILE				*f;

	const char			*buf;
	size_t				 len;
	size_t				 off;

	int				 condition;
	int				 eol;
	int				 eof;
	struct cmd_parse_input		*input;
	u_int				 escapes;

	char				*error;
	struct cmd_parse_commands	*commands;

	struct cmd_parse_scope		*scope;
	TAILQ_HEAD(, cmd_parse_scope)	 stack;
};
extern struct cmd_parse_state parse_state;

char	*cmd_parse_get_error(const char *, u_int, const char *);
void	 cmd_parse_free_command(struct cmd_parse_command *);
struct cmd_parse_commands *cmd_parse_new_commands(void);
void	 cmd_parse_free_commands(struct cmd_parse_commands *);
void	 cmd_parse_build_commands(struct cmd_parse_commands *, struct cmd_parse_input *, struct cmd_parse_result *);
void	 cmd_parse_print_commands(struct cmd_parse_input *, struct cmd_list *);

struct cmd_parse_commands * cmd_parse_do_buffer(const char *buf, size_t len, struct cmd_parse_input *pi, char **cause);
struct cmd_parse_result * cmd_parse_from_string(const char *s, struct cmd_parse_input *pi);
enum cmd_parse_status cmd_parse_and_insert(const char *s, struct cmd_parse_input *pi, struct cmdq_item *after, struct cmdq_state *state, char **error);
enum cmd_parse_status cmd_parse_and_append(const char *s, struct cmd_parse_input *pi, struct client *c, struct cmdq_state *state, char **error);
struct cmd_parse_result * cmd_parse_from_buffer(const void *buf, size_t len, struct cmd_parse_input *pi);
struct cmd_parse_result * cmd_parse_from_arguments(struct args_value *values, u_int count, struct cmd_parse_input *pi);

%}

%union
{
	char					 *token;
	struct cmd_parse_arguments		 *arguments;
	struct cmd_parse_argument		 *argument;
	int					  flag;
	struct {
		int				  flag;
		struct cmd_parse_commands	 *commands;
	} elif;
	struct cmd_parse_commands		 *commands;
	struct cmd_parse_command		 *command;
}

%token ERROR
%token HIDDEN
%token IF
%token ELSE
%token ELIF
%token ENDIF
%token <token> FORMAT TOKEN EQUALS

%type <token> expanded format
%type <arguments> arguments
%type <argument> argument
%type <flag> if_open if_elif
%type <elif> elif elif1
%type <commands> argument_statements statements statement
%type <commands> commands condition condition1
%type <command> command

%%

lines		: /* empty */
		| statements
		{
			struct cmd_parse_state	*ps = &parse_state;

			ps->commands = $1;
		}

statements	: statement '\n'
		{
			$$ = $1;
		}
		| statements statement '\n'
		{
			$$ = $1;
			TAILQ_CONCAT($$, $2, entry);
			free($2);
		}

statement	: /* empty */
		{
			$$ = xmalloc (sizeof *$$);
			TAILQ_INIT($$);
		}
		| hidden_assignment
		{
			$$ = xmalloc (sizeof *$$);
			TAILQ_INIT($$);
		}
		| condition
		{
			struct cmd_parse_state	*ps = &parse_state;

			if (ps->scope == NULL || ps->scope->flag)
				$$ = $1;
			else {
				$$ = cmd_parse_new_commands();
				cmd_parse_free_commands($1);
			}
		}
		| commands
		{
			struct cmd_parse_state	*ps = &parse_state;

			if (ps->scope == NULL || ps->scope->flag)
				$$ = $1;
			else {
				$$ = cmd_parse_new_commands();
				cmd_parse_free_commands($1);
			}
		}

format		: FORMAT
		{
			$$ = $1;
		}
		| TOKEN
		{
			$$ = $1;
		}

expanded	: format
		{
			struct cmd_parse_state	*ps = &parse_state;
			struct cmd_parse_input	*pi = ps->input;
			struct format_tree	*ft;
			struct client		*c = pi->c;
			struct cmd_find_state	*fsp;
			struct cmd_find_state	 fs;
			int			 flags = FORMAT_NOJOBS;

			if (cmd_find_valid_state(&pi->fs))
				fsp = &pi->fs;
			else {
				cmd_find_from_client(&fs, c, 0);
				fsp = &fs;
			}
			ft = format_create(NULL, pi->item, FORMAT_NONE, flags);
			format_defaults(ft, c, fsp->s, fsp->wl, fsp->wp);

			$$ = format_expand(ft, $1);
			format_free(ft);
			free($1);
		}

optional_assignment	: /* empty */
			| assignment

assignment	: EQUALS
		{
			struct cmd_parse_state	*ps = &parse_state;
			int			 flags = ps->input->flags;

			if ((~flags & CMD_PARSE_PARSEONLY) &&
			    (ps->scope == NULL || ps->scope->flag))
				environ_put(global_environ, $1, 0);
			free($1);
		}

hidden_assignment : HIDDEN EQUALS
		{
			struct cmd_parse_state	*ps = &parse_state;
			int			 flags = ps->input->flags;

			if ((~flags & CMD_PARSE_PARSEONLY) &&
			    (ps->scope == NULL || ps->scope->flag))
				environ_put(global_environ, $2, ENVIRON_HIDDEN);
			free($2);
		}

if_open		: IF expanded
		{
			struct cmd_parse_state	*ps = &parse_state;
			struct cmd_parse_scope	*scope;

			scope = xmalloc(sizeof *scope);
			$$ = scope->flag = format_true($2);
			free($2);

			if (ps->scope != NULL)
				TAILQ_INSERT_HEAD(&ps->stack, ps->scope, entry);
			ps->scope = scope;
		}

if_else		: ELSE
		{
			struct cmd_parse_state	*ps = &parse_state;
			struct cmd_parse_scope	*scope;

			scope = xmalloc(sizeof *scope);
			scope->flag = !ps->scope->flag;

			free(ps->scope);
			ps->scope = scope;
		}

if_elif		: ELIF expanded
		{
			struct cmd_parse_state	*ps = &parse_state;
			struct cmd_parse_scope	*scope;

			scope = xmalloc(sizeof *scope);
			$$ = scope->flag = format_true($2);
			free($2);

			free(ps->scope);
			ps->scope = scope;
		}

if_close	: ENDIF
		{
			struct cmd_parse_state	*ps = &parse_state;

			free(ps->scope);
			ps->scope = TAILQ_FIRST(&ps->stack);
			if (ps->scope != NULL)
				TAILQ_REMOVE(&ps->stack, ps->scope, entry);
		}

condition	: if_open '\n' statements if_close
		{
			if ($1)
				$$ = $3;
			else {
				$$ = cmd_parse_new_commands();
				cmd_parse_free_commands($3);
			}
		}
		| if_open '\n' statements if_else '\n' statements if_close
		{
			if ($1) {
				$$ = $3;
				cmd_parse_free_commands($6);
			} else {
				$$ = $6;
				cmd_parse_free_commands($3);
			}
		}
		| if_open '\n' statements elif if_close
		{
			if ($1) {
				$$ = $3;
				cmd_parse_free_commands($4.commands);
			} else if ($4.flag) {
				$$ = $4.commands;
				cmd_parse_free_commands($3);
			} else {
				$$ = cmd_parse_new_commands();
				cmd_parse_free_commands($3);
				cmd_parse_free_commands($4.commands);
			}
		}
		| if_open '\n' statements elif if_else '\n' statements if_close
		{
			if ($1) {
				$$ = $3;
				cmd_parse_free_commands($4.commands);
				cmd_parse_free_commands($7);
			} else if ($4.flag) {
				$$ = $4.commands;
				cmd_parse_free_commands($3);
				cmd_parse_free_commands($7);
			} else {
				$$ = $7;
				cmd_parse_free_commands($3);
				cmd_parse_free_commands($4.commands);
			}
		}

elif		: if_elif '\n' statements
		{
			if ($1) {
				$$.flag = 1;
				$$.commands = $3;
			} else {
				$$.flag = 0;
				$$.commands = cmd_parse_new_commands();
				cmd_parse_free_commands($3);
			}
		}
		| if_elif '\n' statements elif
		{
			if ($1) {
				$$.flag = 1;
				$$.commands = $3;
				cmd_parse_free_commands($4.commands);
			} else if ($4.flag) {
				$$.flag = 1;
				$$.commands = $4.commands;
				cmd_parse_free_commands($3);
			} else {
				$$.flag = 0;
				$$.commands = cmd_parse_new_commands();
				cmd_parse_free_commands($3);
				cmd_parse_free_commands($4.commands);
			}
		}

commands	: command
		{
			struct cmd_parse_state	*ps = &parse_state;

			$$ = cmd_parse_new_commands();
			if (!TAILQ_EMPTY(&$1->arguments) &&
			    (ps->scope == NULL || ps->scope->flag))
				TAILQ_INSERT_TAIL($$, $1, entry);
			else
				cmd_parse_free_command($1);
		}
		| commands ';'
		{
			$$ = $1;
		}
		| commands ';' condition1
		{
			$$ = $1;
			TAILQ_CONCAT($$, $3, entry);
			free($3);
		}
		| commands ';' command
		{
			struct cmd_parse_state	*ps = &parse_state;

			if (!TAILQ_EMPTY(&$3->arguments) &&
			    (ps->scope == NULL || ps->scope->flag)) {
				$$ = $1;
				TAILQ_INSERT_TAIL($$, $3, entry);
			} else {
				$$ = cmd_parse_new_commands();
				cmd_parse_free_commands($1);
				cmd_parse_free_command($3);
			}
		}
		| condition1
		{
			$$ = $1;
		}

command		: assignment
		{
			struct cmd_parse_state	*ps = &parse_state;

			$$ = xcalloc(1, sizeof *$$);
			$$->line = ps->input->line;
			TAILQ_INIT(&$$->arguments);
		}
		| optional_assignment TOKEN
		{
			struct cmd_parse_state		*ps = &parse_state;
			struct cmd_parse_argument	*arg;

			$$ = xcalloc(1, sizeof *$$);
			$$->line = ps->input->line;
			TAILQ_INIT(&$$->arguments);

			arg = xcalloc(1, sizeof *arg);
			arg->type = CMD_PARSE_STRING;
			arg->string = $2;
			TAILQ_INSERT_HEAD(&$$->arguments, arg, entry);
		}
		| optional_assignment TOKEN arguments
		{
			struct cmd_parse_state		*ps = &parse_state;
			struct cmd_parse_argument	*arg;

			$$ = xcalloc(1, sizeof *$$);
			$$->line = ps->input->line;
			TAILQ_INIT(&$$->arguments);

			TAILQ_CONCAT(&$$->arguments, $3, entry);
			free($3);

			arg = xcalloc(1, sizeof *arg);
			arg->type = CMD_PARSE_STRING;
			arg->string = $2;
			TAILQ_INSERT_HEAD(&$$->arguments, arg, entry);
		}

condition1	: if_open commands if_close
		{
			if ($1)
				$$ = $2;
			else {
				$$ = cmd_parse_new_commands();
				cmd_parse_free_commands($2);
			}
		}
		| if_open commands if_else commands if_close
		{
			if ($1) {
				$$ = $2;
				cmd_parse_free_commands($4);
			} else {
				$$ = $4;
				cmd_parse_free_commands($2);
			}
		}
		| if_open commands elif1 if_close
		{
			if ($1) {
				$$ = $2;
				cmd_parse_free_commands($3.commands);
			} else if ($3.flag) {
				$$ = $3.commands;
				cmd_parse_free_commands($2);
			} else {
				$$ = cmd_parse_new_commands();
				cmd_parse_free_commands($2);
				cmd_parse_free_commands($3.commands);
			}
		}
		| if_open commands elif1 if_else commands if_close
		{
			if ($1) {
				$$ = $2;
				cmd_parse_free_commands($3.commands);
				cmd_parse_free_commands($5);
			} else if ($3.flag) {
				$$ = $3.commands;
				cmd_parse_free_commands($2);
				cmd_parse_free_commands($5);
			} else {
				$$ = $5;
				cmd_parse_free_commands($2);
				cmd_parse_free_commands($3.commands);
			}
		}

elif1		: if_elif commands
		{
			if ($1) {
				$$.flag = 1;
				$$.commands = $2;
			} else {
				$$.flag = 0;
				$$.commands = cmd_parse_new_commands();
				cmd_parse_free_commands($2);
			}
		}
		| if_elif commands elif1
		{
			if ($1) {
				$$.flag = 1;
				$$.commands = $2;
				cmd_parse_free_commands($3.commands);
			} else if ($3.flag) {
				$$.flag = 1;
				$$.commands = $3.commands;
				cmd_parse_free_commands($2);
			} else {
				$$.flag = 0;
				$$.commands = cmd_parse_new_commands();
				cmd_parse_free_commands($2);
				cmd_parse_free_commands($3.commands);
			}
		}

arguments	: argument
		{
			$$ = xcalloc(1, sizeof *$$);
			TAILQ_INIT($$);

			TAILQ_INSERT_HEAD($$, $1, entry);
		}
		| argument arguments
		{
			TAILQ_INSERT_HEAD($2, $1, entry);
			$$ = $2;
		}

argument	: TOKEN
		{
			$$ = xcalloc(1, sizeof *$$);
			$$->type = CMD_PARSE_STRING;
			$$->string = $1;
		}
		| EQUALS
		{
			$$ = xcalloc(1, sizeof *$$);
			$$->type = CMD_PARSE_STRING;
			$$->string = $1;
		}
		| '{' argument_statements
		{
			$$ = xcalloc(1, sizeof *$$);
			$$->type = CMD_PARSE_COMMANDS;
			$$->commands = $2;
		}

argument_statements	: statement '}'
			{
				$$ = $1;
			}
			| statements statement '}'
			{
				$$ = $1;
				TAILQ_CONCAT($$, $2, entry);
				free($2);
			}

%%

static int printflike(1, 2)
yyerror(const char *fmt, ...)
{
	struct cmd_parse_state	*ps = &parse_state;
	struct cmd_parse_input	*pi = ps->input;
	va_list			 ap;
	char			*error;

	if (ps->error != NULL)
		return (0);

	va_start(ap, fmt);
	xvasprintf(&error, fmt, ap);
	va_end(ap);

	ps->error = cmd_parse_get_error(pi->file, pi->line, error);
	free(error);
	return (0);
}

static int
yylex_is_var(char ch, int first)
{
	if (ch == '=')
		return (0);
	if (first && isdigit((u_char)ch))
		return (0);
	return (isalnum((u_char)ch) || ch == '_');
}

static void
yylex_append(char **buf, size_t *len, const char *add, size_t addlen)
{
	if (addlen > SIZE_MAX - 1 || *len > SIZE_MAX - 1 - addlen)
		fatalx("buffer is too big");
	*buf = xrealloc(*buf, (*len) + 1 + addlen);
	memcpy((*buf) + *len, add, addlen);
	(*len) += addlen;
}

static void
yylex_append1(char **buf, size_t *len, char add)
{
	yylex_append(buf, len, &add, 1);
}

static int
yylex_getc1(void)
{
	struct cmd_parse_state	*ps = &parse_state;
	int			 ch;

	if (ps->f != NULL)
		ch = getc(ps->f);
	else {
		if (ps->off == ps->len)
			ch = EOF;
		else
			ch = ps->buf[ps->off++];
	}
	return (ch);
}

static void
yylex_ungetc(int ch)
{
	struct cmd_parse_state	*ps = &parse_state;

	if (ps->f != NULL)
		ungetc(ch, ps->f);
	else if (ps->off > 0 && ch != EOF)
		ps->off--;
}

static int
yylex_getc(void)
{
	struct cmd_parse_state	*ps = &parse_state;
	int			 ch;

	if (ps->escapes != 0) {
		ps->escapes--;
		return ('\\');
	}
	for (;;) {
		ch = yylex_getc1();
		if (ch == '\\') {
			ps->escapes++;
			continue;
		}
		if (ch == '\n' && (ps->escapes % 2) == 1) {
			ps->input->line++;
			ps->escapes--;
			continue;
		}

		if (ps->escapes != 0) {
			yylex_ungetc(ch);
			ps->escapes--;
			return ('\\');
		}
		return (ch);
	}
}

static char *
yylex_get_word(int ch)
{
	char	*buf;
	size_t	 len;

	len = 0;
	buf = xmalloc(1);

	do
		yylex_append1(&buf, &len, ch);
	while ((ch = yylex_getc()) != EOF && strchr(" \t\n", ch) == NULL);
	yylex_ungetc(ch);

	buf[len] = '\0';
	log_debug("%s: %s", __func__, buf);
	return (buf);
}

int
yylex(void)
{
	struct cmd_parse_state	*ps = &parse_state;
	char			*token, *cp;
	int			 ch, next, condition;

	if (ps->eol)
		ps->input->line++;
	ps->eol = 0;

	condition = ps->condition;
	ps->condition = 0;

	for (;;) {
		ch = yylex_getc();

		if (ch == EOF) {
			/*
			 * Ensure every file or string is terminated by a
			 * newline. This keeps the parser simpler and avoids
			 * having to add a newline to each string.
			 */
			if (ps->eof)
				break;
			ps->eof = 1;
			return ('\n');
		}

		if (ch == ' ' || ch == '\t') {
			/*
			 * Ignore whitespace.
			 */
			continue;
		}

		if (ch == '\r') {
			/*
			 * Treat \r\n as \n.
			 */
			ch = yylex_getc();
			if (ch != '\n') {
				yylex_ungetc(ch);
				ch = '\r';
			}
		}
		if (ch == '\n') {
			/*
			 * End of line. Update the line number.
			 */
			ps->eol = 1;
			return ('\n');
		}

		if (ch == ';' || ch == '{' || ch == '}') {
			/*
			 * A semicolon or { or } is itself.
			 */
			return (ch);
		}

		if (ch == '#') {
			/*
			 * #{ after a condition opens a format; anything else
			 * is a comment, ignore up to the end of the line.
			 */
			next = yylex_getc();
			if (condition && next == '{') {
				yylval.token = yylex_format();
				if (yylval.token == NULL)
					return (ERROR);
				return (FORMAT);
			}
			while (next != '\n' && next != EOF)
				next = yylex_getc();
			if (next == '\n') {
				ps->input->line++;
				return ('\n');
			}
			continue;
		}

		if (ch == '%') {
			/*
			 * % is a condition unless it is all % or all numbers,
			 * then it is a token.
			 */
			yylval.token = yylex_get_word('%');
			for (cp = yylval.token; *cp != '\0'; cp++) {
				if (*cp != '%' && !isdigit((u_char)*cp))
					break;
			}
			if (*cp == '\0')
				return (TOKEN);
			ps->condition = 1;
			if (strcmp(yylval.token, "%hidden") == 0) {
				free(yylval.token);
				return (HIDDEN);
			}
			if (strcmp(yylval.token, "%if") == 0) {
				free(yylval.token);
				return (IF);
			}
			if (strcmp(yylval.token, "%else") == 0) {
				free(yylval.token);
				return (ELSE);
			}
			if (strcmp(yylval.token, "%elif") == 0) {
				free(yylval.token);
				return (ELIF);
			}
			if (strcmp(yylval.token, "%endif") == 0) {
				free(yylval.token);
				return (ENDIF);
			}
			free(yylval.token);
			return (ERROR);
		}

		/*
		 * Otherwise this is a token.
		 */
		token = yylex_token(ch);
		if (token == NULL)
			return (ERROR);
		yylval.token = token;

		if (strchr(token, '=') != NULL && yylex_is_var(*token, 1)) {
			for (cp = token + 1; *cp != '='; cp++) {
				if (!yylex_is_var(*cp, 0))
					break;
			}
			if (*cp == '=')
				return (EQUALS);
		}
		return (TOKEN);
	}
	return (0);
}

static char *
yylex_format(void)
{
	char	*buf;
	size_t	 len;
	int	 ch, brackets = 1;

	len = 0;
	buf = xmalloc(1);

	yylex_append(&buf, &len, "#{", 2);
	for (;;) {
		if ((ch = yylex_getc()) == EOF || ch == '\n')
			goto error;
		if (ch == '#') {
			if ((ch = yylex_getc()) == EOF || ch == '\n')
				goto error;
			if (ch == '{')
				brackets++;
			yylex_append1(&buf, &len, '#');
		} else if (ch == '}') {
			if (brackets != 0 && --brackets == 0) {
				yylex_append1(&buf, &len, ch);
				break;
			}
		}
		yylex_append1(&buf, &len, ch);
	}
	if (brackets != 0)
		goto error;

	buf[len] = '\0';
	log_debug("%s: %s", __func__, buf);
	return (buf);

error:
	free(buf);
	return (NULL);
}

static int
yylex_token_escape(char **buf, size_t *len)
{
	int	 ch, type, o2, o3, mlen;
	u_int	 size, i, tmp;
	char	 s[9], m[MB_LEN_MAX];

	ch = yylex_getc();

	if (ch >= '4' && ch <= '7') {
		yyerror("invalid octal escape");
		return (0);
	}
	if (ch >= '0' && ch <= '3') {
		o2 = yylex_getc();
		if (o2 >= '0' && o2 <= '7') {
			o3 = yylex_getc();
			if (o3 >= '0' && o3 <= '7') {
				ch = 64 * (ch - '0') +
				      8 * (o2 - '0') +
					  (o3 - '0');
				yylex_append1(buf, len, ch);
				return (1);
			}
		}
		yyerror("invalid octal escape");
		return (0);
	}

	switch (ch) {
	case EOF:
		return (0);
	case 'a':
		ch = '\a';
		break;
	case 'b':
		ch = '\b';
		break;
	case 'e':
		ch = '\033';
		break;
	case 'f':
		ch = '\f';
		break;
	case 's':
		ch = ' ';
		break;
	case 'v':
		ch = '\v';
		break;
	case 'r':
		ch = '\r';
		break;
	case 'n':
		ch = '\n';
		break;
	case 't':
		ch = '\t';
		break;
	case 'u':
		type = 'u';
		size = 4;
		goto unicode;
	case 'U':
		type = 'U';
		size = 8;
		goto unicode;
	}

	yylex_append1(buf, len, ch);
	return (1);

unicode:
	for (i = 0; i < size; i++) {
		ch = yylex_getc();
		if (ch == EOF || ch == '\n')
			return (0);
		if (!isxdigit((u_char)ch)) {
			yyerror("invalid \\%c argument", type);
			return (0);
		}
		s[i] = ch;
	}
	s[i] = '\0';

	if ((size == 4 && sscanf(s, "%4x", &tmp) != 1) ||
	    (size == 8 && sscanf(s, "%8x", &tmp) != 1)) {
		yyerror("invalid \\%c argument", type);
		return (0);
	}
	mlen = wctomb(m, tmp);
	if (mlen <= 0 || mlen > (int)sizeof m) {
		yyerror("invalid \\%c argument", type);
		return (0);
	}
	yylex_append(buf, len, m, mlen);
	return (1);
}

static int
yylex_token_variable(char **buf, size_t *len)
{
	struct environ_entry	*envent;
	int			 ch, brackets = 0;
	char			 name[1024];
	size_t			 namelen = 0;
	const char		*value;

	ch = yylex_getc();
	if (ch == EOF)
		return (0);
	if (ch == '{')
		brackets = 1;
	else {
		if (!yylex_is_var(ch, 1)) {
			yylex_append1(buf, len, '$');
			yylex_ungetc(ch);
			return (1);
		}
		name[namelen++] = ch;
	}

	for (;;) {
		ch = yylex_getc();
		if (brackets && ch == '}')
			break;
		if (ch == EOF || !yylex_is_var(ch, 0)) {
			if (!brackets) {
				yylex_ungetc(ch);
				break;
			}
			yyerror("invalid environment variable");
			return (0);
		}
		if (namelen == (sizeof name) - 2) {
			yyerror("environment variable is too long");
			return (0);
		}
		name[namelen++] = ch;
	}
	name[namelen] = '\0';

	envent = environ_find(global_environ, name);
	if (envent != NULL && envent->value != NULL) {
		value = envent->value;
		log_debug("%s: %s -> %s", __func__, name, value);
		yylex_append(buf, len, value, strlen(value));
	}
	return (1);
}

static int
yylex_token_tilde(char **buf, size_t *len)
{
	struct environ_entry	*envent;
	int			 ch;
	char			 name[1024];
	size_t			 namelen = 0;
	struct passwd		*pw;
	const char		*home = NULL;

	for (;;) {
		ch = yylex_getc();
		if (ch == EOF || strchr("/ \t\n\"'", ch) != NULL) {
			yylex_ungetc(ch);
			break;
		}
		if (namelen == (sizeof name) - 2) {
			yyerror("user name is too long");
			return (0);
		}
		name[namelen++] = ch;
	}
	name[namelen] = '\0';

	if (*name == '\0') {
		envent = environ_find(global_environ, "HOME");
		if (envent != NULL && *envent->value != '\0')
			home = envent->value;
		else if ((pw = getpwuid(getuid())) != NULL)
			home = pw->pw_dir;
	} else {
		if ((pw = getpwnam(name)) != NULL)
			home = pw->pw_dir;
	}
	if (home == NULL)
		return (0);

	log_debug("%s: ~%s -> %s", __func__, name, home);
	yylex_append(buf, len, home, strlen(home));
	return (1);
}

static char *
yylex_token(int ch)
{
	char			*buf;
	size_t			 len;
	enum { START,
	       NONE,
	       DOUBLE_QUOTES,
	       SINGLE_QUOTES }	 state = NONE, last = START;

	len = 0;
	buf = xmalloc(1);

	for (;;) {
		/* EOF or \n are always the end of the token. */
		if (ch == EOF) {
			log_debug("%s: end at EOF", __func__);
			break;
		}
		if (state == NONE && ch == '\r') {
			ch = yylex_getc();
			if (ch != '\n') {
				yylex_ungetc(ch);
				ch = '\r';
			}
		}
		if (state == NONE && ch == '\n') {
			log_debug("%s: end at EOL", __func__);
			break;
		}

		/* Whitespace or ; or } ends a token unless inside quotes. */
		if (state == NONE && (ch == ' ' || ch == '\t')) {
			log_debug("%s: end at WS", __func__);
			break;
		}
		if (state == NONE && (ch == ';' || ch == '}')) {
			log_debug("%s: end at %c", __func__, ch);
			break;
		}

		/*
		 * Spaces and comments inside quotes after \n are removed but
		 * the \n is left.
		 */
		if (ch == '\n' && state != NONE) {
			yylex_append1(&buf, &len, '\n');
			while ((ch = yylex_getc()) == ' ' || ch == '\t')
				/* nothing */;
			if (ch != '#')
				continue;
			ch = yylex_getc();
			if (strchr(",#{}:", ch) != NULL) {
				yylex_ungetc(ch);
				ch = '#';
			} else {
				while ((ch = yylex_getc()) != '\n' && ch != EOF)
					/* nothing */;
			}
			continue;
		}

		/* \ ~ and $ are expanded except in single quotes. */
		if (ch == '\\' && state != SINGLE_QUOTES) {
			if (!yylex_token_escape(&buf, &len))
				goto error;
			goto skip;
		}
		if (ch == '~' && last != state && state != SINGLE_QUOTES) {
			if (!yylex_token_tilde(&buf, &len))
				goto error;
			goto skip;
		}
		if (ch == '$' && state != SINGLE_QUOTES) {
			if (!yylex_token_variable(&buf, &len))
				goto error;
			goto skip;
		}
		if (ch == '}' && state == NONE)
			goto error;  /* unmatched (matched ones were handled) */

		/* ' and " starts or end quotes (and is consumed). */
		if (ch == '\'') {
			if (state == NONE) {
				state = SINGLE_QUOTES;
				goto next;
			}
			if (state == SINGLE_QUOTES) {
				state = NONE;
				goto next;
			}
		}
		if (ch == '"') {
			if (state == NONE) {
				state = DOUBLE_QUOTES;
				goto next;
			}
			if (state == DOUBLE_QUOTES) {
				state = NONE;
				goto next;
			}
		}

		/* Otherwise add the character to the buffer. */
		yylex_append1(&buf, &len, ch);

	skip:
		last = state;

	next:
		ch = yylex_getc();
	}
	yylex_ungetc(ch);

	buf[len] = '\0';
	log_debug("%s: %s", __func__, buf);
	return (buf);

error:
	free(buf);
	return (NULL);
}
