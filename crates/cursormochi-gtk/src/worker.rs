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
    Preview(ThemeName, String),
    Thumbnail(ThemeName, String),
}
pub enum Output {
    Scan(Result<Catalog, Error>),
    Preview(Result<Preview, Error>),
    Thumbnail(Result<(Frame, Resolution), Error>),
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
            Job::Scan => {
                repo = repo.refreshed();
                Output::Scan(repo.scan(&cancel))
            }
            Job::Preview(t, r) => Output::Preview(repo.preview(&t, &r, &cancel)),
            Job::Thumbnail(t, r) => {
                Output::Thumbnail(repo.refreshed().preview(&t, &r, &cancel).and_then(|p| {
                    if p.resolution == Resolution::Fallback {
                        return Err(Error::Invalid("Source changed; refresh to verify"));
                    }
                    let v = p
                        .variants
                        .iter()
                        .min_by_key(|v| v.nominal.abs_diff(24))
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
