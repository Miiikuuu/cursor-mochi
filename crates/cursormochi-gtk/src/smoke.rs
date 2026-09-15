//! Read-only fixture interaction checks. Never constructs a host settings writer.
use super::{preview::PreviewPane, theme_list::ThemeList, ui::State};
use cursormochi_app::{DesktopSettingsPort, Snapshot};
use gtk::{gdk, prelude::*};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};
pub struct Smoke {
    enabled: bool,
    stage: u8,
    at: Instant,
    start: Instant,
    baseline: Snapshot,
    frames: usize,
    elapsed: Duration,
    inspection: Option<(gtk::Expander, gtk::ComboBoxText)>,
}
fn select(id: &str, state: &Rc<RefCell<State>>, list: &Rc<RefCell<ThemeList>>) {
    let index = state
        .borrow()
        .catalog
        .themes
        .iter()
        .position(|t| t.name.as_str() == id)
        .expect("fixture theme present");
    let row = list
        .borrow()
        .list
        .row_at_index(index as i32)
        .expect("fixture row");
    list.borrow().list.select_row(Some(&row));
}
impl Smoke {
    pub fn new(enabled: bool, baseline: &Snapshot) -> Self {
        Self {
            enabled,
            stage: 0,
            at: Instant::now(),
            start: Instant::now(),
            baseline: baseline.clone(),
            frames: 0,
            elapsed: Duration::ZERO,
            inspection: None,
        }
    }
    fn advance(&mut self) {
        self.stage += 1;
        self.at = Instant::now();
    }
    pub fn before_tick(
        &mut self,
        window: &gtk::ApplicationWindow,
        root: &gtk::Box,
        app: &gtk::Application,
    ) -> bool {
        if !self.enabled {
            return false;
        }
        assert!(
            self.start.elapsed() < Duration::from_secs(35),
            "GUI smoke timed out at stage {}",
            self.stage
        );
        if self.stage != 9 && self.stage != 10 {
            return false;
        }
        if self.at.elapsed() < Duration::from_millis(200) {
            return true;
        }
        let snapshot = gtk::Snapshot::new();
        if let Some(parent) = root.parent() {
            parent.snapshot_child(root, &snapshot);
        }
        let node = snapshot.to_node().expect("allocated window content");
        let opaque = gtk::Snapshot::new();
        let color = window
            .style_context()
            .lookup_color("theme_bg_color")
            .unwrap_or(gdk::RGBA::WHITE);
        opaque.append_color(&color, &node.bounds());
        opaque.append_node(node);
        let node = opaque.to_node().expect("opaque content");
        let texture = window
            .renderer()
            .expect("renderer")
            .render_texture(&node, None);
        std::fs::create_dir_all("target/qa").expect("QA output directory");
        texture
            .save_to_png(if self.stage == 9 {
                "target/qa/polish-smoke.png"
            } else {
                "target/qa/polish-smoke-large.png"
            })
            .expect("GUI screenshot");
        if self.stage == 9 {
            println!(
                "GUI_CAPTURE: compact window={}x{} scale={}",
                window.width(),
                window.height(),
                window.scale_factor()
            );
            window.set_default_size(1100, 840);
            if let Some((details, background)) = &self.inspection {
                details.set_expanded(true);
                background.set_active(Some(1));
            }
            self.advance();
            return true;
        }
        println!(
            "GUI_SMOKE PASS: backend={} scale={} window={}x{}; theme classification, independent selection, search/refresh, row targeting, thumbnails, static/animated preview, pause/hide, details, diagnostics and read-only settings",
            gdk::Display::default().expect("display").type_().name(),
            window.scale_factor(),
            window.width(),
            window.height()
        );
        app.quit();
        true
    }
    #[allow(clippy::too_many_arguments)]
    pub fn tick(
        &mut self,
        window: &gtk::ApplicationWindow,
        state: &Rc<RefCell<State>>,
        list: &Rc<RefCell<ThemeList>>,
        p: &PreviewPane,
        apply: &gtk::Button,
        undo: &gtk::Button,
        menu: &gtk::MenuButton,
        copy: &gtk::Button,
        opt: &gtk::CheckButton,
        size: &gtk::SpinButton,
        rescan: &Rc<dyn Fn()>,
        port: &Rc<dyn DesktopSettingsPort>,
    ) {
        if !self.enabled {
            return;
        }
        assert_eq!(
            port.read().expect("memory snapshot"),
            self.baseline,
            "GUI must not change settings"
        );
        assert!(!apply.is_sensitive());
        assert!(!undo.is_sensitive());
        assert!(apply.tooltip_text().is_some());
        match self.stage {
            0 if !state.borrow().scanning && p.has_data() => {
                assert_eq!(
                    state
                        .borrow()
                        .browser
                        .selected
                        .as_ref()
                        .expect("selected")
                        .as_str(),
                    "Mochi-Light"
                );
                self.inspection = Some((p.technical.clone(), p.background.clone()));
                println!(
                    "GUI_CAPTURE: initial window={}x{} scale={}",
                    window.width(),
                    window.height(),
                    window.scale_factor()
                );
                assert!(!p.play.is_visible());
                assert!(!p.hotspot.is_active());
                assert!(!p.technical.is_expanded());
                assert_eq!(apply.label().as_deref(), Some("Already in use"));
                assert!(
                    state
                        .borrow()
                        .catalog
                        .candidates
                        .iter()
                        .any(|t| t.name.as_str() == "Mochi-Broken")
                );
                let row = list.borrow().list.selected_row().expect("selected row");
                let hit = row
                    .pick(
                        (row.width() / 2) as f64,
                        (row.height() / 2) as f64,
                        gtk::PickFlags::DEFAULT,
                    )
                    .expect("pointer target");
                assert!(
                    hit.is::<gtk::ListBoxRow>(),
                    "row contents intercepted pointer"
                );
                assert!(list.borrow().search.grab_focus());
                assert!(
                    list.borrow()
                        .rows
                        .iter()
                        .any(|r| r.id == "Mochi-Light" && r.badge.text().contains("In use"))
                );
                self.frames = p.frames_shown();
                self.advance();
            }
            1 if self.at.elapsed() > Duration::from_millis(350) => {
                assert_eq!(
                    p.frames_shown(),
                    self.frames,
                    "static preview repainted unnecessarily"
                );
                p.technical.set_expanded(true);
                p.hotspot.set_active(true);
                assert!(p.technical.is_expanded());
                assert!(!p.source_summary().is_empty());
                menu.popup();
                assert!(copy.is_sensitive());
                assert!(menu.popover().is_some());
                menu.popdown();
                p.zoom.set_value(1.);
                assert!(!opt.is_active());
                assert_eq!(size.value_as_int(), 24);
                select("Mochi-Motion", state, list);
                self.advance();
            }
            2 if p.has_data() && p.frames_shown() > self.frames + 3 => {
                assert!(p.play.is_visible());
                assert_eq!(
                    state
                        .borrow()
                        .browser
                        .selected
                        .as_ref()
                        .expect("selected")
                        .as_str(),
                    "Mochi-Motion"
                );
                assert_ne!(apply.label().as_deref(), Some("Already in use"));
                list.borrow().search.set_text("Motion");
                p.play.set_active(false);
                self.elapsed = p.elapsed();
                self.advance();
            }
            3 if self.at.elapsed() > Duration::from_millis(350) => {
                assert_eq!(p.elapsed(), self.elapsed, "paused animation advanced");
                assert_eq!(
                    list.borrow()
                        .rows
                        .iter()
                        .filter(|r| r.row.is_visible())
                        .count(),
                    1
                );
                p.play.set_active(true);
                rescan();
                self.advance();
            }
            4 if !state.borrow().scanning && p.has_data() => {
                assert_eq!(state.borrow().browser.query, "Motion");
                assert_eq!(
                    state
                        .borrow()
                        .browser
                        .selected
                        .as_ref()
                        .expect("selected")
                        .as_str(),
                    "Mochi-Motion"
                );
                assert_eq!(
                    list.borrow()
                        .rows
                        .iter()
                        .filter(|r| r.row.is_visible())
                        .count(),
                    1
                );
                list.borrow().search.set_text("no-such-theme");
                self.advance();
            }
            5 if self.at.elapsed() > Duration::from_millis(350) => {
                assert!(list.borrow().empty.text().contains("No matching"));
                assert_eq!(
                    list.borrow()
                        .rows
                        .iter()
                        .filter(|r| r.row.is_visible())
                        .count(),
                    0
                );
                // SearchEntry notifications are debounced; update only the query here.
                list.borrow().search.set_text("");
                self.advance();
            }
            6 if self.at.elapsed() > Duration::from_millis(350) => {
                select("Mochi-Inherited", state, list);
                select("Mochi-Light", state, list);
                select("Mochi-Motion", state, list);
                p.zoom.set_value(3.);
                p.background.set_active(Some(2));
                p.technical.set_expanded(false);
                p.hotspot.set_active(false);
                window.set_default_size(820, 680);
                self.advance();
            }
            7 if p.has_data() && self.at.elapsed() > Duration::from_millis(400) => {
                assert_eq!(
                    state
                        .borrow()
                        .browser
                        .selected
                        .as_ref()
                        .expect("selected")
                        .as_str(),
                    "Mochi-Motion"
                );
                assert!(p.source_summary().contains("Mochi-Motion"));
                assert!(
                    list.borrow()
                        .rows
                        .iter()
                        .any(|r| r.image.paintable().is_some()),
                    "no visible thumbnail loaded"
                );
                assert!(
                    list.borrow()
                        .rows
                        .iter()
                        .filter(|r| r.image.paintable().is_some())
                        .count()
                        <= 16
                );
                assert!(p.canvas.is_mapped());
                self.elapsed = p.elapsed();
                window.set_visible(false);
                self.advance();
            }
            8 if self.at.elapsed() > Duration::from_millis(350) => {
                assert_eq!(p.elapsed(), self.elapsed, "hidden animation advanced");
                window.present();
                self.advance();
            }
            _ => (),
        }
    }
}
