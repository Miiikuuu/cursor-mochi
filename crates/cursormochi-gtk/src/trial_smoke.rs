//! Programmatic checks only: NOT visual acceptance of a moving hardware pointer.
use super::{smoke::select, theme_list::ThemeList, trial::Trial, ui::State};
use gtk::prelude::*;
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};
#[derive(Default)]
pub struct TrialSmoke {
    stage: u8,
    at: Option<Instant>,
    frames: usize,
    drag: super::trial_drag_smoke::DragSmoke,
}
impl TrialSmoke {
    pub fn done(&self) -> bool {
        self.stage == 13
    }
    fn advance(&mut self) {
        self.stage += 1;
        self.at = Some(Instant::now());
    }
    pub fn tick(
        &mut self,
        t: &Trial,
        state: &Rc<RefCell<State>>,
        list: &Rc<RefCell<ThemeList>>,
        button: &gtk::Button,
        views: &gtk::Stack,
        main: &gtk::ApplicationWindow,
    ) {
        let elapsed = self.at.map(|t| t.elapsed()).unwrap_or_default();
        match self.stage {
            0 => {
                select("Mochi-Light", state, list);
                button.emit_clicked();
                button.emit_clicked();
                assert!(!t.window.is_modal());
                self.advance();
            }
            1 if t
                .loaded_theme()
                .as_ref()
                .is_some_and(|t| t.as_str() == "Mochi-Light") =>
            {
                let cursor = t.region_cursor(0).expect("static texture cursor");
                assert!(cursor.name().is_none());
                assert!(cursor.texture().is_some());
                assert_eq!((cursor.hotspot_x(), cursor.hotspot_y()), (1, 2));
                assert!(t.sources.text().contains("unavailable"));
                let text = t.region_cursor(2).expect("fixture text cursor");
                let child = t.entry.first_child().expect("GtkEntry internal GtkText");
                assert_eq!(child.cursor().as_ref(), Some(&text));
                // Simulate GtkText trying to restore its own cursor property.
                child.set_cursor(None);
                assert_eq!(child.cursor().as_ref(), Some(&text));
                assert!(child.cursor().unwrap().name().is_none());
                assert!(t.region_cursor(1).is_some());
                assert!(t.region_cursor(4).is_some());
                assert!(t.region_cursor(3).is_none());

                self.frames = t.frames_shown();
                self.advance();
            }
            2 if elapsed > Duration::from_millis(350) => {
                if !self.drag.tick(t) {
                    return;
                }
                capture(t);
                assert_eq!(t.frames_shown(), self.frames, "static cursor re-rendered");
                t.link.emit_clicked();
                assert_eq!(t.link.label().as_deref(), Some("Link previewed"));
                t.entry.set_text("Only demo text");
                t.move_gesture
                    .emit_by_name::<()>("drag-begin", &[&0f64, &0f64]);
                t.move_gesture
                    .emit_by_name::<()>("drag-update", &[&35f64, &5f64]);
                t.move_gesture
                    .emit_by_name::<()>("drag-end", &[&35f64, &5f64]);
                assert!(!t.dragging());
                assert!(super::trial::position(&t.board, &t.card).0 > 12.);
                let before_width = t.card.width();
                t.resize_gesture
                    .emit_by_name::<()>("drag-begin", &[&0f64, &0f64]);
                t.resize_gesture
                    .emit_by_name::<()>("drag-update", &[&30f64, &0f64]);
                t.resize_gesture
                    .emit_by_name::<()>("drag-end", &[&30f64, &0f64]);
                assert!(t.card.width_request() > before_width);
                select("Mochi-Inherited", state, list);
                select("Mochi-Motion", state, list);
                self.advance();
            }
            3 if t
                .loaded_theme()
                .as_ref()
                .is_some_and(|t| t.as_str() == "Mochi-Motion") =>
            {
                assert!(t.sources.text().contains("4 frame(s)"));
                self.frames = t.frames_shown();
                self.advance();
            }
            4 if elapsed > Duration::from_millis(650) => {
                assert!(
                    t.frames_shown() > self.frames,
                    "animated cursor frames did not advance"
                );
                t.size.set_value(48.);
                self.advance();
            }
            5 if t.loaded_theme().is_some() => {
                let cursor = t.region_cursor(0).expect("resized cursor");
                assert_eq!((cursor.hotspot_x(), cursor.hotspot_y()), (2, 4));
                t.move_gesture
                    .emit_by_name::<()>("drag-begin", &[&0f64, &0f64]);
                t.size.set_value(32.); // cancel an in-flight replacement on close
                t.window.close();
                assert!(!t.window.is_visible());
                assert!(!t.dragging());
                assert!(t.region_cursor(0).is_none());
                assert!(t.loaded_theme().is_none());
                self.frames = t.frames_shown();
                self.advance();
            }
            6 if elapsed > Duration::from_millis(300) => {
                assert_eq!(self.frames, t.frames_shown());
                t.present();
                self.advance();
            }
            7 if t.loaded_theme().is_some() => {
                t.window.close();
                println!(
                    "TRIAL_SMOKE PASS: static before animated GDK texture cursors, hotspots, independent size, latest theme, demo actions, drag cleanup, close/reopen, zero settings-write calls and no Undo; moving-pointer visual acceptance NOT RUN"
                );
                self.advance();
            }
            8 => {
                views.set_visible_child_name("playground");
                select("Mochi-Light", state, list);
                t.size.set_value(24.);
                main.set_default_size(1120, 820);
                self.advance();
            }
            9 if elapsed > Duration::from_millis(450) && t.loaded_theme().is_some() => {
                assert!(!t.window.is_visible());
                assert!(t.root.is_mapped());
                assert!(!t.source_details.is_expanded());
                assert!(
                    !t.source_details.is_ancestor(&t.root),
                    "sources belong in Inspect"
                );
                assert!(t.issues.is_visible());
                assert_eq!(
                    t.issues.parent(),
                    t.icon.parent(),
                    "availability belongs at the right of the thumbnail heading"
                );
                assert_eq!(t.icon.pixel_size(), 64);
                assert!(!t.issues.label().unwrap().contains("0 inherited"));
                assert!(
                    t.popout
                        .parent()
                        .unwrap()
                        .has_css_class("playground-chrome")
                );
                assert!(list.borrow().refresh.is_ancestor(&list.borrow().root));
                let canvas_panel = t.scale.parent().unwrap().parent().unwrap();
                let note_panel = t.entry.parent().unwrap();
                assert!(
                    canvas_panel.width() > note_panel.width(),
                    "Canvas gets more room than Notes"
                );
                let ratio =
                    note_panel.width() as f64 / (note_panel.width() + canvas_panel.width()) as f64;
                assert!(
                    (0.35..0.45).contains(&ratio),
                    "Notes width fraction {ratio}"
                );
                assert!(t.snap.is_ancestor(&canvas_panel));

                t.notes.buffer().set_text("Edited note");
                t.snap.set_active(true);
                t.scale.set_value(125.);
                assert!(t.card.width_request() > 180);
                t.move_gesture
                    .emit_by_name::<()>("drag-begin", &[&0f64, &0f64]);
                t.move_gesture
                    .emit_by_name::<()>("drag-update", &[&13f64, &13f64]);
                t.move_gesture
                    .emit_by_name::<()>("drag-end", &[&13f64, &13f64]);
                let (x, y) = super::trial::position(&t.board, &t.card);
                assert_eq!(x % 12., 0.);
                assert_eq!(y % 12., 0.);
                let handle = t.move_gesture.widget().expect("move handle");
                let controls = handle.observe_controllers();
                for i in 0..controls.n_items() {
                    if let Some(k) = controls
                        .item(i)
                        .and_then(|c| c.downcast::<gtk::EventControllerKey>().ok())
                    {
                        assert!(k.emit_by_name::<bool>(
                            "key-pressed",
                            &[
                                &gtk::gdk::Key::Right,
                                &0u32,
                                &gtk::gdk::ModifierType::empty()
                            ]
                        ));
                    }
                }
                assert!(super::trial::position(&t.board, &t.card).0 > x);
                t.reset.emit_clicked();
                assert_eq!(t.entry.text(), "Hello, cursor.");
                assert!(!t.snap.is_active());
                assert_eq!(t.scale.value(), 100.);
                assert_eq!(
                    t.notes.buffer().text(
                        &t.notes.buffer().start_iter(),
                        &t.notes.buffer().end_iter(),
                        false
                    ),
                    "A little space for ideas.\nSelect, edit, and make it yours."
                );
                t.backgrounds.set_active_id(Some("dark"));
                assert_eq!(t.backgrounds.active_id().as_deref(), Some("dark"));
                t.backgrounds.set_active_id(Some("light"));
                let controllers = t.entry.observe_controllers();
                for i in 0..controllers.n_items() {
                    if let Some(c) = controllers
                        .item(i)
                        .and_then(|c| c.downcast::<gtk::EventControllerMotion>().ok())
                    {
                        c.emit_by_name::<()>("enter", &[&0f64, &0f64]);
                    }
                }
                t.tick();
                assert_eq!(t.active_label.text(), "Text");
                assert!(t.icon.paintable().is_some());
                self.advance();
            }
            10 if elapsed > Duration::from_millis(350) => {
                capture_main(main, "target/qa/frontend-playground.png");
                capture_window(main, "target/qa/frontend-header.png");
                t.backgrounds.set_active_id(Some("dark"));
                main.set_default_size(820, 680);
                self.advance();
            }
            11 if elapsed > Duration::from_millis(450) => {
                assert!(t.notes.is_mapped() && t.notes.height() >= 150);
                assert!(t.entry.parent().unwrap().width() > 400);
                capture_main(main, "target/qa/frontend-compact.png");
                t.backgrounds.set_active_id(Some("light"));
                views.set_visible_child_name("inspect");
                println!(
                    "FRONTEND_SMOKE PASS: embedded/detached shared playground, notes/reset, snap/scale controls, backgrounds, active texture badge, responsive layout and inspector access; no desktop writes"
                );
                self.advance();
            }
            12 if elapsed > Duration::from_millis(250) => {
                self.advance();
            }
            _ => (),
        }
    }
}

