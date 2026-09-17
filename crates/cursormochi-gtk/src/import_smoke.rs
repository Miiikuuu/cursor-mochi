//! Isolated original fixtures and a unique QA directory, never a user theme root.
use super::{import::Importer, ui::State};
use gtk::prelude::*;
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
    time::{Duration, Instant},
};
#[derive(Default)]
pub struct ImportSmoke {
    stage: u8,
    at: Option<Instant>,
    ready_frame: Option<i64>,
    preview_frames: Option<usize>,
}
impl ImportSmoke {
    pub fn done(&self) -> bool {
        self.stage == 17
    }
    fn next(&mut self) {
        self.stage += 1;
        self.at = Some(Instant::now());
        self.ready_frame = None;
        self.preview_frames = None;
    }
    // Worker completion can change the page in this same tick. Wait for real
    // frame-clock progress before snapshotting the newly allocated content.
    fn layout_ready(&mut self, window: &gtk::ApplicationWindow) -> bool {
        let Some(clock) = window.frame_clock() else {
            return false;
        };
        let current = clock.frame_counter();
        let first = *self.ready_frame.get_or_insert(current);
        window.queue_draw();
        current > first + 1
    }
    pub fn tick(&mut self, t: &Rc<Importer>, state: &Rc<RefCell<State>>, target: &Path) {
        let f = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/import-fixtures")
            .canonicalize()
            .expect("fixtures");
        let elapsed = self.at.map(|i| i.elapsed()).unwrap_or_default();
        assert!(
            elapsed < Duration::from_secs(8),
            "import smoke stage {}: {}",
            self.stage,
            t.status.text()
        );
        match self.stage {
            0 => {
                t.present();
                t.load(f.join("normal.cur"));
                self.next();
            }
            1 if !t.is_busy() && t.preview.has_data() => {
                assert_eq!(t.pages.visible_child_name().as_deref(), Some("review"));
                assert!(t.try_button.is_sensitive());
                assert!(!t.install.is_visible());
                assert!(!t.name.text().is_empty());
                assert_eq!(t.preview.zoom.value(), 1.);
                assert_eq!(t.assignments().len(), 1);
                assert_eq!(t.assignments()[0].0, 0);
                assert!(
                    t.review_summary
                        .text()
                        .contains("System files: 1/34 covered · 33 missing")
                );
                assert!(
                    t.install_overview
                        .text()
                        .contains("System files: 1/34 covered · 33 missing")
                );
                assert!(t.summary.text().contains("Column resize (col-resize)"));
                assert!(t.summary.text().contains("Grab (grab)"));
                assert!(!t.install.is_sensitive());
                t.load(f.join("busy.ani"));
                self.next();
            }
            2 if !t.is_busy() && t.preview.has_data() => {
                assert_eq!(t.assignments()[0].0, 3);
                assert!(t.summary.text().contains("Working in background"));
                assert!(t.summary.text().contains("rounded to 17 ms"));
                let first = *self.preview_frames.get_or_insert(t.preview.frames_shown());
                // Check animation while this window is exposed; X11 may stop
                // drawing it once the nonmodal trial covers it completely.
                if t.preview.frames_shown() <= first + 1 {
                    return;
                }
                t.try_button.emit_clicked();
                self.next();
            }
            3 if t.trial.region_cursor(3).is_some() && elapsed > Duration::from_millis(450) => {
                assert!(t.trial.frames_shown() > 2);
                let cursor = t.trial.region_cursor(3).unwrap();
                assert!(cursor.name().is_none());
                assert!(cursor.texture().is_some());
                assert!(t.trial.region_cursor(4).is_none());
                // The trial check is finished. Keep the import window exposed
                // so X11 can allocate and paint subsequent review screenshots.
                t.trial.window.close();
                t.window.present();
                t.load(f.join("unsupported.cur"));
                self.next();
            }
            4 if !t.is_busy() => {
                assert!(!t.preview.has_data());
                assert!(t.preview.warning.is_visible());
                assert!(t.trial.region_cursor(3).is_none());
                t.load(f.join("theme.zip"));
                self.next();
            }
            5 if !t.is_busy() && t.preview.has_data() => {
                assert_eq!(t.assignments().len(), 3);
                assert_eq!(t.roots.active_id().as_deref(), Some(""));
                let choice = t.rows.borrow()[0].1.clone();
                // Scrolling the file list over a closed role menu must never
                // silently change a mapping or disable Continue.
                let before = choice.active();
                let controllers = choice.observe_controllers();
                for i in 0..controllers.n_items() {
                    if let Some(scroll) = controllers
                        .item(i)
                        .and_then(|c| c.downcast::<gtk::EventControllerScroll>().ok())
                        && scroll.propagation_phase() != gtk::PropagationPhase::None
                    {
                        scroll.emit_by_name::<bool>("scroll", &[&0f64, &1f64]);
                    }
                }
                assert_eq!(
                    choice.active(),
                    before,
                    "file-list scrolling changed a role mapping"
                );
                assert!(t.next.is_sensitive());
                choice.set_active(Some(1)); // Busy and Normal would collide.
                assert!(!t.next.is_sensitive());
                assert!(t.requirements.text().contains("multiple files"));
                let hint = choice
                    .parent()
                    .unwrap()
                    .parent()
                    .unwrap()
                    .last_child()
                    .unwrap()
                    .downcast::<gtk::Label>()
                    .unwrap();
                assert!(hint.text().contains("Normal is assigned more than once"));
                // Explicit recovery restores suggestions and invalidates any
                // earlier confirmation without installing or touching settings.
                t.confirm.set_active(true);
                t.reset_roles.emit_clicked();
                assert!(!t.confirm.is_active());
                assert_eq!(t.rows.borrow()[0].1.active(), Some(4));
                assert!(t.next.is_sensitive());
                t.next.emit_clicked();
                assert_eq!(t.pages.visible_child_name().as_deref(), Some("install"));
                assert!(t.requirements.text().contains("confirmation"));
                t.name.set_text("invalid/name");
                assert!(t.requirements.text().contains("valid theme name"));
                assert!(!t.install.is_sensitive());
                t.name.set_text("Import-Smoke");
                t.confirm.set_active(true);
                assert!(t.install.is_sensitive());
                t.install.emit_clicked();
                self.next();
            }
            6 if !t.is_busy() && !state.borrow().scanning => {
                if !self.layout_ready(&t.window) {
                    return;
                }
                assert!(
                    t.status.text().starts_with("Installed "),
                    "{}",
                    t.status.text()
                );
                assert_eq!(
                    state.borrow().browser.selected.as_ref().unwrap().as_str(),
                    "Import-Smoke"
                );
                assert_eq!(t.next.label().as_deref(), Some("Done"));
                assert!(!t.install.is_visible());
                super::trial_smoke::capture_main(&t.window, "target/qa/import-installed.png");
                assert_eq!(
                    state.borrow().catalog.themes[0].name.as_str(),
                    "Import-Smoke",
                    "installed user themes should lead the refreshed list"
                );
                assert!(target.join("Import-Smoke/cursors/watch").is_file());
                assert!(!target.join("Import-Smoke/install.inf").exists());
                assert!(state.borrow().controller.undo.is_none());
                t.confirm.set_active(true);
                t.install.emit_clicked();
                self.next();
            }
            7 if !t.is_busy() => {
                assert!(t.status.text().contains("already exists"));
                assert!(!t.source_status.text().contains("stopped"));
                assert_eq!(std::fs::read_dir(target).unwrap().count(), 1);
                // Modifying a mapping invalidates its earlier explicit confirmation.
                t.confirm.set_active(true);
                let choice = t.rows.borrow()[0].1.clone();
                choice.set_active(Some(5));
                assert!(!t.confirm.is_active());
                assert_eq!(t.assignments()[0].0, 4);
                assert!(!t.install.is_sensitive());
                assert!(
                    t.thumbnails
                        .borrow()
                        .iter()
                        .all(|p| p.paintable().is_none())
                );
                t.back.emit_clicked();
                assert_eq!(t.pages.visible_child_name().as_deref(), Some("review"));
                t.status.set_text("Smoke verified: static/animated imports, editable roles, uninstalled trial, isolated install and conflict handling.");
                self.next();
            }
            8 if elapsed > Duration::from_millis(250) => {
                if !self.layout_ready(&t.window) {
                    return;
                }
                let row = t.rows.borrow()[0].1.parent().unwrap().parent().unwrap();
                assert!(
                    row.height() <= 80,
                    "ordinary import rows should stay compact"
                );
                assert!(
                    t.preview.image_scroll.height() >= t.window.height() / 2,
                    "preview should receive at least half the window height by default"
                );
                let viewport = t.preview.root.parent().expect("inspection viewport");
                let bounds = t
                    .preview
                    .picture
                    .compute_bounds(&viewport)
                    .expect("preview bounds");
                assert!(
                    bounds.y() >= 0. && bounds.y() + bounds.height() <= viewport.height() as f32,
                    "default import layout must keep the cursor image visible"
                );
                super::trial_smoke::capture_main(&t.window, "target/qa/import-window.png");
                t.load(f.clone());
                self.next();
            }
            9 if !t.is_busy() && t.preview.has_data() => {
                assert_eq!(t.rows.borrow().len(), 4);
                assert_eq!(t.assignments().len(), 3);
                t.load(f.join("roots.zip"));
                self.next();
            }
            10 if !t.is_busy() && elapsed > Duration::from_millis(250) => {
                if !self.layout_ready(&t.window) {
                    return;
                }
                assert_eq!(t.pages.visible_child_name().as_deref(), Some("review"));
                assert_eq!(
                    t.roots.active_id().as_deref(),
                    Some(""),
                    "show the entire package by default"
                );
                assert_eq!(t.rows.borrow().len(), 2);
                assert!(t.preview.has_data());
                assert_eq!(
                    t.thumbnails
                        .borrow()
                        .iter()
                        .filter(|p| p.paintable().is_some())
                        .count(),
                    2
                );
                assert!(!t.install.is_visible());
                assert!(
                    !t.next.is_sensitive(),
                    "duplicate roles still need resolution"
                );
                super::trial_smoke::capture_main(&t.window, "target/qa/import-all-files.png");
                let choice = t.rows.borrow()[1].1.clone();
                choice.set_active(Some(5));
                assert!(t.next.is_sensitive());
                assert!(t.try_button.is_sensitive());
                t.name.set_text("Package-Must-Be-Confirmed");
                t.start_install();
                assert!(!t.is_busy());
                assert!(!target.join("Package-Must-Be-Confirmed").exists());
                assert!(t.status.text().contains("confirm"));
                t.confirm.set_active(true);
                t.roots.set_active_id(Some("First"));
                assert!(!t.confirm.is_active());
                assert_eq!(t.rows.borrow().len(), 1);
                assert!(t.preview.title.text().contains("First"));
                t.roots.set_active_id(Some("Second"));
                assert_eq!(
                    t.assignments()[0].0,
                    4,
                    "filtering must preserve manual mappings"
                );
                assert!(t.preview.title.text().contains("Second"));
                t.roots.set_active_id(Some(""));
                assert_eq!(t.rows.borrow().len(), 2);
                assert_eq!(t.assignments()[1].0, 4);
                assert!(t.next.is_sensitive());
                t.roots.set_active_id(Some("First"));
                t.reset_roles.emit_clicked();
                t.roots.set_active_id(Some(""));
                assert_eq!(
                    t.assignments()[1].0,
                    0,
                    "Reset roles must clear edits in hidden folders too"
                );
                assert!(
                    !t.next.is_sensitive(),
                    "reset cannot silently resolve genuine package conflicts"
                );
                self.next();
            }
            11 => {
                t.load(f.join("busy.ani"));
                t.close();
                assert!(t.is_busy(), "closing waits for the bounded worker");
                self.next();
            }
            12 if !t.is_busy() => {
                assert!(!t.window.is_visible());
                assert!(!t.preview.has_data());
                assert!(t.trial.region_cursor(3).is_none());
                t.present();
                t.load(f.join("text.cur"));
                self.next();
            }
            13 if !t.is_busy() && t.preview.has_data() => {
                assert_eq!(t.assignments()[0].0, 2);
                t.load(
                    f.parent()
                        .expect("test directory")
                        .join("ani-compat/busy.ani"),
                );
                self.next();
            }
            14 if !t.is_busy() && t.preview.has_data() => {
                assert!(t.summary.text().contains("8-byte"));
                assert_eq!(
                    t.assignments()[0].1.decoded.as_ref().unwrap().variants[0].nominal,
                    160
                );
                t.try_button.emit_clicked();
                self.next();
            }
            15 if t.trial.region_cursor(3).is_some() && elapsed > Duration::from_millis(350) => {
                assert!(t.trial.frames_shown() > 2);
                let bounds = t
                    .preview
                    .picture
                    .compute_bounds(&t.preview.image_scroll)
                    .unwrap();
                assert!(
                    bounds.x() >= 0.
                        && bounds.y() >= 0.
                        && bounds.x() + bounds.width() <= t.preview.image_scroll.width() as f32
                        && bounds.y() + bounds.height() <= t.preview.image_scroll.height() as f32,
                    "160px ANI must fit completely at the import default zoom: {bounds:?}"
                );
                t.close();
                assert!(!t.window.is_visible());
                t.present();
                t.load(f.join("diagonals.zip"));
                self.next();
            }
            16 if !t.is_busy() && t.preview.has_data() => {
                if !self.layout_ready(&t.window) {
                    return;
                }
                assert_eq!(
                    t.assignments().iter().map(|(i, _)| *i).collect::<Vec<_>>(),
                    [9, 13]
                );
                assert!(!t.confirm.is_active());
                let first = t.rows.borrow()[0].1.clone();
                let second = t.rows.borrow()[1].1.clone();
                assert!(first.active_text().unwrap().contains("↖ ↘"));
                assert!(second.active_text().unwrap().contains("↗ ↙"));
                super::trial_smoke::capture_main(&t.window, "target/qa/import-diagonals.png");
                first.set_active(Some(0));
                let hint = first
                    .parent()
                    .unwrap()
                    .parent()
                    .unwrap()
                    .last_child()
                    .unwrap()
                    .downcast::<gtk::Label>()
                    .unwrap();
                assert!(hint.is_visible() && hint.text().contains("Skipped diagonal"));
                assert_eq!(t.assignments().len(), 1);
                first.set_active(Some(14));
                assert!(
                    !t.next.is_sensitive(),
                    "manual mapping can still create a duplicate"
                );
                t.reset_roles.emit_clicked();
                assert_eq!(
                    t.assignments()
                        .iter()
                        .map(|(role, _)| *role)
                        .collect::<Vec<_>>(),
                    [9, 13]
                );
                assert!(t.next.is_sensitive());
                assert!(!t.confirm.is_active());
                t.close();
                assert!(!t.window.is_visible());
                std::fs::remove_dir_all(target)
                    .expect("remove only unique QA installation directory");
                println!(
                    "IMPORT_SMOKE PASS: CUR/PNG/ANI (including size-hint/RIFF compatibility)/folder/ZIP preview, missing roles, scroll-safe role menus, Reset roles recovery, editable mapping including numbered diagonals and explicit corner directions, whole-package view, optional filtering, retained mappings, bounded thumbnails, confirmation reset, uninstalled texture animation, create-only QA installation, refreshed selection, conflict, read cancellation and close cleanup; settings unchanged"
                );
                self.next();
            }
            _ => (),
        }
    }
}
