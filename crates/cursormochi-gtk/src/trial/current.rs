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
    details: gtk::Label,
    file_summary: gtk::Label,
    vertical_divider: gtk::Paned,
    category: gtk::DropDown,
    role_groups: Vec<gtk::Box>,
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
            .title("System cursor test")
            .transient_for(parent)
            .destroy_with_parent(true)
            .modal(false)
            .resizable(true)
            .default_width(1040)
            .default_height(820)
            .build();
        let header = gtk::HeaderBar::new();
        header.add_css_class("main-header");
        window.set_titlebar(Some(&header));
        let root = gtk::Box::new(gtk::Orientation::Vertical, 12);
        root.add_css_class("current-test");
        let heading = gtk::Box::new(gtk::Orientation::Horizontal, 16);
        let title = label("Test your system cursor");
        title.add_css_class("theme-title");
        title.set_hexpand(true);
        heading.append(&title);
        let info = label("Reading settings…");
        info.add_css_class("system-theme-badge");
        info.set_wrap(false);
        info.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
        info.set_max_width_chars(32);
        heading.append(&info);
        root.append(&heading);
        let notice = label("Move the pointer through each area, then check what you see.");
        notice.add_css_class("system-notice");
        root.append(&notice);
        let details = label("");
        details.set_selectable(true);
        let file_summary = label("Checking files…");
        let diagnostics = gtk::Box::new(gtk::Orientation::Vertical, 6);
        diagnostics.append(&details);
        diagnostics.append(&file_summary);
        diagnostics.append(&label("File evidence cannot identify the compositor’s loaded image. Recheck resets your observations; it does not reload the system cursor cache."));
        let expander = gtk::Expander::builder()
            .label("Session & file details")
            .child(&diagnostics)
            .build();
        expander.add_css_class("system-details");
        root.append(&expander);
        let stack = gtk::Stack::new();
        stack.set_vexpand(true);
        // Each page scrolls independently at smaller window sizes.
        stack.set_vhomogeneous(false);
        let switcher = gtk::StackSwitcher::builder()
            .stack(&stack)
            .halign(gtk::Align::Center)
            .build();
        header.set_title_widget(Some(&switcher));

        let page = gtk::Box::new(gtk::Orientation::Vertical, 12);
        page.append(&section_heading(
            "01",
            "Text & dividers",
            "Type, select text, and drag the separators in both directions.",
        ));
        let entry = gtk::Entry::builder()
            .text("Try selecting or editing this title")
            .build();
        let notes = gtk::TextView::new();
        notes.set_wrap_mode(gtk::WrapMode::WordChar);
        notes.set_left_margin(16);
        notes.set_right_margin(16);
        notes.set_top_margin(12);
        notes.set_bottom_margin(12);
        notes.buffer().set_text("A small space to try your cursor.\n\nSelect a few words, type something, then move onto the background.\n\nDoes the text cursor switch back when you leave?");
        let editor = gtk::ScrolledWindow::builder()
            .child(&notes)
            .min_content_height(130)
            .vexpand(true)
            .build();
        let editor_card = gtk::Box::new(gtk::Orientation::Vertical, 10);
        editor_card.add_css_class("system-surface");
        editor_card.set_overflow(gtk::Overflow::Hidden);
        editor_card.append(&surface_heading("Text input", "Native entry & editor"));
        entry.set_margin_start(16);
        entry.set_margin_end(16);
        editor_card.append(&entry);
        editor_card.append(&editor);
        let normal = gtk::Box::new(gtk::Orientation::Vertical, 8);
        normal.add_css_class("system-background");
        normal.set_valign(gtk::Align::Fill);
        normal.append(&surface_heading(
            "Normal background",
            "Move here to restore the ordinary pointer",
        ));
        let icon = gtk::Image::from_icon_name("input-mouse-symbolic");
        icon.set_pixel_size(36);
        icon.set_vexpand(true);
        icon.add_css_class("system-area-icon");
        normal.append(&icon);
        let recovery = gtk::Box::new(gtk::Orientation::Vertical, 4);
        recovery.add_css_class("system-background");
        recovery.append(&surface_heading(
            "Try the horizontal divider ↑",
            "Drag up and down, then release",
        ));
        let vertical_divider = gtk::Paned::new(gtk::Orientation::Vertical);
        vertical_divider.set_wide_handle(true);
        vertical_divider.set_start_child(Some(&normal));
        vertical_divider.set_end_child(Some(&recovery));
        vertical_divider.set_shrink_start_child(false);
        vertical_divider.set_shrink_end_child(false);
        vertical_divider.set_position(164);
        vertical_divider.add_css_class("system-native-pane");
        let divider = gtk::Paned::new(gtk::Orientation::Horizontal);
        divider.set_wide_handle(true);
        divider.set_start_child(Some(&editor_card));
        divider.set_end_child(Some(&vertical_divider));
        divider.set_shrink_start_child(false);
        divider.set_shrink_end_child(false);
        divider.set_position(450);
        divider.set_height_request(270);
        divider.set_vexpand(true);
        divider.add_css_class("system-native-pane");
        page.append(&divider);
        let hint = label(
            "↔ Drag the vertical divider between the two panels to test horizontal resizing.",
        );
        hint.add_css_class("system-caption");
        page.append(&hint);

        page.append(&section_heading("02", "Window edges & corners", "Move this window by its title bar. Resize its actual outer border, then mark each direction below."));
        let checks = gtk::Grid::builder()
            .column_spacing(8)
            .row_spacing(8)
            .column_homogeneous(true)
            .build();
        checks.add_css_class("system-border-map");
        // Spatial checklist only; these controls do not simulate resize handles.
        let positions = [
            (0, 1),
            (2, 1),
            (1, 0),
            (1, 2),
            (0, 0),
            (2, 0),
            (0, 2),
            (2, 2),
        ];
        let arrows = ["←", "→", "↑", "↓", "↖", "↗", "↙", "↘"];
        let native_checks: Vec<_> = ROLES[12..20]
            .iter()
            .enumerate()
            .map(|(i, (title, _))| {
                let check = gtk::CheckButton::with_label(&format!("{}  {title}", arrows[i]));
                check.set_tooltip_text(Some(
                    "Mark only after testing the actual outer window border.",
                ));
                checks.attach(&check, positions[i].0, positions[i].1, 1, 1);
                check
            })
            .collect();
        let center = label("Resize the outer window\nKeep it unmaximized");
        center.set_xalign(0.5);
        center.set_justify(gtk::Justification::Center);
        center.add_css_class("system-border-center");
        checks.attach(&center, 1, 1, 1, 1);
        page.append(&checks);
        stack.add_titled(&scroll_page(&page), Some("native"), "Everyday use");

        let roles = gtk::Box::new(gtk::Orientation::Vertical, 16);
        let active = label("Hover a tile to try its system cursor. Record only what you see.");
        active.add_css_class("system-active-role");
        let role_page = gtk::Box::new(gtk::Orientation::Vertical, 12);
        let tools = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        active.set_hexpand(true);
        tools.append(&active);
        let category = gtk::DropDown::from_strings(&[
            "All categories",
            "Everyday pointers",
            "Move & drag",
            "Resize & split",
            "Precision & tools",
        ]);
        category.set_tooltip_text(Some("Show cursor roles by use"));
        tools.append(&category);
        role_page.append(&tools);
        let mut evidence = Vec::new();
        let mut results = Vec::new();
        let mut role_targets = Vec::new();
        let mut cells = Vec::new();
        for (title, name) in ROLES {
            let cell = gtk::Box::new(gtk::Orientation::Vertical, 8);
            cell.add_css_class("cursor-test-role");
            let target = gtk::Button::with_label(title);
            target.add_css_class("system-role-target");
            target.set_height_request(60);
            let cursor = gdk::Cursor::from_name(name, None);
            target.set_cursor(cursor.as_ref());
            target.set_tooltip_text(Some(&format!(
                "System request: {name}. Hover to inspect; no external action."
            )));
            let motion = gtk::EventControllerMotion::new();
            let status = active.clone();
            motion.connect_enter(move |_, _, _| {
                status.set_text(&format!("Testing: {title}  ·  {name}"))
            });
            let status = active.clone();
            motion.connect_leave(move |_| {
                status.set_text("Hover a tile to try its system cursor. Record only what you see.")
            });
            target.add_controller(motion);
            cell.append(&target);
            let source = label("Checking file…");
            source.add_css_class("system-caption");
            source.set_wrap(false);
            source.set_ellipsize(gtk::pango::EllipsizeMode::End);
            cell.append(&source);
            let result = gtk::ComboBoxText::new();
            for text in ["Not checked", "Looks correct", "Problem"] {
                result.append_text(text);
            }
            result.set_active(Some(0));
            result.set_tooltip_text(Some("Your visual observation; never set automatically"));
            // Scrolling the page must not accidentally accept a visual check.
            let controllers = result.observe_controllers();
            for i in 0..controllers.n_items() {
                if let Some(scroll) = controllers
                    .item(i)
                    .and_then(|c| c.downcast::<gtk::EventControllerScroll>().ok())
                {
                    scroll.set_propagation_phase(gtk::PropagationPhase::None);
                }
            }
            let weak = cell.downgrade();
            result.connect_changed(move |r| {
                if let Some(cell) = weak.upgrade() {
                    for (class, index) in [("observed-correct", 1), ("observed-problem", 2)] {
                        if r.active() == Some(index) {
                            cell.add_css_class(class);
                        } else {
                            cell.remove_css_class(class);
                        }
                    }
                }
            });
            cell.append(&result);
            cells.push(cell);
            evidence.push(source);
            results.push(result);
            role_targets.push(target);
        }
        let mut role_groups = Vec::new();
        for (title, subtitle, indices) in [
            (
                "Everyday pointers",
                "Normal, text, links and activity",
                &[0, 1, 2, 3, 4, 8, 20][..],
            ),
            (
                "Move & drag",
                "Explicit requests; these tiles do not move files",
                &[7, 21, 22, 23, 24, 25, 31],
            ),
            (
                "Resize & split",
                "Named requests; use Everyday use for real window borders",
                &[5, 6, 9, 11, 29, 30, 12, 13, 14, 15, 16, 17, 18, 19],
            ),
            (
                "Precision & tools",
                "Selection, menus and zoom",
                &[10, 26, 27, 28, 32, 33],
            ),
        ] {
            let group = gtk::Box::new(gtk::Orientation::Vertical, 12);
            group.append(&surface_heading(title, subtitle));
            let grid = gtk::Grid::builder()
                .column_spacing(12)
                .row_spacing(12)
                .column_homogeneous(true)
                .build();
            for (position, &index) in indices.iter().enumerate() {
                grid.attach(
                    &cells[index],
                    (position % 3) as i32,
                    (position / 3) as i32,
                    1,
                    1,
                );
            }
            group.append(&grid);
            roles.append(&group);
            role_groups.push(group);
        }
        let scroller = scroll_page(&roles);
        let groups = role_groups.clone();
        let adjustment = scroller.vadjustment();
        category.connect_selected_notify(move |choice| {
            for (index, group) in groups.iter().enumerate() {
                group.set_visible(choice.selected() == 0 || choice.selected() == index as u32 + 1);
            }
            adjustment.set_value(0.0);
        });
        role_page.append(&scroller);
        stack.add_titled(&role_page, Some("roles"), "All roles");
        root.append(&stack);
        let footer = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        footer.add_css_class("system-test-footer");
        let summary = label("Your observations · Not started");
        summary.set_hexpand(true);
        footer.append(&summary);
        let refresh = gtk::Button::with_label("Reset & recheck");
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
            details,
            file_summary,
            vertical_divider,
            category,
            role_groups,
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
        self.file_summary.set_text("Checking files…");
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
                "{} · {} px",
                context.gtk_theme.as_deref().unwrap_or("Theme unavailable"),
                context.gtk_size
            ));
            self.details.set_text(&format!(
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
            self.notice.remove_css_class("warning-text");
            if mismatch && !self.fixture {
                self.notice.add_css_class("warning-text");
            }
            self.notice.set_text(if self.fixture {
                "Sample file checks · The pointer still comes from your desktop."
            } else if mismatch {
                "Desktop and GTK settings differ — expand session details and check the theme you see."
            } else {
                "Move through each area and watch the actual pointer. Observations are yours to mark."
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
                    self.notice.set_text("The current theme is unavailable. Test native behavior; file evidence cannot be checked.");
                    self.file_summary
                        .set_text("Theme unavailable; file audit not run.");
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
                        self.file_summary.set_text(&format!("File evidence: {inherited} inherited · {fallback} fallback · {unverified} unverified. Per-role reasons are in All roles."));
                        if inherited + fallback + unverified > 0 {
                            self.notice.set_text(&format!("{} Some roles use inherited, fallback or unverified files; see All roles.", self.notice.text()));
                        }
                        for (i, row) in rows.into_iter().enumerate() {
                            let name = ROLES[i].1;
                            match row {
                                Ok(e) => {
                                    self.evidence[i].set_text(&format!(
                                        "{:?} · {} frames",
                                        e.resolution, e.frames
                                    ));
                                    self.evidence[i].set_tooltip_text(Some(&format!("Exact-name file evidence: {name}\n{}\nChain: {}\nDoes not identify the compositor’s loaded cursor.", e.source.display(), e.chain.join(" → "))));
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
                        self.notice.set_text(&format!("File check failed: {e}"));
                        self.notice.add_css_class("warning-text");
                        self.file_summary
                            .set_text("File check failed. No visual result inferred.");
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
        self.summary.set_text(&format!("Your observations · {checked}/{} roles correct · {problems} problems · {native}/8 window borders", ROLES.len()));
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
        assert_eq!(self.role_targets.len(), ROLES.len());
        for target in &self.role_targets {
            assert!(
                target.parent().and_then(|p| p.parent()).is_some(),
                "every role has a visible group"
            );
        }
        for result in &self.results {
            let controllers = result.observe_controllers();
            for i in 0..controllers.n_items() {
                if let Some(scroll) = controllers
                    .item(i)
                    .and_then(|c| c.downcast::<gtk::EventControllerScroll>().ok())
                {
                    assert_eq!(
                        scroll.propagation_phase(),
                        gtk::PropagationPhase::None,
                        "wheel browsing must not accept observations"
                    );
                }
            }
        }
        assert_eq!(
            self.vertical_divider.orientation(),
            gtk::Orientation::Vertical
        );
        assert!(self.vertical_divider.cursor().is_none());
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

fn scroll_page(child: &impl IsA<gtk::Widget>) -> gtk::ScrolledWindow {
    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(child)
        .vexpand(true)
        .build()
}

fn surface_heading(title: &str, subtitle: &str) -> gtk::Box {
    let heading = gtk::Box::new(gtk::Orientation::Vertical, 4);
    heading.add_css_class("system-surface-heading");
    let title = label(title);
    title.add_css_class("system-section-title");
    heading.append(&title);
    let subtitle = label(subtitle);
    subtitle.add_css_class("system-caption");
    heading.append(&subtitle);
    heading
}

fn section_heading(number: &str, title: &str, subtitle: &str) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let number = label(number);
    number.add_css_class("system-step");
    number.set_valign(gtk::Align::Start);
    row.append(&number);
    row.append(&surface_heading(title, subtitle));
    row
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
        self.stage == 6
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
                for selected in 1..=4 {
                    t.category.set_selected(selected);
                    for (index, group) in t.role_groups.iter().enumerate() {
                        assert_eq!(group.is_visible(), index as u32 + 1 == selected);
                    }
                }
                t.category.set_selected(3);
                self.advance();
            }
            3 if elapsed > Duration::from_millis(250) => {
                crate::trial_smoke::capture_window(
                    &t.window,
                    "target/qa/current-cursor-resize.png",
                );
                t.category.set_selected(0);
                assert!(t.role_groups.iter().all(|g| g.is_visible()));
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
            4 if t.audit_ready() && elapsed > Duration::from_millis(200) => {
                t.smoke_check();
                let position = t.divider.position();
                t.divider.set_position(position + 10);
                assert_eq!(t.divider.position(), position + 10);
                t.divider.set_position(position);
                let position = t.vertical_divider.position();
                t.vertical_divider.set_position(position + 10);
                assert_eq!(t.vertical_divider.position(), position + 10);
                t.vertical_divider.set_position(position);
                t.window.set_default_size(780, 640);
                self.advance();
            }
            5 if elapsed > Duration::from_millis(350) => {
                crate::trial_smoke::capture_window(
                    &t.window,
                    "target/qa/current-cursor-compact.png",
                );
                assert!(
                    t.window.width() <= 800,
                    "compact layout should fit without widening the window"
                );
                t.smoke_check();
                t.window.close();
                println!(
                    "CURRENT_CURSOR_SMOKE PASS: grouped roles, scroll-safe observations, compact layout, both native pane orientations; unmodified native entry/editor/divider, native col/row requests covered by import aliases, no texture-trial cursor guard, 34 named requests, eight independent native checks, no automatic visual pass, singleton/reopen, theme invalidation, close cleanup; no settings writes. Actual pointer, native resizing, animation and scaling remain manual."
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
