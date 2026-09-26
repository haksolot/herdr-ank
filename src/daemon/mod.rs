//! The `[[startup]]` process (ADR-6fb76f3a1197): one instance per herdr,
//! a sync at start, then one per burst of herdr events or `events.jsonl`
//! lines, at most one a second and at least one every `sync.poll_seconds`.
//!
//! Journal: stderr only, one line per sync, which `herdr plugin log list`
//! reads back.
//!
//! Between two syncs it tells the user what changed (`crate::notify`).

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::ank;
use crate::config::Config;
use crate::herdr::{self, Event, Sound, Subscription};
use crate::notify::{Notifier, Observed};
use crate::sync::{self, Corpus};

const LOCK_FILE: &str = "daemon.lock";

/// Left in the state dir once the welcome was shown, so it is shown once.
const WELCOME_MARKER: &str = "welcomed";

const WELCOME_TITLE: &str = "ank suit vos agents";

const WELCOME_BODY: &str = "La sidebar peut afficher $ank_task, $ank_expires et $ank_title : \
voir la section Sidebar du README de herdr-ank.";

/// Syncs are coalesced to one per this gap (ADR-6fb76f3a1197).
const MIN_GAP: Duration = Duration::from_secs(1);

/// The backoff after losing herdr's socket grows to this and stays there.
const MAX_BACKOFF: Duration = Duration::from_secs(30);

/// How often `events.jsonl` is looked at.
const TAIL_EVERY: Duration = Duration::from_secs(1);

/// The kinds ADR-6fb76f3a1197 lists that herdr accepts without a pane.
const KINDS: [&str; 7] = [
    "pane.created",
    "pane.closed",
    "pane.agent_detected",
    "worktree.created",
    "worktree.opened",
    "worktree.removed",
    "workspace.closed",
];

/// herdr 0.9.1 refuses this kind without a `pane_id`: one per agent pane.
const PER_PANE_KIND: &str = "pane.agent_status_changed";

/// The single-instance lock: an exclusive `flock` on a file of the plugin's
/// state directory, released when dropped and when the process ends, however
/// it ends.
#[derive(Debug)]
pub struct Lock {
    file: File,
}

impl Lock {
    /// `None` when another process holds the lock.
    pub fn acquire(state_dir: &Path) -> io::Result<Option<Lock>> {
        fs::create_dir_all(state_dir)?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(state_dir.join(LOCK_FILE))?;
        match file.try_lock() {
            Ok(()) => Ok(Some(Lock { file })),
            Err(fs::TryLockError::WouldBlock) => Ok(None),
            Err(fs::TryLockError::Error(err)) => Err(err),
        }
    }
}

impl Drop for Lock {
    /// Unlocks before the close: a child forked meanwhile holds a duplicate of
    /// the descriptor until its exec, and the `flock` follows the open file,
    /// so closing ours alone would leave the lock held a moment longer.
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

/// When the next sync is due. Pure: the caller hands it the clock.
#[derive(Debug, Clone)]
pub struct Schedule {
    min_gap: Duration,
    max_gap: Duration,
    last: Option<Instant>,
    pending: bool,
}

impl Schedule {
    /// A sync is due at once, then after a trigger once `min_gap` has passed
    /// since the last one, and in any case `max_gap` after it.
    pub fn new(min_gap: Duration, max_gap: Duration) -> Self {
        Schedule {
            min_gap,
            max_gap,
            last: None,
            pending: false,
        }
    }

    /// Something moved: an event, a line of `events.jsonl`.
    pub fn trigger(&mut self) {
        self.pending = true;
    }

    pub fn ran(&mut self, now: Instant) {
        self.last = Some(now);
        self.pending = false;
    }

    pub fn due(&self, now: Instant) -> bool {
        self.wait(now).is_zero()
    }

    /// How long until the next sync is due; zero when it is.
    pub fn wait(&self, now: Instant) -> Duration {
        let Some(last) = self.last else {
            return Duration::ZERO;
        };
        let gap = if self.pending {
            self.min_gap
        } else {
            self.max_gap
        };
        (last + gap).saturating_duration_since(now)
    }
}

/// Whole minutes from `now_unix` to `expiry` (`YYYY-MM-DDTHH:MM:SSZ`, as
/// `ank status --json` writes it), rounded up; 0 once it has passed.
pub fn minutes_until(expiry: &str, now_unix: u64) -> Option<u64> {
    let at = parse_utc(expiry)?;
    Some(at.saturating_sub(now_unix).div_ceil(60))
}

fn parse_utc(s: &str) -> Option<u64> {
    let s = s.strip_suffix('Z')?;
    let (date, time) = s.split_once('T')?;
    let mut date = date.splitn(3, '-').map(str::parse::<i64>);
    let (y, m, d) = (date.next()?.ok()?, date.next()?.ok()?, date.next()?.ok()?);
    let time = time.split('.').next()?;
    let mut time = time.splitn(3, ':').map(str::parse::<i64>);
    let (hh, mm, ss) = (time.next()?.ok()?, time.next()?.ok()?, time.next()?.ok()?);
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) || hh > 23 || mm > 59 || ss > 60 {
        return None;
    }
    // Days from civil, Howard Hinnant's algorithm.
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    u64::try_from(days * 86_400 + hh * 3_600 + mm * 60 + ss).ok()
}

