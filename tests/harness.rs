use std::env;
use std::io::BufReader;
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
/// Each instance gets a unique socket name so tests can run in parallel
/// without interfering with each other or any running tmux session.
pub struct TmuxTestHarness {
    socket_name: String,
    server_started: bool,
}

#[allow(dead_code)]
impl TmuxTestHarness {
    /// Create a new harness with a unique socket name. Does not start a server yet.
    pub fn new() -> Self {
        let id = SOCKET_COUNTER.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        let socket_name = format!("tmux-rs-test-{pid}-{id}");
        Self {
            socket_name,
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
            self.socket_name,
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
            .args(["-L", &self.socket_name, "-C", "attach"])
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
            self.socket_name, timeout
        );
    }

    /// Kill the server.
    pub fn kill_server(&self) {
        let _ = self.cmd().args(["kill-server"]).run();
    }

    pub fn socket_name(&self) -> &str {
        &self.socket_name
    }
}

impl Drop for TmuxTestHarness {
    fn drop(&mut self) {
        if self.server_started {
            self.kill_server();
        }
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
        cmd.args(["-L", &self.harness.socket_name]);
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
