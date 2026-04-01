use std::env;
use std::io::{BufReader, Read as _, Write as _};
use std::os::fd::{AsRawFd, OwnedFd};
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

static SOCKET_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Returns the path to the tmux server binary.
///
/// Priority: `TMUX_SERVER_BIN` env var, then the cargo-built binary.
pub fn server_bin() -> PathBuf {
    if let Ok(bin) = env::var("TMUX_SERVER_BIN") {
        return PathBuf::from(bin);
    }
    cargo_bin()
}

/// Returns the path to the tmux client binary.
///
/// Priority: `TMUX_CLIENT_BIN` env var, then the cargo-built binary.
pub fn client_bin() -> PathBuf {
    if let Ok(bin) = env::var("TMUX_CLIENT_BIN") {
        return PathBuf::from(bin);
    }
    cargo_bin()
}

/// Returns the path to the cargo-built tmux-rs binary.
fn cargo_bin() -> PathBuf {
    // cargo sets this for integration tests
    env!("CARGO_BIN_EXE_tmux-rs").into()
}

#[allow(dead_code)]
/// A test harness that manages a tmux server with an isolated socket.
///
/// Each instance gets a unique socket path so tests can run in parallel
/// without interfering with each other or any running tmux session.
///
/// Uses `-S <absolute_path>` instead of `-L <name>` so that mixed
/// binary testing (tmux-rs client ↔ C tmux server) works regardless
/// of each binary's default socket directory.
pub struct TmuxTestHarness {
    socket_path: String,
    server_started: bool,
}

#[allow(dead_code)]
impl TmuxTestHarness {
    /// Create a new harness with a unique socket path. Does not start a server yet.
    pub fn new() -> Self {
        let id = SOCKET_COUNTER.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        let socket_path = format!("/tmp/tmux-test-{pid}-{id}");
        Self {
            socket_path,
            server_started: false,
        }
    }