/// `herdr-ank daemon`.
pub fn run() -> Result<(), String> {
    // Before anything else: the binary may be replaced while this one starts.
    let binary = Binary::of_plugin();
    let state_dir = env_path("HERDR_PLUGIN_STATE_DIR")?;
    let Some(lock) =
        Lock::acquire(&state_dir).map_err(|e| format!("herdr-ank daemon: lock: {e}"))?
    else {
        // Every [[events]] hook lands here while the daemon lives
        // (ADR-6fb76f3a1197): exit 0 and leave the plugin log alone.
        return Ok(());
    };
    let config = Config::load(&env_path("HERDR_PLUGIN_CONFIG_DIR")?).map_err(|e| e.to_string())?;
    let herdr = herdr::Client::from_env().map_err(|e| format!("herdr-ank daemon: {e}"))?;
    welcome(&herdr, &state_dir);

    let (tx, rx) = mpsc::channel::<()>();
    {
        let herdr = herdr.clone();
        let tx = tx.clone();
        thread::spawn(move || subscribe_forever(&herdr, &tx));
    }
    if let Ok(events) = ank::events_jsonl(&ank::program()) {
        let tx = tx.clone();
        thread::spawn(move || tail_forever(&events, &tx));
    }
    drop(tx);

    let poll = Duration::from_secs(config.sync.poll_seconds.max(1));
    let mut schedule = Schedule::new(MIN_GAP, poll);
    let mut notifier = Notifier::default();
    loop {
        if let Some(binary) = binary.as_ref().filter(|binary| binary.changed()) {
            drop(lock);
            binary.hand_over();
            return Ok(());
        }
        let now = Instant::now();
        if schedule.due(now) {
            match sync_once(&herdr, &config) {
                Ok((reports, observed)) => {
                    eprintln!("herdr-ank daemon: sync, {reports} report(s)");
                    notify(&herdr, &config, &mut notifier, observed);
                }
                Err(err) => eprintln!("herdr-ank daemon: sync failed: {err}"),
            }
            schedule.ran(Instant::now());
            continue;
        }
        match rx.recv_timeout(schedule.wait(now)) {
            Ok(()) => schedule.trigger(),
            Err(RecvTimeoutError::Timeout) => {}
            // Both feeders gone: only the poll is left.
            Err(RecvTimeoutError::Disconnected) => thread::sleep(schedule.wait(Instant::now())),
        }
    }
}

/// The first daemon of a state dir says once what the sidebar can show. The
/// marker is left only after herdr took the notification, so a refused one
/// is tried again at the next start.
fn welcome(herdr: &herdr::Client, state_dir: &Path) {
    let marker = state_dir.join(WELCOME_MARKER);
    if marker.exists() {
        return;
    }
    if let Err(err) = herdr.notification_show(WELCOME_TITLE, Some(WELCOME_BODY), Sound::None) {
        eprintln!("herdr-ank daemon: welcome: {err}");
        return;
    }
    if let Err(err) = fs::write(&marker, "") {
        eprintln!("herdr-ank daemon: welcome marker: {err}");
    }
}

/// The plugin's own binary, `$HERDR_PLUGIN_ROOT/bin/herdr-ank`, as it was
/// when the daemon started. herdr replaces it on an update and leaves this
/// process running, holding the lock the new version's hooks give way to.
struct Binary {
    root: PathBuf,
    path: PathBuf,
    stamp: Option<(u64, Option<SystemTime>)>,
}

impl Binary {
    fn of_plugin() -> Option<Binary> {
        let root = std::env::var_os("HERDR_PLUGIN_ROOT").filter(|root| !root.is_empty())?;
        let root = PathBuf::from(root);
        let path = root
            .join("bin")
            .join(format!("herdr-ank{}", std::env::consts::EXE_SUFFIX));
        let stamp = Self::stamp(&path);
        Some(Binary { root, path, stamp })
    }

    fn stamp(path: &Path) -> Option<(u64, Option<SystemTime>)> {
        fs::metadata(path)
            .ok()
            .map(|meta| (meta.len(), meta.modified().ok()))
    }

    /// Replaced or gone, including gone since before the daemon looked.
    fn changed(&self) -> bool {
        let now = Self::stamp(&self.path);
        now.is_none() || now != self.stamp
    }

