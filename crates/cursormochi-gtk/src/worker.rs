use cursormochi_app::{Catalog, ThemeRepository};
use cursormochi_core::{Error, Frame, Preview, Resolution, ThemeName};
use cursormochi_platform::Repository;
use std::sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicU64, Ordering},
    mpsc::{self, Receiver, SyncSender},
};
#[derive(Clone)]
pub enum Job {
    Scan,
    Audit(ThemeName),
    Preview(ThemeName, String),
    Thumbnail(ThemeName, String, u32),
    Trial(ThemeName, u32),
    ImportTrial(Vec<(usize, cursormochi_app::import::Asset)>, u32),
}
pub enum Output {
    Audit(Result<cursormochi_app::current_cursor::Audit, Error>),
    Scan(Result<Catalog, Error>),
    Preview(Result<Preview, Error>),
    Thumbnail(Result<(Frame, Resolution), Error>),
    Trial(Result<cursormochi_app::trial::TrialSet, Error>),
}
type Slot = Arc<(Mutex<Option<(u64, Job)>>, Condvar)>;
#[derive(Clone)]
pub struct Worker {
    slot: Slot,
    generation: Arc<AtomicU64>,
}
impl Worker {
    pub fn new(repo: Repository) -> (Self, Receiver<(u64, Output)>) {
        let slot: Slot = Arc::new((Mutex::new(None), Condvar::new()));
        let generation = Arc::new(AtomicU64::new(0));
        let (tx, rx) = mpsc::sync_channel(1);
        let s = slot.clone();
        let g = generation.clone();
        std::thread::spawn(move || run(repo, s, g, tx));
        (Self { slot, generation }, rx)
    }
    pub fn cancel(&self) {
        self.generation.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut slot) = self.slot.0.lock() {
            *slot = None;
        }
    }
    pub fn submit(&self, job: Job) -> u64 {
        let id = self.generation.fetch_add(1, Ordering::Relaxed) + 1;
        if let Ok(mut slot) = self.slot.0.lock() {
            *slot = Some((id, job));
            self.slot.1.notify_one();
        }
        id
    }
}
fn run(mut repo: Repository, slot: Slot, g: Arc<AtomicU64>, tx: SyncSender<(u64, Output)>) {
    loop {
        let Ok(mut guard) = slot.0.lock() else { return };
        while guard.is_none() {
            let Ok(next) = slot.1.wait(guard) else { return };
            guard = next;
        }
        let Some((id, job)) = guard.take() else {
            continue;
        };
        drop(guard);
        let cancel = || g.load(Ordering::Relaxed) != id;
        let output = match job {
            Job::Audit(t) => Output::Audit(cursormochi_app::current_cursor::audit(
                &repo.refreshed(),
                &t,
                &cancel,
            )),
            Job::Scan => {
                repo = repo.refreshed();
                Output::Scan(repo.scan(&cancel))
            }
            Job::ImportTrial(assets, size) => {
                Output::Trial(cursormochi_app::import::trial(&assets, size, &cancel))
            }
            Job::Trial(t, size) => Output::Trial(cursormochi_app::trial::load(
                &repo.refreshed(),
                &t,
                size,
                &cancel,
            )),
            Job::Preview(t, r) => Output::Preview(repo.preview(&t, &r, &cancel)),
            Job::Thumbnail(t, r, pixels) => {
                Output::Thumbnail(repo.refreshed().preview(&t, &r, &cancel).and_then(|p| {
                    if p.resolution == Resolution::Fallback {
                        return Err(Error::Invalid("Source changed; refresh to verify"));
                    }
                    let v = cursormochi_app::browser::thumbnail_variant(&p, pixels)
                        .ok_or(Error::Missing)?;
                    Ok((
                        v.frames.first().ok_or(Error::Missing)?.clone(),
                        p.resolution,
                    ))
                }))
            }
        };
        if !cancel() && tx.send((id, output)).is_err() {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cursormochi_platform::Paths;
    use std::time::{Duration, Instant};
    #[test]
    fn trial_replacements_deliver_only_current_theme_and_size_to_consumer() {
        let root =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
        let (w, rx) = Worker::new(Repository::new(Paths::fixture(root)));
        w.submit(Job::Trial(ThemeName::new("Mochi-Light").unwrap(), 16));
        w.submit(Job::Trial(ThemeName::new("Mochi-Inherited").unwrap(), 32));
        let latest = w.submit(Job::Trial(ThemeName::new("Mochi-Motion").unwrap(), 48));
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let (id, out) = rx
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .unwrap();
            if id != latest {
                continue;
            }
            let Output::Trial(Ok(data)) = out else {
                panic!("trial load failed")
            };
            assert_eq!(data.theme.as_str(), "Mochi-Motion");
            assert_eq!(data.size, 48);
            assert_eq!(
                data.roles[0].1.as_ref().unwrap().variants[0].frames.len(),
                4
            );
            break;
        }
    }
    #[test]
    fn rapid_selection_delivers_latest_generation() {
        let root =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
        let (w, rx) = Worker::new(Repository::new(Paths::fixture(root)));
        w.submit(Job::Preview(
            ThemeName::new("Mochi-Light").unwrap(),
            "left_ptr".into(),
        ));
        w.submit(Job::Preview(
            ThemeName::new("Mochi-Broken").unwrap(),
            "left_ptr".into(),
        ));
        let latest = w.submit(Job::Preview(
            ThemeName::new("Mochi-Motion").unwrap(),
            "left_ptr".into(),
        ));
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let (id, result) = rx
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .unwrap();
            if id != latest {
                continue;
            }
            let Output::Preview(Ok(p)) = result else {
                panic!("latest preview failed")
            };
            assert_eq!(p.requested.as_str(), "Mochi-Motion");
            assert_eq!(p.variants[0].frames.len(), 4);
            break;
        }
    }
}