    /// Start a detached session with the given arguments.
    /// This is typically the first call — it starts the server implicitly.
    ///
    /// Extra args are appended to `new-session -d`.
    pub fn new_session(&mut self) -> TmuxCommandBuilder<'_> {
        self.server_started = true;
        self.server_cmd().args(["-f/dev/null", "new-session", "-d"])
    }

    /// Run a tmux command using the *client* binary against this harness's server.
    pub fn cmd(&self) -> TmuxCommandBuilder<'_> {
        self.client_cmd()
    }

    /// Run `display-message -p` with the given format string and return the output.
    pub fn query(&self, format: &str) -> String {
        self.cmd()
            .args(["display-message", "-p", format])
            .run()
            .stdout_trimmed()
    }

    /// Run `capture-pane -p` and return the pane contents.
    pub fn capture_pane(&self) -> String {
        self.cmd()
            .args(["capture-pane", "-p"])
            .run()
            .stdout_trimmed()
    }

    /// Run `capture-pane -pt<target>` and return the pane contents.
    pub fn capture_pane_target(&self, target: &str) -> String {
        self.cmd()
            .args(["capture-pane", "-p", "-t", target])
            .run()
            .stdout_trimmed()
    }

    /// Send keys to the current pane.
    /// `args` can include flags like `-t <target>` or `-X <command>`.
    pub fn send_keys(&self, args: &[&str]) -> TmuxResult {
        let mut all_args = vec!["send-keys".to_string()];
        all_args.extend(args.iter().map(|s| s.to_string()));
        self.cmd().args(all_args).run()
    }

    /// Run a tmux command that chains multiple commands with `\;`.
    /// Each element of `commands` is a separate tmux command.
    pub fn run_shell_cmd(&self, shell_args: &str) -> TmuxResult {
        let bin = client_bin();
        let full_cmd = format!(
            "{} -L{} {}",
            bin.display(),
            self.socket_path,
            shell_args,
        );
        let output = Command::new("sh")
            .args(["-c", &full_cmd])
            .stdin(Stdio::null())
            .output()
            .expect("failed to run sh");
        TmuxResult { output }
    }

    /// Pipe input to `tmux -C` (control mode) and return all output lines.
    pub fn control_mode(&self, input: &str) -> TmuxResult {
        let bin = client_bin();
        let child = Command::new(bin)
            .args(["-S", &self.socket_path, "-C", "attach"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to spawn control mode client");

        use std::io::Write;
        let mut stdin = child.stdin.unwrap();
        stdin.write_all(input.as_bytes()).ok();
        drop(stdin);

        let output = Command::new("true") // dummy — we just need the Output struct
            .output()
            .unwrap();

        // Read with timeout
        let stdout_handle = child.stdout.unwrap();
        let stderr_handle = child.stderr.unwrap();

        let stdout_reader = std::thread::spawn(move || {
            let mut buf = Vec::new();
            std::io::Read::read_to_end(&mut BufReader::new(stdout_handle), &mut buf).ok();
            buf
        });
        let stderr_reader = std::thread::spawn(move || {
            let mut buf = Vec::new();
            std::io::Read::read_to_end(&mut BufReader::new(stderr_handle), &mut buf).ok();
            buf
        });

        let stdout = stdout_reader.join().unwrap_or_default();
        let stderr = stderr_reader.join().unwrap_or_default();

        TmuxResult {
            output: Output {
                status: output.status,
                stdout,
                stderr,
            },
        }
    }

    /// Build a command using the server binary with `-L <socket>`.
    fn server_cmd(&self) -> TmuxCommandBuilder<'_> {
        TmuxCommandBuilder {
            harness: self,
            bin: server_bin(),
            extra_args: Vec::new(),
            env_vars: Vec::new(),
            stdin_data: None,
        }
    }

    /// Build a command using the client binary with `-L <socket>`.
    fn client_cmd(&self) -> TmuxCommandBuilder<'_> {
        TmuxCommandBuilder {
            harness: self,
            bin: client_bin(),
            extra_args: Vec::new(),
            env_vars: Vec::new(),
            stdin_data: None,
        }
    }

    /// Wait until the server is responding (has-session succeeds or timeout).
    pub fn wait_ready(&self, timeout: Duration) {
        let start = Instant::now();
        while start.elapsed() < timeout {
            let result = self.cmd().args(["has-session"]).run();
            if result.output.status.success() {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        panic!(
            "tmux server on socket '{}' did not become ready within {:?}",
            self.socket_path, timeout
        );
    }

    /// Kill the server.
    pub fn kill_server(&self) {
        let _ = self.cmd().args(["kill-server"]).run();
    }

    pub fn socket_path(&self) -> &str {
        &self.socket_path
    }
}

impl Drop for TmuxTestHarness {
    fn drop(&mut self) {
        if self.server_started {
            self.kill_server();
        }
        // Clean up the socket file
        let _ = std::fs::remove_file(&self.socket_path);
    }
}

#[allow(dead_code)]
/// Builder for a tmux command invocation.
pub struct TmuxCommandBuilder<'a> {
    harness: &'a TmuxTestHarness,
    bin: PathBuf,
    extra_args: Vec<String>,
    env_vars: Vec<(String, String)>,
    stdin_data: Option<String>,
}

#[allow(dead_code)]
impl<'a> TmuxCommandBuilder<'a> {
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.extra_args.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn env(mut self, key: &str, val: &str) -> Self {
        self.env_vars.push((key.to_string(), val.to_string()));
        self
    }

    pub fn stdin(mut self, data: &str) -> Self {
        self.stdin_data = Some(data.to_string());
        self
    }

    pub fn run(self) -> TmuxResult {
        let mut cmd = Command::new(&self.bin);
        cmd.args(["-S", &self.harness.socket_path]);
        cmd.args(&self.extra_args);
        cmd.env("TERM", "screen");

        for (k, v) in &self.env_vars {
            cmd.env(k, v);
        }

        if let Some(ref data) = self.stdin_data {
            cmd.stdin(Stdio::piped());
            let mut child = cmd
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("failed to spawn tmux");
            use std::io::Write;
            child
                .stdin
                .as_mut()
                .unwrap()
                .write_all(data.as_bytes())
                .ok();
            let output = child.wait_with_output().expect("failed to wait on tmux");
            TmuxResult { output }
        } else {
            cmd.stdin(Stdio::null());
            let output = cmd
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .expect("failed to run tmux");
            TmuxResult { output }
        }
    }
}

/// Result of a tmux command invocation.
pub struct TmuxResult {
    pub output: Output,
}

#[allow(dead_code)]
impl TmuxResult {
    pub fn success(&self) -> bool {
        self.output.status.success()
    }

    pub fn stdout_str(&self) -> String {
        String::from_utf8_lossy(&self.output.stdout).into_owned()
    }

    pub fn stderr_str(&self) -> String {
        String::from_utf8_lossy(&self.output.stderr).into_owned()
    }

