//! Widget-local, read-only cursor playground. No settings port or controller.
pub mod current;
mod view;
use super::{
    label, texture,
    worker::{Job, Output, Worker},
};
use cursormochi_app::trial::TrialSet;
use cursormochi_core::{ThemeName, Variant};
use cursormochi_platform::{Environment, Repository, redact};
use gtk::{gdk, prelude::*};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::mpsc::Receiver,
    time::{Duration, Instant},
};

struct Role {
    frames: Vec<gdk::Cursor>,
    variant: Variant,
    thumbnail: Result<Variant, cursormochi_core::Error>,
    shown: usize,
}
struct Region {
    widget: gtk::Widget,
    role: usize,
    desired: Rc<RefCell<Option<gdk::Cursor>>>,
    recursive: bool,
}
impl Region {
    fn set_cursor(&self, cursor: Option<&gdk::Cursor>) {
        *self.desired.borrow_mut() = cursor.cloned();
        if self.recursive {
            assign_tree(&self.widget, cursor);
        } else {
            self.widget.set_cursor(cursor);
        }
    }
}
struct Live {
    roles: Vec<Option<Role>>,
    regions: Vec<Region>,
    elapsed: Duration,
    last: Instant,
    drag: Option<(gtk::GestureDrag, usize)>,
    frames_shown: usize,
}
#[derive(Clone)]
pub struct Trial {
    pub window: gtk::Window,
    pub root: gtk::Box,
    pub notes: gtk::TextView,
    pub reset: gtk::Button,
    pub snap: gtk::CheckButton,
    pub scale: gtk::Scale,
    pub backgrounds: gtk::ComboBoxText,
    pub source_details: gtk::Expander,
    pub icon: gtk::Image,
    icon_frame: Cell<usize>,
    pub active_label: gtk::Label,
    layout: gtk::Paned,
    pub popout: gtk::Button,
    pub issues: gtk::MenuButton,
    surface: gtk::Box,
    host: RefCell<Option<gtk::Box>>,
    title: gtk::Label,
    active: Rc<Cell<usize>>,
    pub size: gtk::SpinButton,
    pub sources: gtk::Label,
    pub entry: gtk::Entry,
    pub link: gtk::Button,
    pub board: gtk::Fixed,
    pub card: gtk::Box,
    pub move_gesture: gtk::GestureDrag,
    pub resize_gesture: gtk::GestureDrag,
    live: Rc<RefCell<Live>>,
    worker: Worker,
    rx: Rc<Receiver<(u64, Output)>>,
    request: Rc<Cell<u64>>,
    selected: Rc<RefCell<Option<ThemeName>>>,
    loaded: Rc<RefCell<Option<ThemeName>>>,
    epoch: Rc<Cell<u64>>,
    description: gtk::Label,
    imported: RefCell<Option<Vec<(usize, cursormochi_app::import::Asset)>>>,
}
/// Protect child widgets (notably GtkEntry's GtkText) from replacing our cursor.
fn bind_tree(widget: &gtk::Widget, desired: &Rc<RefCell<Option<gdk::Cursor>>>) {
    let d = desired.clone();
    widget.connect_cursor_notify(move |w| {
        let cursor = d.borrow().clone();
        if w.cursor() != cursor {
            w.set_cursor(cursor.as_ref());
        }
    });
    let mut child = widget.first_child();
    while let Some(w) = child {
        bind_tree(&w, desired);
        child = w.next_sibling();
    }
}
fn assign_tree(w: &gtk::Widget, cursor: Option<&gdk::Cursor>) {
    if w.cursor().as_ref() != cursor {
        w.set_cursor(cursor);
    }
    let mut child = w.first_child();
    while let Some(w) = child {
        assign_tree(&w, cursor);
        child = w.next_sibling();
    }
}
impl Trial {
    pub fn new(parent: &gtk::ApplicationWindow, repo: Repository) -> Rc<Self> {
        let window = gtk::Window::builder()
            .title("Try this cursor theme")
            .transient_for(parent)
            .destroy_with_parent(true)
            .modal(false)
            .default_width(740)
            .default_height(800)
            .build();
        let view::View {
            root,
            notes,
            reset,
            snap,
            scale,
            backgrounds,
            source_details,
            icon,
            active_label,
            layout,
            popout,
            issues,
            playground,
            title,
            active,
            size,
            sources,
            entry,
            link,
            board,
            card,
            move_gesture,
            resize_gesture,
            live,
            description,
        } = view::build(&window);
        let (worker, rx) = Worker::new(repo);
        let this = Rc::new(Self {
            window,
            root,
            notes,
            reset,
            snap,
            scale,
            backgrounds,
            source_details,
            icon,
            icon_frame: Cell::new(0),
            active_label,
            layout,
            popout,
            issues,
            surface: playground,
            host: RefCell::new(None),
            title,
            active,
            size,
            sources,
            entry,
            link,
            board,
            card,
            move_gesture,
            resize_gesture,
            live,
            worker,
            rx: Rc::new(rx),
            request: Rc::new(Cell::new(0)),
            selected: Rc::new(RefCell::new(None)),
            loaded: Rc::new(RefCell::new(None)),
            epoch: Rc::new(Cell::new(0)),
            description,
            imported: RefCell::new(None),
        });
        let t = Rc::downgrade(&this);
        this.popout.connect_clicked(move |_| {
            if let Some(t) = t.upgrade() {
                t.present();
            }
        });
        let t = Rc::downgrade(&this);
        this.reset.connect_clicked(move |_| {
            if let Some(t) = t.upgrade() {
                let gesture = t.live.borrow().drag.as_ref().map(|(g, _)| g.clone());
                if let Some(g) = gesture {
                    g.reset();
                }
                t.live.borrow_mut().drag = None;
            }
        });
        let t = Rc::downgrade(&this);
        this.size.connect_value_changed(move |_| {
            if let Some(t) = t.upgrade() {
                t.reload();
            }
        });
        let t = Rc::downgrade(&this);
        this.window.connect_close_request(move |_| {
            if let Some(t) = t.upgrade() {
                t.clear();
                t.window.set_visible(false);
                t.restore_inline();
            }
            glib::Propagation::Stop
        });
        this
    }
    pub fn clear_import(&self) {
        self.clear();
        *self.imported.borrow_mut() = None;
        *self.selected.borrow_mut() = None;
    }
    pub fn select_import(&self, assets: Vec<(usize, cursormochi_app::import::Asset)>) {
        *self.imported.borrow_mut() = Some(assets);
        *self.selected.borrow_mut() = ThemeName::new("Uninstalled-preview").ok();
        self.reload();
    }
    pub fn select(&self, theme: Option<ThemeName>, epoch: u64) {
        if *self.selected.borrow() == theme && self.epoch.get() == epoch {
            return;
        }
        *self.imported.borrow_mut() = None;
        *self.selected.borrow_mut() = theme;
        self.epoch.set(epoch);
        self.reload();
    }
    pub fn present(&self) {
        let opening = !self.window.is_visible();
        if opening {
            if let Some(host) = self.host.borrow().as_ref() {
                host.remove(&self.root);
            }
            self.window.set_child(Some(&self.root));
        }
        self.window.present();
        if opening {
            self.reload();
        }
    }
    pub fn embed(&self, host: &gtk::Box) {
        self.window.set_child(None::<&gtk::Widget>);
        host.append(&self.root);
        *self.host.borrow_mut() = Some(host.clone());
    }
    fn restore_inline(&self) {
        if let Some(host) = self.host.borrow().as_ref() {
            self.window.set_child(None::<&gtk::Widget>);
            host.append(&self.root);
        }
    }
    pub fn clear(&self) {
        self.window.set_cursor(None);
        self.root.set_cursor(None);
        self.surface.set_cursor(None);
        self.icon.set_paintable(None::<&gdk::Texture>);
        self.icon_frame.set(0);
        self.worker.cancel();
        self.request.set(0);
        *self.loaded.borrow_mut() = None;
        let gesture = self.live.borrow().drag.as_ref().map(|(g, _)| g.clone());
        if let Some(g) = gesture {
            g.reset();
        }
        let mut s = self.live.borrow_mut();
        s.drag = None;
        s.roles.clear();
        s.elapsed = Duration::ZERO;
        for r in &s.regions {
            r.set_cursor(None);
        }
    }
    fn reload(&self) {
        self.clear();
        self.issues.set_visible(false);
        self.sources.set_text("Open Playground to load this theme’s trial sources. The selected cursor’s details are shown above.");
        if !self.root.is_mapped() && !self.window.is_visible() {
            return;
        }
        if let Some(theme) = self.selected.borrow().clone() {
            self.title.set_text(theme.as_str());
            self.description
                .set_text("Loading cursors… Desktop settings unchanged.");
            self.sources
                .set_text("Loading verified sources… Ambient pointers are not trial evidence.");
            let size = self.size.value_as_int() as u32;
            let job = self.imported.borrow().as_ref().map_or_else(
                || Job::Trial(theme, size),
                |a| Job::ImportTrial(a.clone(), size),
            );
            self.request.set(self.worker.submit(job));
        } else {
            self.description
                .set_text("Select an available theme in the main window.");
            self.sources.set_text("No trial theme selected.");
        }
    }
    /// UI entry for a bounded set of decoded previews; future staging uses this too.
    fn set_data(&self, data: TrialSet) {
        let env = Environment::capture();
        let mut lines = Vec::new();
        let mut roles = Vec::new();
        let mut issues = Vec::new();
        let mut inherited = 0;
        let mut fallback = 0;
        for ((label, result), thumbnail) in data.roles.into_iter().zip(data.thumbnails) {
            match result {
                Ok(p) => {
                    match p.resolution {
                        cursormochi_core::Resolution::Inherited => inherited += 1,
                        cursormochi_core::Resolution::Fallback => fallback += 1,
                        cursormochi_core::Resolution::Direct => (),
                    }
                    if p.resolution != cursormochi_core::Resolution::Direct {
                        issues.push(format!(
                            "{label}: {:?} · {}",
                            p.resolution,
                            p.chain.join(" → ")
                        ));
                    }
                    let variant = p.variants[0].clone();
                    let frames = variant
                        .frames
                        .iter()
                        .map(|f| {
                            gdk::Cursor::from_texture(
                                &texture(f),
                                f.hotspot.0 as i32,
                                f.hotspot.1 as i32,
                                None,
                            )
                        })
                        .collect();
                    let f = &variant.frames[0];
                    lines.push(redact(&format!("{label}: {:?} · {} · {}×{} · hotspot {},{} · {} frame(s)\n{} · chain: {}",p.resolution,p.role,f.width,f.height,f.hotspot.0,f.hotspot.1,variant.frames.len(),p.source.display(),p.chain.join(" → ")),&env));
                    roles.push(Some(Role {
                        frames,
                        variant,
                        thumbnail,
                        shown: usize::MAX,
                    }));
                }
                Err(e) => {
                    issues.push(format!("{label}: {e}. Uses the ambient pointer."));
                    lines.push(format!("{label}: unavailable ({e}) · ambient pointer only"));
                    roles.push(None);
                }
            }
        }
        let missing = roles.iter().filter(|r| r.is_none()).count();
        let summary = [
            (missing, "unavailable"),
            (inherited, "inherited"),
            (fallback, "fallback"),
        ]
        .into_iter()
        .filter(|(n, _)| *n > 0)
        .map(|(n, kind)| format!("{n} {kind}"))
        .collect::<Vec<_>>()
        .join(" · ");
        self.issues.set_label(&summary);
        self.issues.set_visible(!summary.is_empty());
        self.description.set_text(&issues.join("\n"));
        for region in &self.live.borrow().regions {
            region
                .widget
                .set_tooltip_text(lines.get(region.role).map(String::as_str));
        }
        self.sources.set_text(&format!(
            "Trial nominal size {} · source details\n{}",
            data.size,
            lines.join("\n")
        ));
        *self.loaded.borrow_mut() = Some(data.theme);
        let mut s = self.live.borrow_mut();
        s.roles = roles;
        s.elapsed = Duration::ZERO;
        s.last = Instant::now();
    }
    pub fn tick(&self) {
        if self.root.is_mapped() && self.root.width() > 0 {
            let horizontal = self.root.width() >= 670;
            let desired = if horizontal {
                gtk::Orientation::Horizontal
            } else {
                gtk::Orientation::Vertical
            };
            if self.layout.orientation() != desired {
                self.layout.set_orientation(desired);
            }
            let position = if horizontal {
                self.layout.width() * 2 / 5
            } else {
                290
            };
            let minimum = self.layout.property::<i32>("min-position");
            let maximum = self.layout.property::<i32>("max-position").max(minimum);
            let position = position.clamp(minimum, maximum);
            if self.layout.position() != position {
                self.layout.set_position(position);
            }
        }
        if self.root.is_mapped() && self.request.get() == 0 && self.selected.borrow().is_some() {
            self.reload();
        }
        while let Ok((id, out)) = self.rx.try_recv() {
            if id != self.request.get() {
                continue;
            }
            if let Output::Trial(result) = out {
                match result {
                    Ok(data) if Some(&data.theme) == self.selected.borrow().as_ref() => {
                        self.set_data(data)
                    }
                    Err(e) => {
                        self.sources.set_text(&format!("Trial unavailable: {e}"));
                        self.issues.set_label("Trial unavailable");
                        self.description.set_text(&e.to_string());
                        self.issues.set_visible(true);
                    }
                    _ => (),
                }
            }
        }
        let now = Instant::now();
        let mut s = self.live.borrow_mut();
        let visible = self.root.is_mapped()
            && self
                .root
                .native()
                .and_then(|n| n.surface())
                .and_then(|v| v.downcast::<gdk::Toplevel>().ok())
                .is_none_or(|v| !v.state().contains(gdk::ToplevelState::MINIMIZED));
        if !visible {
            s.last = now;
            return;
        }
        let delta = now.duration_since(s.last);
        s.last = now;
        s.elapsed += delta;
        let elapsed = s.elapsed.as_millis() as u64;
        let mut changes = 0;
        for r in s.roles.iter_mut().flatten() {
            let frame = r.variant.frame_at(elapsed);
            if r.shown != frame {
                r.shown = frame;
                changes += 1;
            }
        }
        s.frames_shown += changes;
        let dragging = s.drag.as_ref().map(|(_, role)| *role);
        let held = dragging
            .and_then(|role| s.roles.get(role))
            .and_then(Option::as_ref)
            .map(|r| r.frames[r.shown].clone());
        let normal = s
            .roles
            .first()
            .and_then(Option::as_ref)
            .map(|r| &r.frames[r.shown]);
        self.surface.set_cursor(held.as_ref().or(normal));
        let role = dragging.unwrap_or_else(|| {
            s.regions
                .get(self.active.get())
                .map(|r| r.role)
                .unwrap_or(0)
        });
        let current = s.roles.get(role).and_then(Option::as_ref);
        let thumbnail = current.and_then(|r| r.thumbnail.as_ref().ok());
        let frame = thumbnail.map(|v| &v.frames[v.frame_at(elapsed)]);
        let key = frame.map_or(0, |f| f.rgba.as_ptr() as usize);
        if self.icon_frame.get() != key {
            self.icon_frame.set(key);
            self.icon
                .set_paintable(frame.and_then(super::thumbnail::paintable).as_ref());
        }
        self.icon
            .set_tooltip_text(Some(&match current.map(|r| &r.thumbnail) {
                Some(Ok(v)) => format!(
                    "Original source frame · {}×{} pixels",
                    v.frames[0].width, v.frames[0].height
                ),
                Some(Err(e)) => format!("Source thumbnail unavailable: {e}"),
                None => "No source thumbnail for this role".into(),
            }));
        let name = cursormochi_app::browser::ROLE_GROUPS[role].0;
        self.active_label.set_text(&if current.is_some() {
            name.into()
        } else {
            format!("{name} · unavailable")
        });
        for region in &s.regions {
            let cursor = s
                .roles
                .get(dragging.unwrap_or(region.role))
                .and_then(Option::as_ref)
                .map(|r| r.frames[r.shown].clone());
            if *region.desired.borrow() != cursor {
                region.set_cursor(cursor.as_ref());
            }
        }
    }
    pub fn loaded_theme(&self) -> Option<ThemeName> {
        self.loaded.borrow().clone()
    }
    pub fn region_cursor(&self, role: usize) -> Option<gdk::Cursor> {
        self.live
            .borrow()
            .regions
            .iter()
            .find(|r| r.role == role)
            .and_then(|r| r.widget.cursor())
    }
    pub fn dragging(&self) -> bool {
        self.live.borrow().drag.is_some()
    }
    pub fn frames_shown(&self) -> usize {
        self.live.borrow().frames_shown
    }
}
pub(super) fn position(board: &gtk::Fixed, card: &gtk::Box) -> (f64, f64) {
    let (x, y) = board
        .child_transform(card)
        .map(|t| t.to_translate())
        .unwrap_or((0., 0.));
    (x as f64, y as f64)
}
fn drag(
    w: &impl IsA<gtk::Widget>,
    board: &gtk::Fixed,
    card: &gtk::Box,
    live: &Rc<RefCell<Live>>,
    snap: &gtk::CheckButton,
    role: usize,
) -> gtk::GestureDrag {
    let gesture = gtk::GestureDrag::new();
    gesture.set_button(1);
    let start = Rc::new(Cell::new((0., 0.)));
    let (b, c, s, l) = (
        board.downgrade(),
        card.downgrade(),
        start.clone(),
        Rc::downgrade(live),
    );
    gesture.connect_drag_begin(move |g, _, _| {
        // A resize child owns its gesture; the card body must not also move.
        g.set_state(gtk::EventSequenceState::Claimed);
        let (Some(b), Some(c)) = (b.upgrade(), c.upgrade()) else {
            return;
        };
        s.set(if role == 7 {
            position(&b, &c)
        } else {
            (c.width() as f64, c.height() as f64)
        });
        if let Some(l) = l.upgrade() {
            l.borrow_mut().drag = Some((g.clone(), role));
        }
    });
    let (b, c, s) = (board.downgrade(), card.downgrade(), start.clone());
    let drag_snap = snap.clone();
    gesture.connect_drag_update(move |_, x, y| {
        let snap = |v: f64| {
            if drag_snap.is_active() {
                (v / 12.).round() * 12.
            } else {
                v
            }
        };
        let (Some(b), Some(c)) = (b.upgrade(), c.upgrade()) else {
            return;
        };
        let (a, d) = s.get();
        if role == 7 {
            b.move_(
                &c,
                snap(a + x).clamp(0., (b.width() - c.width()).max(0) as f64),
                snap(d + y).clamp(0., (b.height() - c.height()).max(0) as f64),
            );
        } else {
            let (cx, cy) = position(&b, &c);
            let width = if role == 5 || role == 9 {
                snap(a + x).clamp(120., (b.width() as f64 - cx).max(120.)) as i32
            } else {
                c.width()
            };
            let height = if role == 6 || role == 9 {
                snap(d + y).clamp(80., (b.height() as f64 - cy).max(80.)) as i32
            } else {
                c.height()
            };
            c.set_size_request(width, height);
        }
    });
    let l = Rc::downgrade(live);
    gesture.connect_drag_end(move |_, _, _| {
        if let Some(l) = l.upgrade() {
            l.borrow_mut().drag = None;
        }
    });
    let l = Rc::downgrade(live);
    gesture.connect_cancel(move |_, _| {
        if let Some(l) = l.upgrade() {
            l.borrow_mut().drag = None;
        }
    });
    let keys = gtk::EventControllerKey::new();
    w.set_can_focus(true);
    let (b, c) = (board.downgrade(), card.downgrade());
    let step_snap = snap.clone();
    keys.connect_key_pressed(move |_, key, _, _| {
        let (dx, dy) = match key {
            gdk::Key::Left => (-1., 0.),
            gdk::Key::Right => (1., 0.),
            gdk::Key::Up => (0., -1.),
            gdk::Key::Down => (0., 1.),
            _ => return glib::Propagation::Proceed,
        };
        if let (Some(b), Some(c)) = (b.upgrade(), c.upgrade()) {
            let step = if step_snap.is_active() { 12. } else { 4. };
            let (x, y) = position(&b, &c);
            if role == 7 {
                b.move_(
                    &c,
                    (x + dx * step).clamp(0., (b.width() - c.width()).max(0) as f64),
                    (y + dy * step).clamp(0., (b.height() - c.height()).max(0) as f64),
                );
            } else {
                let w = (c.width() as f64
                    + if role == 5 || role == 9 {
                        dx * step
                    } else {
                        0.
                    })
                .clamp(120., (b.width() as f64 - x).max(120.));
                let h = (c.height() as f64
                    + if role == 6 || role == 9 {
                        dy * step
                    } else {
                        0.
                    })
                .clamp(80., (b.height() as f64 - y).max(80.));
                c.set_size_request(w as i32, h as i32);
            }
        }
        glib::Propagation::Stop
    });
    w.add_controller(keys);
    w.add_controller(gesture.clone());
    gesture
}