    /// Starts the binary now in place, once this process holds no lock, and
    /// leaves it running when this one exits.
    fn hand_over(&self) {
        if !self.path.is_file() {
            eprintln!("herdr-ank daemon: {} is gone, exiting", self.path.display());
            return;
        }
        eprintln!(
            "herdr-ank daemon: {} changed, handing over",
            self.path.display()
        );
        let mut command = std::process::Command::new(&self.path);
        command
            .arg("daemon")
            .current_dir(&self.root)
            .stdin(std::process::Stdio::null());
        // A binary just written may still be open for writing in a child
        // forked meanwhile, until that child execs.
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            match command.spawn() {
                Err(err)
                    if err.kind() == io::ErrorKind::ExecutableFileBusy
                        && Instant::now() < deadline =>
                {
                    thread::sleep(Duration::from_millis(20))
                }
                Err(err) => {
                    eprintln!("herdr-ank daemon: starting {}: {err}", self.path.display());
                    return;
                }
                Ok(_) => return,
            }
        }
    }
}

/// `herdr-ank sync`: one pass, no lock, no notification.
pub fn sync() -> Result<(), String> {
    let config = Config::load(&env_path("HERDR_PLUGIN_CONFIG_DIR")?).map_err(|e| e.to_string())?;
    let herdr = herdr::Client::from_env().map_err(|e| format!("herdr-ank sync: {e}"))?;
    let (reports, _) = sync_once(&herdr, &config).map_err(|e| format!("herdr-ank sync: {e}"))?;
    println!("{reports} report(s)");
    Ok(())
}

/// Tells what changed since the previous sync. A task is looked up as done
/// only once its claim has disappeared.
fn notify(herdr: &herdr::Client, config: &Config, notifier: &mut Notifier, observed: Observed) {
    let is_done = |root: &Path, id: &str| {
        ank::Client::new(root).find(&[id]).is_ok_and(|found| {
            found
                .results
                .iter()
                .any(|r| r.id == id && r.status == "done")
        })
    };
    for told in notifier.observe(observed, config, is_done) {
        if let Err(err) = herdr.notification_show(&told.title, told.body.as_deref(), told.sound) {
            eprintln!("herdr-ank daemon: notification {:?}: {err}", told.title);
        }
    }
}

/// One pass: read herdr and every corpus an agent pane sits in, plan, report.
/// Returns the number of reports and what the notifications compare.
pub fn sync_once(herdr: &herdr::Client, config: &Config) -> Result<(usize, Observed), String> {
    let panes = herdr.pane_list(None).map_err(|e| e.to_string())?;
    let agents = herdr.agent_list().map_err(|e| e.to_string())?;

    // Every pane counts, agent or not: a workspace is reported as soon as
    // one of its panes sits in a corpus (SPEC-43438bbcb5ca).
    let roots: BTreeSet<PathBuf> = panes
        .iter()
        .filter_map(|p| corpus_root(p.cwd.as_deref()?))
        .collect();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    // A corpus ank could not read this pass is left out, and so are its
    // panes: planned without it, they would lose their tokens to a glitch.
    let mut unread = BTreeSet::new();
    let mut corpora: Vec<Corpus> = Vec::new();
    let mut queues = BTreeMap::new();
    for root in roots {
        match read_corpus(&root, now) {
            Ok((corpus, queue)) => {
                queues.insert(root, queue);
                corpora.push(corpus);
            }
            Err(err) => {
                eprintln!("herdr-ank daemon: {}: {err}", root.display());
                unread.insert(root);
            }
        }
    }
    let panes: Vec<_> = panes
        .into_iter()
        .filter(|p| {
            let root = p.cwd.as_deref().and_then(corpus_root);
            !root.is_some_and(|r| unread.contains(&r))
        })
        .collect();

    let reports = sync::plan(&panes, &agents, &corpora, config);
    for report in &reports {
        let tokens: Vec<(&str, &str)> = report
            .tokens
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        let clear: Vec<&str> = report.clear.iter().map(String::as_str).collect();
        if let Err(err) = herdr.report_metadata_as(
            &report.pane_id,
            report.display_agent.as_deref(),
            &tokens,
            &clear,
            Some(report.ttl_ms),
        ) {
            eprintln!("herdr-ank daemon: report on {}: {err}", report.pane_id);
        }
    }
    let workspaces = sync::plan_workspaces(&panes, &corpora, &queues);
    for report in &workspaces {
        let tokens: Vec<(&str, &str)> = report
            .tokens
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        if let Err(err) =
            herdr.report_workspace_metadata(&report.workspace_id, &tokens, &[], Some(report.ttl_ms))
        {
            eprintln!("herdr-ank daemon: report on {}: {err}", report.workspace_id);
        }
    }
    // Notifications keep to the corpora an agent works in, as before the
    // workspace tokens widened what a pass reads.
    let agent_roots: BTreeSet<PathBuf> = panes
        .iter()
        .filter(|p| agents.iter().any(|a| a.pane_id == p.pane_id))
        .filter_map(|p| corpus_root(p.cwd.as_deref()?))
        .collect();
    queues.retain(|root, _| agent_roots.contains(root));
    let observed = Observed::from_sync(&panes, &agents, &corpora, &queues);
    Ok((reports.len(), observed))
}