    pub fn stdout_trimmed(&self) -> String {
        self.stdout_str().trim_end().to_string()
    }

    pub fn stdout_lines(&self) -> Vec<String> {
        self.stdout_str().lines().map(String::from).collect()
    }

    pub fn assert_success(&self) {
        assert!(
            self.success(),
            "tmux command failed with status {:?}\nstderr: {}",
            self.output.status,
            self.stderr_str()
        );
    }

    pub fn assert_failure(&self) {
        assert!(
            !self.success(),
            "tmux command succeeded but expected failure"
        );
    }
}

/// An attached tmux client backed by a real PTY.
///
/// Spawns `tmux attach` (or `new-session`) with its stdio connected to a
/// pseudo-terminal we control. The test harness reads/writes the master side
/// to interact with the full terminal output — status bar, pane contents, etc.
///
/// ```text
/// ┌─────────────┐     ┌──────────┐     ┌──────────────┐
/// │ Test harness │────▸│ PTY pair │────▸│ tmux client  │
/// │ (master fd)  │◂────│ m ←→ s   │◂────│ (slave=stdio)│
/// └─────────────┘     └──────────┘     └──────────────┘
///                                             │ unix socket
///                                      ┌──────────────┐
///                                      │ tmux server   │
///                                      └──────────────┘
/// ```
#[allow(dead_code)]
pub struct PtyClient {
    master: OwnedFd,
    child: std::process::Child,
}

#[allow(dead_code)]
impl PtyClient {
    /// Attach to an existing session on the given harness.
    ///
    /// The client is spawned as `tmux -L<socket> attach`, connected to a PTY
    /// with the given dimensions.
    pub fn attach(harness: &TmuxTestHarness, cols: u16, rows: u16) -> Self {
        Self::spawn_with_args(harness, &["attach"], cols, rows)
    }

    /// Create a new session and attach to it.
    pub fn new_session(harness: &TmuxTestHarness, cols: u16, rows: u16) -> Self {
        Self::spawn_with_args(harness, &["-f/dev/null", "new-session"], cols, rows)
    }

    fn spawn_with_args(
        harness: &TmuxTestHarness,
        args: &[&str],
        cols: u16,
        rows: u16,
    ) -> Self {
        let winsize = nix::pty::Winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };

        let pty = nix::pty::openpty(Some(&winsize), None)
            .expect("openpty failed");

        let slave_fd = pty.slave;

        // Make the master non-blocking so reads don't hang
        let master_raw = pty.master.as_raw_fd();
        unsafe {
            let flags = libc::fcntl(master_raw, libc::F_GETFL);
            libc::fcntl(master_raw, libc::F_SETFL, flags | libc::O_NONBLOCK);
        }

        // Dup the slave fd for each stdio stream — from_raw_fd takes ownership
        let slave_stdin = slave_fd.try_clone().expect("dup slave for stdin");
        let slave_stdout = slave_fd.try_clone().expect("dup slave for stdout");
        let slave_stderr = slave_fd.try_clone().expect("dup slave for stderr");
        drop(slave_fd); // close original in parent

        let bin = client_bin();
        let child = Command::new(&bin)
            .args(["-S", harness.socket_path()])
            .args(args)
            .env("TERM", "screen")
            .stdin(Stdio::from(slave_stdin))
            .stdout(Stdio::from(slave_stdout))
            .stderr(Stdio::from(slave_stderr))
            .spawn()
            .expect("failed to spawn tmux client");

        let mut client = PtyClient {
            master: pty.master,
            child,
        };

        // Wait for the client to connect and render initial output
        client.wait_for_content(Duration::from_secs(5));

