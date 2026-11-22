use clap::Parser;
use fs2::FileExt;
use skill_sync::{apply, detect, plan, PlanError, SyncReport};
use skill_sync_core::fingerprint;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(
    about = "Conservative, dependency-free synchronization of user Agent Skills.",
    after_help = "Whole folders are linked. Conflicts stay untouched; replaced copies are backed up. Exit 2 means unresolved conflicts or broken links."
)]
struct Args {
    #[arg(default_value = "sync", value_enum)]
    command: Command,
    #[arg(long)]
    dry_run: bool,
    #[arg(long, value_name = "HARNESS:SKILL")]
    promote: Option<String>,
    #[arg(long)]
    json: bool,
    #[arg(long)]
    verbose: bool,
    #[arg(long)]
    home: Option<PathBuf>,
    /// Directory to fingerprint when `command` is `digest`.
    #[arg(value_name = "PATH")]
    path: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, clap::ValueEnum)]
enum Command {
    #[default]
    Sync,
    Status,
    Doctor,
    #[value(name = "dry-run")]
    DryRun,
    Digest,
}

fn main() {
    let args = Args::parse();
    if matches!(args.command, Command::Digest) {
        let path = args.path.unwrap_or_else(|| {
            eprintln!("skill-sync digest: missing PATH");
            std::process::exit(1);
        });
        match fingerprint(&path) {
            Ok(digest) => {
                println!("{digest}");
                std::process::exit(0);
            }
            Err(err) => {
                eprintln!("skill-sync: {err}");
                std::process::exit(1);
            }
        }
    }
    let home = args
        .home
        .clone()
        .unwrap_or_else(|| dirs_home().unwrap_or_else(|| PathBuf::from("/")));
    // Match Python: expanduser/absolute only — do not resolve /private on macOS.
    let use_binaries = args.home.is_none();
    let (roots, shared) = detect(&home, use_binaries);
    let readonly = args.dry_run
        || matches!(
            args.command,
            Command::Status | Command::Doctor | Command::DryRun
        );
    let state = home.join(".local/state/skill-sync");
    let code = match run(&home, &roots, shared, readonly, &args, &state) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("skill-sync: {err}");
            1
        }
    };
    std::process::exit(code);
}

fn run(
    home: &Path,
    roots: &indexmap::IndexMap<String, PathBuf>,
    shared: Vec<String>,
    readonly: bool,
    args: &Args,
    state: &Path,
) -> Result<i32, PlanError> {
    let mut lock_file: Option<File> = None;
    if !readonly {
        std::fs::create_dir_all(state)?;
        let lock = OpenOptions::new()
            .create(true)
            .append(true)
            .open(state.join("sync.lock"))?;
        lock.try_lock_exclusive().map_err(|_| {
            PlanError::Msg("Another skill-sync process holds the lock".into())
        })?;
        lock_file = Some(lock);
    }

    let mut report = plan(home, roots, args.promote.as_deref())?;
    report.shared_consumers = shared;
    report.applied = !readonly;
    report.backup = if readonly {
        None
    } else {
        apply(&report, state)?
    };
    if !readonly {
        std::fs::write(
            state.join("last-sync.json"),
            serde_json::to_string_pretty(&report)? + "\n",
        )?;
    }

    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_text(&report, roots, readonly, args.verbose);
    }

    let _ = lock_file;
    Ok(if !report.conflicts.is_empty() || !report.broken_links.is_empty() {
        2
    } else {
        0
    })
}

fn print_text(
    report: &SyncReport,
    roots: &indexmap::IndexMap<String, PathBuf>,
    readonly: bool,
    verbose: bool,
) {
    println!("Canonical: {}", report.canonical);
    let names: Vec<_> = roots.keys().cloned().collect();
    println!("Harness roots: {}", names.join(", "));
    println!(
        "Shared ~/.agents/skills consumers: {}",
        if report.shared_consumers.is_empty() {
            "none".into()
        } else {
            report.shared_consumers.join(", ")
        }
    );
    println!(
        "Skills: {}; {} actions: {}; conflicts: {}; broken links: {}; managed excluded: {}",
        report.skills.len(),
        if report.applied { "applied" } else { "planned" },
        report.actions.len(),
        report.conflicts.len(),
        report.broken_links.len(),
        report.excluded_managed.len()
    );
    if let Some(pstack) = report.plugins.get("pstack") {
        if let (Some(version), Some(skills)) = (
            pstack.get("version").and_then(|v| v.as_str()),
            pstack.get("skills").and_then(|s| s.as_array()),
        ) {
            println!(
                "Upstream plugin: pstack {} skills: {}",
                version,
                skills.len()
            );
        }
    }
    if !report.repairable_broken_links.is_empty() {
        println!(
            "Broken links repaired/planned: {}",
            report.repairable_broken_links.len()
        );
    }
    if verbose {
        for action in &report.actions {
            println!(
                "{} {} <- {}",
                action.op.to_uppercase(),
                action.path,
                action.source
            );
        }
    }
    for (slug, reasons) in &report.conflicts {
        println!("CONFLICT {slug} {}", reasons.join("; "));
    }
    for path in &report.broken_links {
        println!("BROKEN {path}");
    }
    if let Some(backup) = &report.backup {
        println!("Preserved backups: {backup}");
    }
    if !report.conflicts.is_empty() {
        println!("Resolve intentionally: skill-sync --dry-run --promote HARNESS:SKILL, then remove --dry-run.");
    }
    if readonly {
        println!("No changes made. Use --json for each skill/link state; --verbose for actions.");
    }
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}