/// The first directory from `cwd` up that carries a corpus. Only the
/// presence of `.ank` is looked at, nothing inside it (ADR-3cd19cd6acb9).
fn corpus_root(cwd: &Path) -> Option<PathBuf> {
    cwd.ancestors()
        .find(|dir| dir.join(".ank").is_dir())
        .map(Path::to_path_buf)
}

/// The corpus at `root`, and the ratification `queue` its status reports.
fn read_corpus(root: &Path, now_unix: u64) -> Result<(Corpus, u64), ank::AnkError> {
    let client = ank::Client::new(root);
    let in_progress = client.find(&["--status", "in_progress"])?;
    let context = client.context(None)?;
    let status = client.status()?;
    let held = status.claim.iter().map(|c| (&c.id, Some(&c.expires)));
    let elsewhere = status.elsewhere.iter().map(|e| (&e.id, e.expires.as_ref()));
    let expires_in = held
        .chain(elsewhere)
        .filter_map(|(id, expires)| Some((id.clone(), minutes_until(expires?, now_unix)?)))
        .collect();
    let corpus = Corpus {
        root: root.to_path_buf(),
        in_progress,
        context,
        expires_in,
    };
    Ok((corpus, status.queue))
}

/// Subscribes, forwards every event as a trigger, and reconnects: at once
/// when an agent pane may have appeared or gone (its per-pane subscription
/// has to follow), with a backoff up to 30 s when the socket is lost.
fn subscribe_forever(herdr: &herdr::Client, tx: &Sender<()>) {
    let mut backoff = Duration::from_secs(1);
    loop {
        match subscribe_once(herdr, tx) {
            Ok(Ended::Resubscribe) => backoff = Duration::from_secs(1),
            Ok(Ended::ReceiverGone) => return,
            Ok(Ended::Closed) => {
                eprintln!(
                    "herdr-ank daemon: herdr socket closed, retrying in {}s",
                    backoff.as_secs()
                );
                thread::sleep(backoff);
                backoff = (backoff * 2).min(MAX_BACKOFF);
            }
            Err(err) => {
                eprintln!(
                    "herdr-ank daemon: herdr socket: {err}, retrying in {}s",
                    backoff.as_secs()
                );
                thread::sleep(backoff);
                backoff = (backoff * 2).min(MAX_BACKOFF);
            }
        }
    }
}

enum Ended {
    Resubscribe,
    ReceiverGone,
    Closed,
}

fn subscribe_once(herdr: &herdr::Client, tx: &Sender<()>) -> Result<Ended, herdr::HerdrError> {
    let agents = herdr.agent_list()?;
    let mut subscriptions: Vec<Subscription> = KINDS.iter().map(|k| Subscription::new(k)).collect();
    subscriptions.extend(
        agents
            .iter()
            .map(|a| Subscription::for_pane(PER_PANE_KIND, &a.pane_id)),
    );
    for event in herdr.subscribe(&subscriptions)? {
        if tx.send(()).is_err() {
            return Ok(Ended::ReceiverGone);
        }
        if matches!(
            event,
            Event::PaneCreated(_) | Event::PaneClosed { .. } | Event::PaneAgentDetected { .. }
        ) {
            return Ok(Ended::Resubscribe);
        }
    }
    Ok(Ended::Closed)
}

/// A trigger for every growth of `path`; a missing file is only waited for.
/// The lines are not read: that the corpus moved is all they say here.
fn tail_forever(path: &Path, tx: &Sender<()>) {
    let size = |p: &Path| fs::metadata(p).map(|m| m.len()).ok();
    let mut last = size(path);
    loop {
        thread::sleep(TAIL_EVERY);
        let now = size(path);
        let moved = match (last, now) {
            (Some(before), Some(after)) => after != before,
            (None, Some(after)) => after > 0,
            _ => false,
        };
        last = now;
        if moved && tx.send(()).is_err() {
            return;
        }
    }
}

fn env_path(var: &str) -> Result<PathBuf, String> {
    match std::env::var_os(var) {
        Some(value) if !value.is_empty() => Ok(value.into()),
        _ => Err(format!(
            "herdr-ank daemon: {var} is not set; herdr sets it for a plugin"
        )),
    }
}