        client
    }

    /// Write raw bytes to the client's terminal input.
    pub fn write(&mut self, data: &[u8]) {
        let mut f = std::fs::File::from(self.master.try_clone().unwrap());
        f.write_all(data).expect("failed to write to PTY master");
    }

    /// Write a string to the client's terminal input.
    pub fn write_str(&mut self, s: &str) {
        self.write(s.as_bytes());
    }

    /// Send a key by name (translates common names to escape sequences).
    pub fn send_key(&mut self, key: &str) {
        let bytes: &[u8] = match key {
            "Enter" => b"\r",
            "Escape" | "Esc" => b"\x1b",
            "Tab" => b"\t",
            "Up" => b"\x1b[A",
            "Down" => b"\x1b[B",
            "Right" => b"\x1b[C",
            "Left" => b"\x1b[D",
            "C-b" => b"\x02", // tmux prefix key
            "C-c" => b"\x03",
            "C-d" => b"\x04",
            "C-l" => b"\x0c",
            _ => {
                // Single character
                if key.len() == 1 {
                    self.write(key.as_bytes());
                    return;
                }
                panic!("unknown key name: {key}");
            }
        };
        self.write(bytes);
    }

    /// Read all currently available output from the terminal.
    /// Returns the raw bytes (including escape sequences).
    pub fn read_raw(&mut self) -> Vec<u8> {
        let mut buf = vec![0u8; 65536];
        let mut output = Vec::new();
        let mut f = std::fs::File::from(self.master.try_clone().unwrap());
        loop {
            match f.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => output.extend_from_slice(&buf[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => panic!("read from PTY master failed: {e}"),
            }
        }
        output
    }

    /// Read output and strip ANSI escape sequences, returning plain text.
    pub fn read_screen(&mut self) -> String {
        let raw = self.read_raw();
        strip_ansi_escapes(&raw)
    }

    /// Wait until some output is available, then read and return it.
    /// Useful after sending a command to wait for the response.
    pub fn wait_and_read(&mut self, timeout: Duration) -> String {
        let start = Instant::now();
        loop {
            let output = self.read_screen();
            if !output.is_empty() {
                return output;
            }
            if start.elapsed() > timeout {
                return String::new();
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// Wait until the screen contains a specific string.
    pub fn wait_for_text(&mut self, needle: &str, timeout: Duration) -> String {
        let start = Instant::now();
        let mut accumulated = String::new();
        loop {
            let chunk = self.read_screen();
            accumulated.push_str(&chunk);
            if accumulated.contains(needle) {
                return accumulated;
            }
            if start.elapsed() > timeout {
                panic!(
                    "timed out waiting for '{needle}' in PTY output.\nGot so far:\n{accumulated}"
                );
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// Wait until any content appears on the PTY.
    fn wait_for_content(&mut self, timeout: Duration) {
        let start = Instant::now();
        loop {
            let raw = self.read_raw();
            if !raw.is_empty() {
                return;
            }
            if start.elapsed() > timeout {
                panic!("timed out waiting for initial PTY output");
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// Check if the child process is still running.
    pub fn is_alive(&mut self) -> bool {
        self.child
            .try_wait()
            .expect("failed to check child status")
            .is_none()
    }
}

impl Drop for PtyClient {
    fn drop(&mut self) {
        // Send q to exit any mode, then detach
        let _ = self.write(b"\x02d"); // C-b d (prefix + detach)
        std::thread::sleep(Duration::from_millis(100));
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Strip ANSI escape sequences from raw terminal output.
fn strip_ansi_escapes(input: &[u8]) -> String {
    let mut output = Vec::new();
    let mut i = 0;
    while i < input.len() {
        if input[i] == 0x1b {
            i += 1;
            if i >= input.len() {
                break;
            }
            match input[i] {
                b'[' => {
                    // CSI sequence: ESC [ ... final_byte
                    i += 1;
                    while i < input.len() && !(0x40..=0x7e).contains(&input[i]) {
                        i += 1;
                    }
                    if i < input.len() {
                        i += 1; // skip final byte
                    }
                }
                b']' => {
                    // OSC sequence: ESC ] ... ST (or BEL)
                    i += 1;
                    while i < input.len() && input[i] != 0x07 {
                        if input[i] == 0x1b && i + 1 < input.len() && input[i + 1] == b'\\' {
                            i += 2;
                            break;
                        }
                        i += 1;
                    }
                    if i < input.len() && input[i] == 0x07 {
                        i += 1;
                    }
                }
                b'(' | b')' => {
                    // Character set designation: ESC ( X or ESC ) X
                    i += 1;
                    if i < input.len() {
                        i += 1;
                    }
                }
                _ => {
                    // Other ESC sequences: ESC + one byte
                    i += 1;
                }
            }
        } else if input[i] == b'\r' {
            // Skip carriage returns (keep newlines)
            i += 1;
        } else if input[i] >= 0x20 || input[i] == b'\n' || input[i] == b'\t' {
            output.push(input[i]);
            i += 1;
        } else {
            // Skip other control characters
            i += 1;
        }
    }
    String::from_utf8_lossy(&output).into_owned()
}