fn capture(t: &Trial) {
    let child = t.window.child().expect("trial content");
    let snapshot = gtk::Snapshot::new();
    t.window.snapshot_child(&child, &snapshot);
    let node = snapshot.to_node().expect("trial allocated");
    let opaque = gtk::Snapshot::new();
    let color = t
        .window
        .style_context()
        .lookup_color("theme_bg_color")
        .unwrap_or(gtk::gdk::RGBA::WHITE);
    opaque.append_color(&color, &node.bounds());
    opaque.append_node(&node);
    let node = opaque.to_node().expect("opaque trial snapshot");
    t.window
        .renderer()
        .expect("renderer")
        .render_texture(&node, None)
        .save_to_png("target/qa/trial-window.png")
        .expect("trial layout screenshot");
}

pub(super) fn capture_main(window: &gtk::ApplicationWindow, path: &str) {
    let child = window.child().expect("main content");
    let snapshot = gtk::Snapshot::new();
    window.snapshot_child(&child, &snapshot);
    let node = snapshot.to_node().expect("main allocated");
    let opaque = gtk::Snapshot::new();
    opaque.append_color(
        &window
            .style_context()
            .lookup_color("theme_bg_color")
            .unwrap_or(gtk::gdk::RGBA::WHITE),
        &node.bounds(),
    );
    opaque.append_node(&node);
    window
        .renderer()
        .expect("renderer")
        .render_texture(opaque.to_node().expect("opaque"), None)
        .save_to_png(path)
        .expect("frontend capture");
}

pub(crate) fn capture_window(window: &impl IsA<gtk::Window>, path: &str) {
    let window = window.as_ref();
    let paintable = gtk::WidgetPaintable::new(Some(window));
    let snapshot = gtk::Snapshot::new();
    paintable.snapshot(&snapshot, window.width() as f64, window.height() as f64);
    window
        .renderer()
        .expect("renderer")
        .render_texture(snapshot.to_node().expect("whole window allocated"), None)
        .save_to_png(path)
        .expect("header and frontend capture");
}
