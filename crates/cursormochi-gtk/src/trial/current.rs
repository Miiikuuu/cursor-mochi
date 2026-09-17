//! System-named cursors, deliberately separate from selected-theme texture trials.
//! No settings port, controller, installation capability or texture substitution.
use super::*;
use cursormochi_app::current_cursor::ROLES;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Context {
    desktop: Option<String>,
    desktop_size: i32,
    gtk_theme: Option<String>,
    gtk_size: i32,
    scale: i32,
    epoch: u64,
}

pub struct CurrentTest {
    pub window: gtk::Window,
    entry: gtk::Entry,
    notes: gtk::TextView,
    divider: gtk::Paned,
    info: gtk::Label,
    notice: gtk::Label,
    summary: gtk::Label,
    evidence: Vec<gtk::Label>,
    results: Vec<gtk::ComboBoxText>,
    native_checks: Vec<gtk::CheckButton>,
    context: RefCell<Option<Context>>,
    worker: Worker,
    rx: Receiver<(u64, Output)>,
    request: Cell<u64>,
    pub stack: gtk::Stack,
    pub role_targets: Vec<gtk::Button>,
    fixture: bool,
}
impl CurrentTest {
    pub fn new(parent: &gtk::ApplicationWindow, repo: Repository, fixture: bool) -> Rc<Self> {
        let window = gtk::Window::builder()
            .title("Current cursor test")
            .transient_for(parent)
            .destroy_with_parent(true)
            .modal(false)
            .resizable(true)
            .default_width(940)
            .default_height(850)
            .build();
        let header = gtk::HeaderBar::new();
        header.add_css_class("main-header");
        window.set_titlebar(Some(&header));
        let root = gtk::Box::new(gtk::Orientation::Vertical, 10);
        root.add_css_class("current-test");
        let info = label("Reading current cursor settings…");
        root.append(&info);
        let notice = label(
            "Named cursors use the running desktop. Hover, click and drag to check the actual pointer.",
        );
        notice.add_css_class("dim-label");
        root.append(&notice);
        let stack = gtk::Stack::new();
        stack.set_vexpand(true);
        let switcher = gtk::StackSwitcher::builder()
            .stack(&stack)
            .halign(gtk::Align::Center)
            .build();
        header.set_title_widget(Some(&switcher));
        let page = gtk::Box::new(gtk::Orientation::Vertical, 16);
        page.add_css_class("playground-page");
        let title = label("Test the real window");
        title.add_css_class("theme-title");
        page.append(&title);
        page.append(&label("Drag the actual title bar to move this window. Resize its outer edges and all four corners. These actions use GTK and the window system directly; there are no simulated cards."));
        page.append(&label("Select and edit text below, then drag the divider between the panes. These are ordinary GTK controls with their own native cursor behavior."));
        let entry = gtk::Entry::builder()
            .text("Select or edit this text")
            .build();
        page.append(&entry);
        let notes = gtk::TextView::new();
        notes.set_wrap_mode(gtk::WrapMode::WordChar);
        notes
            .buffer()
            .set_text("A native text editor.\nSelect text, type, and move the pointer in and out.");
        let editor = gtk::ScrolledWindow::builder()
            .child(&notes)
            .min_content_height(160)
            .min_content_width(180)
            .build();
        let normal = gtk::Box::new(gtk::Orientation::Vertical, 12);
        normal.set_margin_start(20);
        normal.set_margin_end(20);
        normal.append(&label("Normal background"));
        normal.append(&label(
            "Move here to check that the ordinary pointer returns.",
        ));
        let divider = gtk::Paned::new(gtk::Orientation::Horizontal);
        divider.set_wide_handle(true);
        divider.set_start_child(Some(&editor));
        divider.set_end_child(Some(&normal));
        divider.set_position(380);
        divider.set_vexpand(true);
        page.append(&divider);
        page.append(&label("Use All roles for explicit system cursor requests such as Busy and Working. A role tile is not a real window resize operation."));
        stack.add_titled(&page, Some("native"), "Real window");

        let native = gtk::Box::new(gtk::Orientation::Vertical, 8);
        native.add_css_class("native-checks");
        native.append(&label("Resize this real window using all four edges and corners. Keep it unmaximized. Check each box only after observing the pointer and resizing successfully."));
        let checks = gtk::Grid::builder()
            .column_spacing(12)
            .row_spacing(8)
            .column_homogeneous(true)
            .build();
        let native_checks: Vec<_> = ROLES[12..20]
            .iter()
            .enumerate()
            .map(|(i, (title, _))| {
                let check = gtk::CheckButton::with_label(title);
                checks.attach(&check, (i % 4) as i32, (i / 4) as i32, 1, 1);
                check
            })
            .collect();
        native.append(&checks);
        root.append(&native);

        let roles = gtk::Box::new(gtk::Orientation::Vertical, 12);
        roles.set_margin_top(12);
        roles.append(&label("Hover each role to test its actual system cursor. Mark your observation below it. File evidence uses CursorMochi’s resolver; GTK/compositor aliases, built-in cursors and caches may differ."));
        let grid = gtk::Grid::builder()
            .column_spacing(12)
            .row_spacing(12)
            .column_homogeneous(true)
            .build();
        let mut evidence = Vec::new();
        let mut results = Vec::new();
        let mut role_targets = Vec::new();
        for (i, (title, name)) in ROLES.iter().enumerate() {
            let cell = gtk::Box::new(gtk::Orientation::Vertical, 6);
            cell.add_css_class("cursor-test-role");
            let target = gtk::Button::with_label(title);
            target.set_height_request(44);
            let cursor = gdk::Cursor::from_name(name, None);
            target.set_cursor(cursor.as_ref());
            target.set_tooltip_text(Some(&format!(
                "System cursor request: {name}. No external action."
            )));
            cell.append(&target);
            let source = label("Checking file…");
            source.add_css_class("dim-label");
            source.set_wrap(false);
            source.set_ellipsize(gtk::pango::EllipsizeMode::End);
            cell.append(&source);
            let result = gtk::ComboBoxText::new();
            for text in ["Not checked", "Looks correct", "Problem"] {
                result.append_text(text);
            }
            result.set_active(Some(0));
            result.set_tooltip_text(Some("Your visual observation; never set automatically"));
            cell.append(&result);
            grid.attach(&cell, (i % 3) as i32, (i / 3) as i32, 1, 1);
            evidence.push(source);
            results.push(result);
            role_targets.push(target);
        }
        roles.append(&grid);
        let scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&roles)
            .build();
        stack.add_titled(&scroll, Some("roles"), "All roles");
        root.append(&stack);
        let footer = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let summary = label("Visual checks: not started");
        summary.set_hexpand(true);
        footer.append(&summary);
        let refresh = gtk::Button::with_label("Recheck files");
        refresh.set_tooltip_text(Some("Re-read file evidence and reset observations. Does not reload the desktop’s cursor cache."));
        footer.append(&refresh);
        root.append(&footer);
        window.set_child(Some(&root));
        let (worker, rx) = Worker::new(repo);
        let this = Rc::new(Self {
            window,
            entry,
            notes,
            divider,
            info,
            notice,
            summary,
            evidence,
            results,
            native_checks,
            context: RefCell::new(None),
            worker,
            rx,
            request: Cell::new(0),
            stack,
            role_targets,
            fixture,
        });
        let weak = Rc::downgrade(&this);
        refresh.connect_clicked(move |_| {
            if let Some(t) = weak.upgrade() {
                t.invalidate();
            }
        });
        let weak = Rc::downgrade(&this);
        this.window.connect_close_request(move |w| {
            if let Some(t) = weak.upgrade() {
                t.clear();
            }
            w.set_visible(false);
            glib::Propagation::Stop
        });
        this
    }
    pub fn present(&self) {
        if !self.window.is_visible() {
            self.invalidate();
        }
        self.window.present();
    }
    fn invalidate(&self) {
        self.worker.cancel();
        self.request.set(0);
        self.context.replace(None);
        for r in &self.results {
            r.set_active(Some(0));
        }
        for r in &self.native_checks {
            r.set_active(false);
        }
        for r in &self.evidence {
            r.set_text("Checking file…");
            r.set_tooltip_text(None);
        }
    }
    pub fn clear(&self) {
        self.invalidate();
    }

    pub fn tick(&self, desktop: Option<&str>, desktop_size: i32, epoch: u64) {
        if !self.window.is_visible() {
            return;
        }
        let settings = self.window.settings();
        let context = Context {
            desktop: desktop.map(str::to_owned),
            desktop_size,
            gtk_theme: settings.gtk_cursor_theme_name().map(|n| n.to_string()),
            gtk_size: settings.gtk_cursor_theme_size(),
            scale: self.window.scale_factor(),
            epoch,
        };
        if self.context.borrow().as_ref() != Some(&context) {
            self.invalidate();
            self.info.set_text(&format!(
                "Desktop: {} · {} px     GTK: {} · {} px     Window scale: {}×{}",
                desktop.unwrap_or("Unavailable"),
                desktop_size,
                context.gtk_theme.as_deref().unwrap_or("Unreported"),
                context.gtk_size,
                context.scale,
                if self.fixture {
                    " · Fixture file evidence"
                } else {
                    ""
                }
            ));
            let mismatch =
                context.desktop != context.gtk_theme || context.desktop_size != context.gtk_size;
            self.notice.set_text(if self.fixture {
                "Fixture mode: file checks use samples; named pointers still come from the running desktop. Visual acceptance is pending."
            } else if mismatch {
                "Desktop and GTK settings differ. This window requests the running system’s cursors; check which theme you actually see. Visual acceptance is pending."
            } else {
                "Test the actual moving pointer, including animations. File checks do not prove desktop behavior. Visual acceptance is pending."
            });
            // Normal use audits GTK's reported theme, not the browser selection.
            let name = if self.fixture {
                context.desktop.as_deref()
            } else {
                context.gtk_theme.as_deref()
            };
            match name.and_then(|name| ThemeName::new(name).ok()) {
                Some(name) => self.request.set(self.worker.submit(Job::Audit(name))),
                None => {
                    for e in &self.evidence {
                        e.set_text("Theme unavailable");
                    }
                }
            }
            self.context.replace(Some(context));
        }
        while let Ok((id, output)) = self.rx.try_recv() {
            if id != self.request.get() {
                continue;
            }
            if let Output::Audit(result) = output {
                match result {
                    Ok(rows) => {
                        let inherited = rows.iter().filter(|r| matches!(r, Ok(e) if e.resolution == cursormochi_core::Resolution::Inherited)).count();
                        let fallback = rows.iter().filter(|r| matches!(r, Ok(e) if e.resolution == cursormochi_core::Resolution::Fallback)).count();
                        let unverified = rows.iter().filter(|r| r.is_err()).count();
                        self.notice.set_text(&format!("{} File checks: {inherited} inherited · {fallback} fallback · {unverified} unverified. See All roles for reasons.", self.notice.text()));
                        for (i, row) in rows.into_iter().enumerate() {
                            let name = ROLES[i].1;
                            match row {
                                Ok(e) => {
                                    self.evidence[i].set_text(&format!(
                                        "{name} · {:?} · {} frames",
                                        e.resolution, e.frames
                                    ));
                                    self.evidence[i].set_tooltip_text(Some(&format!("Exact-name file evidence\n{}\nChain: {}\nDoes not identify the compositor’s loaded cursor.", e.source.display(), e.chain.join(" → "))));
                                }
                                Err(e) => {
                                    self.evidence[i]
                                        .set_text(&format!("{name} · No verified file"));
                                    self.evidence[i].set_tooltip_text(Some(&format!("{e}\nThe system may use a legacy alias or built-in fallback. Check the actual pointer.")));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        for label in &self.evidence {
                            label.set_text(&format!("Check failed: {e}"));
                        }
                    }
                }
            }
        }
        let checked = self
            .results
            .iter()
            .filter(|r| r.active() == Some(1))
            .count();
        let problems = self
            .results
            .iter()
            .filter(|r| r.active() == Some(2))
            .count();
        let native = self.native_checks.iter().filter(|r| r.is_active()).count();
        self.summary.set_text(&format!("Your checks: {checked}/{} correct · {problems} problems · {native}/8 native edges & corners", ROLES.len()));
    }
    pub fn audit_ready(&self) -> bool {
        self.evidence.iter().all(|e| e.text() != "Checking file…")
    }
    pub fn smoke_check(&self) {
        assert!(!self.window.is_modal());
        assert!(self.window.is_resizable());
        assert!(
            self.window.cursor().is_none(),
            "native border cursor must remain unmanaged"
        );
        assert!(
            self.results.iter().all(|r| r.active() == Some(0)),
            "never auto-pass visual checks"
        );
        assert!(self.native_checks.iter().all(|r| !r.is_active()));
        for (button, (_, name)) in self.role_targets.iter().zip(ROLES) {
            assert_eq!(
                button.cursor().and_then(|c| c.name()).as_deref(),
                Some(*name)
            );
        }
        // Check the native control's actual request against import output names.
        // Window-edge aliases alone do not cover GTK pane dividers.
        for (orientation, role, expected) in [
            (gtk::Orientation::Horizontal, 5, "col-resize"),
            (gtk::Orientation::Vertical, 6, "row-resize"),
        ] {
            let native = gtk::Paned::new(orientation);
            let handle = native.first_child().expect("native pane handle");
            let name = handle
                .cursor()
                .and_then(|c| c.name())
                .expect("native pane cursor");
            assert_eq!(name.as_str(), expected);
            assert!(
                cursormochi_app::import::IMPORT_ROLES[role]
                    .1
                    .contains(&name.as_str()),
                "native pane request {name} must be included in confirmed resize exports"
            );
        }
        // Native widgets must remain free to update their own cursor. The old
        // texture-trial notification guard changed "default" back to "text".
        let child = self.entry.first_child().expect("GtkText child");
        let saved = child.cursor();
        child.set_cursor_from_name(Some("default"));
        assert_eq!(
            child.cursor().and_then(|c| c.name()).as_deref(),
            Some("default"),
            "system tests must not force the texture-trial cursor over native widget behavior"
        );
        child.set_cursor(saved.as_ref());
        assert!(self.entry.cursor().is_none());
        assert_eq!(
            self.notes.cursor().and_then(|c| c.name()),
            gtk::TextView::new().cursor().and_then(|c| c.name()),
            "editor must retain GTK's native cursor"
        );
        assert_eq!(
            self.divider.cursor().and_then(|c| c.name()),
            gtk::Paned::new(gtk::Orientation::Horizontal)
                .cursor()
                .and_then(|c| c.name())
        );
    }
}

/// Programmatic checks only. Does not move the physical pointer or auto-pass
/// the window's manual acceptance checklist.
#[derive(Default)]
pub struct Smoke {
    stage: u8,
    at: Option<Instant>,
}
impl Smoke {
    pub fn done(&self) -> bool {
        self.stage == 4
    }
    pub fn tick(&mut self, t: &CurrentTest, button: &gtk::Button) {
        let elapsed = self.at.map(|at| at.elapsed()).unwrap_or_default();
        match self.stage {
            0 => {
                button.emit_clicked();
                let count = gtk::Window::list_toplevels().len();
                button.emit_clicked();
                assert_eq!(gtk::Window::list_toplevels().len(), count);
                self.advance();
            }
            1 if t.audit_ready() && elapsed > Duration::from_millis(400) => {
                t.smoke_check();
                crate::trial_smoke::capture_window(
                    &t.window,
                    "target/qa/current-cursor-playground.png",
                );
                t.stack.set_visible_child_name("roles");
                self.advance();
            }
            2 if elapsed > Duration::from_millis(250) => {
                crate::trial_smoke::capture_window(&t.window, "target/qa/current-cursor-roles.png");
                t.results[0].set_active(Some(1));
                t.native_checks[0].set_active(true);
                let context = t.context.borrow().clone().expect("current context");
                t.tick(Some("Mochi-Motion"), 48, context.epoch + 1);
                t.smoke_check();
                let stale = t.request.get();
                t.tick(
                    context.desktop.as_deref(),
                    context.desktop_size,
                    context.epoch,
                );
                assert_ne!(t.request.get(), stale);
                t.window.close();
                assert!(!t.window.is_visible());
                assert!(t.context.borrow().is_none());
                t.present();
                t.stack.set_visible_child_name("native");
                self.advance();
            }
            3 if t.audit_ready() && elapsed > Duration::from_millis(200) => {
                t.smoke_check();
                let position = t.divider.position();
                t.divider.set_position(position + 10);
                assert_eq!(t.divider.position(), position + 10);
                t.divider.set_position(position);
                t.window.close();
                println!(
                    "CURRENT_CURSOR_SMOKE PASS: unmodified native entry/editor/divider, native col/row requests covered by import aliases, no texture-trial cursor guard, 34 named requests, eight independent native checks, no automatic visual pass, singleton/reopen, theme invalidation, close cleanup; no settings writes. Actual pointer, native resizing, animation and scaling remain manual."
                );
                self.advance();
            }
            _ => (),
        }
    }
    fn advance(&mut self) {
        self.stage += 1;
        self.at = Some(Instant::now());
    }
}
