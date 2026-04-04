/// Landlock-based sandbox for fuzz targets.
///
/// Restricts the fuzz process to:
/// - Read + execute filesystem access everywhere (no writes)
/// - Write access only to fuzz corpus and artifact directories
///
/// Child processes inherit these restrictions, so even if code under test
/// spawns a shell command, it cannot write to the filesystem outside the
/// fuzz output directories. Requires Linux 5.13+ with Landlock enabled.
use landlock::{
    Access, AccessFs, CompatLevel, Compatible, PathBeneath, PathFd, Ruleset, RulesetAttr,
    RulesetCreatedAttr, ABI,
};
use std::sync::Once;

static SANDBOX_INIT: Once = Once::new();

/// Apply Landlock restrictions to the current process.
/// Safe to call multiple times — only the first call takes effect.
///
/// `target_name` is the fuzz target name (e.g., "cmd_parse"), used to
/// locate the corpus and artifacts directories.
pub fn enable(target_name: &str) {
    SANDBOX_INIT.call_once(|| {
        if let Err(e) = apply_sandbox(target_name) {
            panic!("Landlock sandbox failed to initialize: {e}. Refusing to fuzz without sandbox.");
        }
    });
}

fn apply_sandbox(target_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let abi = ABI::V5;

    // Resolve absolute paths — cargo fuzz uses absolute paths for artifact_prefix
    // but the binary may run from a different cwd.
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let corpus_dir = manifest_dir.join("corpus").join(target_name);
    let artifacts_dir = manifest_dir.join("artifacts").join(target_name);
    std::fs::create_dir_all(&corpus_dir)?;
    std::fs::create_dir_all(&artifacts_dir)?;

    // Handle all filesystem access types — anything not explicitly allowed
    // via add_rule() will be denied. HardRequirement ensures the fuzzer
    // refuses to run if the kernel doesn't support Landlock.
    let status = Ruleset::default()
        .set_compatibility(CompatLevel::HardRequirement)
        .handle_access(AccessFs::from_all(abi))?
        .create()?
        // Allow reads everywhere, plus execute for system binaries and
        // rustup toolchains (libFuzzer spawns llvm-symbolizer for crash reports).
        .add_rule(PathBeneath::new(
            PathFd::new("/")?,
            AccessFs::ReadFile | AccessFs::ReadDir | AccessFs::Execute,
        ))?
        // Allow read+write to corpus dir (fuzzer reads and saves inputs here).
        .add_rule(PathBeneath::new(
            PathFd::new(&corpus_dir)?,
            AccessFs::from_all(abi),
        ))?
        // Allow read+write to artifacts dir (fuzzer saves crash cases here).
        .add_rule(PathBeneath::new(
            PathFd::new(&artifacts_dir)?,
            AccessFs::from_all(abi),
        ))?
        .restrict_self()?;

    eprintln!("Landlock sandbox enabled: {status:?}");
    Ok(())
}
