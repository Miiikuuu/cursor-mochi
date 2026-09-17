//! One bounded import operation at a time. All file reads, decoding and writes run off-thread.
use super::{label, preview::PreviewPane, trial::Trial};
use cursormochi_app::{
    ThemeRepository,
    import::IMPORT_ROLES,
    import::{self as model, Asset, Package},
};
use cursormochi_core::{Error, ThemeName};
use cursormochi_platform::{Environment, Repository, import as files};
use gtk::prelude::*;
use std::{
    cell::{Cell, RefCell},
    path::PathBuf,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
    },
    time::Duration,
};
enum ResultMessage {
    Read(Result<Package, Error>),
    Installed(Result<PathBuf, Error>),
}
type Rows = Vec<(Asset, gtk::ComboBoxText)>;
pub struct Importer {
    pub window: gtk::ApplicationWindow,
    pub preview: PreviewPane,
    pub trial: Rc<Trial>,
    pub roots: gtk::ComboBoxText,
    pub source_status: gtk::Label,
    pub name: gtk::Entry,
    pub confirm: gtk::CheckButton,
    pub install: gtk::Button,
    pub try_button: gtk::Button,
    pub status: gtk::Label,
    pub summary: gtk::Label,
    pub pages: gtk::Stack,
    pub next: gtk::Button,
    pub back: gtk::Button,
    pub reset_roles: gtk::Button,
    pub requirements: gtk::Label,
    step_title: gtk::Label,
    pub review_summary: gtk::Label,
    pub install_overview: gtk::Label,
    source_name: gtk::Label,
    completed: Cell<bool>,
    targets: Vec<PathBuf>,
    repo: Repository,
    target: gtk::ComboBoxText,
    controls: gtk::Box,
    list: gtk::ListBox,
    list_scroll: gtk::ScrolledWindow,
    pub thumbnails: RefCell<Vec<gtk::Image>>,
    mappings: RefCell<std::collections::BTreeMap<PathBuf, u32>>,
    pub(super) rows: RefCell<Rows>,
    package: RefCell<Option<Package>>,
    receiver: RefCell<Option<Receiver<ResultMessage>>>,
    cancelled: RefCell<Arc<AtomicBool>>,
    busy: Cell<bool>,
    installing: Cell<bool>,
    close_pending: Cell<bool>,
    chooser: RefCell<Option<gtk::FileChooserNative>>,
    installed: Rc<dyn Fn(ThemeName)>,
}
impl Importer {
    pub fn new(
        parent: &gtk::ApplicationWindow,
        targets: Vec<PathBuf>,
        repo: Repository,
        installed: Rc<dyn Fn(ThemeName)>,
    ) -> Rc<Self> {
        let window = gtk::ApplicationWindow::builder()
            .application(&parent.application().unwrap_or_default())
            .title("Import cursor theme")
            .transient_for(parent)
            .destroy_with_parent(true)
            .default_width(1280)
            .default_height(900)
            .build();
        let root = gtk::Box::new(gtk::Orientation::Vertical, 8);
        root.add_css_class("import-wizard");
        window.set_child(Some(&root));
        let step_title = label("Import cursors");
        step_title.add_css_class("theme-title");
        root.append(&step_title);
        let controls = gtk::Box::new(gtk::Orientation::Vertical, 0);
        controls.set_vexpand(true);
        root.append(&controls);
        let pages = gtk::Stack::new();
        pages.set_vexpand(true);
        pages.set_hhomogeneous(false);
        pages.set_vhomogeneous(false);
        controls.append(&pages);

        let review = gtk::Box::new(gtk::Orientation::Vertical, 10);
        let open = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        let file = gtk::Button::with_label("Open CUR / ANI / ZIP…");
        file.add_css_class("primary-action");
        let folder = gtk::Button::with_label("Open folder…");
        open.append(&file);
        open.append(&folder);
        review.append(&open);
        let source_name = label("Choose a file or an extracted theme folder.");
        source_name.add_css_class("import-source-name");
        source_name.set_hexpand(true);
        source_name.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
        open.append(&source_name);
        let source_status = label("Your files stay unchanged. You can preview before installing.");

        let roots = gtk::ComboBoxText::new();
        let details_menu = gtk::MenuButton::builder().label("Details").build();
        let popover = gtk::Popover::new();
        let metadata = gtk::Box::new(gtk::Orientation::Vertical, 10);
        metadata.add_css_class("menu-content");
        metadata.set_width_request(380);
        metadata.append(&source_status);
        metadata.append(&label("Included files"));
        metadata.append(&roots);
        popover.set_child(Some(&metadata));
        details_menu.set_popover(Some(&popover));
        open.append(&details_menu);
        let review_summary = label("No files loaded.");
        review_summary.set_hexpand(true);
        let mapping_bar = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        mapping_bar.append(&review_summary);
        let reset_roles = gtk::Button::with_label("Reset roles");
        reset_roles.add_css_class("flat");
        reset_roles.set_tooltip_text(Some("Restore filename suggestions for every file, including hidden folders. Replaces your role edits; does not install anything."));
        mapping_bar.append(&reset_roles);
        review.append(&mapping_bar);
        let split = gtk::Paned::new(gtk::Orientation::Horizontal);
        split.set_vexpand(true);
        split.set_position(580);
        review.append(&split);
        let list = gtk::ListBox::new();
        list.add_css_class("import-file-list");
        list.set_selection_mode(gtk::SelectionMode::Single);
        let list_scroll = gtk::ScrolledWindow::builder()
            .child(&list)
            .min_content_width(470)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build();
        split.set_start_child(Some(&list_scroll));
        let preview = PreviewPane::new();
        preview.root.add_css_class("import-preview");
        preview.root.set_spacing(6);
        preview.subtitle.set_visible(false);
        preview.title.set_text("Import preview");
        preview.roles.set_visible(false);
        preview.configure_import();
        split.set_end_child(Some(
            &gtk::ScrolledWindow::builder()
                .child(&preview.root)
                .hscrollbar_policy(gtk::PolicyType::Never)
                .build(),
        ));
        let summary = label("No files loaded.");
        let details = gtk::Expander::builder()
            .label("Missing roles & conversion details")
            .child(
                &gtk::ScrolledWindow::builder()
                    .child(&summary)
                    .max_content_height(150)
                    .propagate_natural_height(true)
                    .hscrollbar_policy(gtk::PolicyType::Never)
                    .build(),
            )
            .build();
        metadata.append(&details);
        pages.add_named(&review, Some("review"));

        let install_page = gtk::Box::new(gtk::Orientation::Vertical, 16);
        install_page.add_css_class("import-install-page");
        install_page.append(&label("Add a new theme to your account. It will appear in Themes; applying it is a separate action."));
        let install_overview = label("");
        install_page.append(&install_overview);
        install_page.append(&label("Theme name"));
        let name = gtk::Entry::builder().placeholder_text("e.g. Miku").build();
        name.set_tooltip_text(Some(
            "1–80 English letters, digits, dash, underscore or dot; no leading dot",
        ));
        install_page.append(&name);
        install_page.append(&label("Install location"));
        let target = gtk::ComboBoxText::new();
        for p in &targets {
            target.append_text(&p.display().to_string());
        }
        target.set_active((!targets.is_empty()).then_some(0));
        install_page.append(&target);
        let final_summary = label("");
        summary
            .bind_property("label", &final_summary, "label")
            .sync_create()
            .build();
        install_page.append(
            &gtk::Expander::builder()
                .label("Review missing roles & conversion details")
                .child(
                    &gtk::ScrolledWindow::builder()
                        .child(&final_summary)
                        .max_content_height(180)
                        .propagate_natural_height(true)
                        .hscrollbar_policy(gtk::PolicyType::Never)
                        .build(),
                )
                .build(),
        );
        let confirm = gtk::CheckButton::with_label(
            "I reviewed the included files, roles, conversion details and install location.",
        );
        if let Some(l) = confirm
            .last_child()
            .and_then(|c| c.downcast::<gtk::Label>().ok())
        {
            l.set_wrap(true);
        }
        install_page.append(&confirm);
        pages.add_named(
            &gtk::ScrolledWindow::builder()
                .child(&install_page)
                .hscrollbar_policy(gtk::PolicyType::Never)
                .build(),
            Some("install"),
        );

        let status = label("");
        status.set_visible(false);
        status.add_css_class("import-operation");
        root.append(&status);
        status.connect_label_notify(|l| l.set_visible(!l.text().is_empty()));
        let requirements = label("Choose a file or folder to begin.");
        requirements.add_css_class("import-next-step");
        root.append(&requirements);
        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        let back = gtk::Button::with_label("Back");
        let cancel = gtk::Button::with_label("Close");
        let try_button = gtk::Button::with_label("Try this theme");
        let next = gtk::Button::with_label("Continue to install");
        next.add_css_class("primary-action");
        let install = gtk::Button::with_label("Install theme");
        install.add_css_class("primary-action");
        actions.append(&back);
        actions.append(&cancel);
        let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        actions.append(&spacer);
        actions.append(&try_button);
        actions.append(&next);
        actions.append(&install);
        root.append(&actions);
        let trial = Trial::new(&window, repo.clone());
        if let Some(details) = preview
            .technical
            .child()
            .and_then(|c| c.downcast::<gtk::Box>().ok())
        {
            details.append(&trial.source_details);
        }
        let this = Rc::new(Self {
            window,
            preview,
            trial,
            roots,
            source_status,
            name,
            confirm,
            install,
            try_button,
            status,
            summary,
            pages,
            next,
            back,
            reset_roles,
            requirements,
            step_title,
            review_summary,
            install_overview,
            source_name,
            completed: Cell::new(false),
            targets,
            repo,
            target,
            controls,
            list,
            list_scroll,
            thumbnails: RefCell::new(Vec::new()),
            mappings: RefCell::new(std::collections::BTreeMap::new()),
            rows: RefCell::new(Vec::new()),
            package: RefCell::new(None),
            receiver: RefCell::new(None),
            cancelled: RefCell::new(Arc::new(AtomicBool::new(false))),
            busy: Cell::new(false),
            installing: Cell::new(false),
            close_pending: Cell::new(false),
            chooser: RefCell::new(None),
            installed,
        });
        let weak = Rc::downgrade(&this);
        this.reset_roles.connect_clicked(move |_| {
            if let Some(t) = weak.upgrade()
                && !t.is_busy()
            {
                // Clear both visible edits and cached edits from filtered folders.
                // rebuild() otherwise preserves current rows before rebuilding.
                t.mappings.borrow_mut().clear();
                t.rows.borrow_mut().clear();
                t.rebuild();
            }
        });
        let weak = Rc::downgrade(&this);
        this.next.connect_clicked(move |_| {
            if let Some(t) = weak.upgrade() {
                if t.completed.get() {
                    t.close();
                } else if model::plan("Preview", &t.assignments(), true).is_ok() {
                    t.show_page("install");
                }
            }
        });
        let weak = Rc::downgrade(&this);
        this.back.connect_clicked(move |_| {
            if let Some(t) = weak.upgrade() {
                t.show_page("review");
            }
        });
        for (button, action) in [
            (file, gtk::FileChooserAction::Open),
            (folder, gtk::FileChooserAction::SelectFolder),
        ] {
            let weak = Rc::downgrade(&this);
            button.connect_clicked(move |_| {
                if let Some(t) = weak.upgrade() {
                    t.choose(action);
                }
            });
        }
        let weak = Rc::downgrade(&this);
        this.roots.connect_changed(move |_| {
            if let Some(t) = weak.upgrade() {
                t.rebuild();
                if t.roots.active_id().is_some() {
                    if t.name.text().is_empty() {
                        let root = t.roots.active_id().unwrap_or_default();
                        let source = t.source_name.text();
                        t.name.set_text(&suggested_name(if root.is_empty() {
                            &source
                        } else {
                            &root
                        }));
                    }
                    t.show_page("review");
                }
            }
        });
        let weak = Rc::downgrade(&this);
        this.list.connect_row_selected(move |_, row| {
            if let Some(t) = weak.upgrade() {
                t.preview_row(row.map(|r| r.index() as usize));
            }
        });
        let weak = Rc::downgrade(&this);
        this.name.connect_changed(move |_| {
            if let Some(t) = weak.upgrade() {
                t.changed();
            }
        });
        let weak = Rc::downgrade(&this);
        this.target.connect_changed(move |_| {
            if let Some(t) = weak.upgrade() {
                t.changed();
            }
        });
        let weak = Rc::downgrade(&this);
        this.confirm.connect_toggled(move |_| {
            if let Some(t) = weak.upgrade() {
                t.buttons();
            }
        });
        let weak = Rc::downgrade(&this);
        this.install.connect_clicked(move |_| {
            if let Some(t) = weak.upgrade() {
                t.start_install();
            }
        });
        let weak = Rc::downgrade(&this);
        this.try_button.connect_clicked(move |_| {
            if let Some(t) = weak.upgrade() {
                t.trial.select_import(t.assignments());
                t.trial.present();
            }
        });
        let weak = Rc::downgrade(&this);
        cancel.connect_clicked(move |_| {
            if let Some(t) = weak.upgrade() {
                t.close();
            }
        });
        let weak = Rc::downgrade(&this);
        this.window.connect_close_request(move |_| {
            if let Some(t) = weak.upgrade() {
                t.close();
            }
            glib::Propagation::Stop
        });
        let weak = Rc::downgrade(&this);
        glib::timeout_add_local(Duration::from_millis(16), move || {
            let Some(t) = weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            t.tick();
            glib::ControlFlow::Continue
        });
        this.show_page("review");
        this
    }
    fn show_page(&self, page: &str) {
        self.pages.set_visible_child_name(page);
        self.buttons();
    }
    pub fn present(&self) {
        self.window.present();
    }
    fn choose(self: &Rc<Self>, action: gtk::FileChooserAction) {
        if self.chooser.borrow().is_some() {
            return;
        }
        let chooser = gtk::FileChooserNative::new(
            Some("Choose cursor source"),
            Some(&self.window),
            action,
            Some("Open"),
            Some("Cancel"),
        );
        if action == gtk::FileChooserAction::Open {
            let filter = gtk::FileFilter::new();
            filter.set_name(Some("CUR, ANI or ZIP"));
            for p in ["*.cur", "*.CUR", "*.ani", "*.ANI", "*.zip", "*.ZIP"] {
                filter.add_pattern(p);
            }
            chooser.add_filter(&filter);
        }
        let weak = Rc::downgrade(self);
        chooser.connect_response(move |c, r| {
            if let Some(t) = weak.upgrade() {
                let path = c.file().and_then(|f| f.path());
                t.chooser.borrow_mut().take();
                c.destroy();
                if r == gtk::ResponseType::Accept {
                    if let Some(p) = path {
                        t.load(p);
                    } else {
                        t.status.set_text("Choose a local file or folder.");
                    }
                }
            }
        });
        *self.chooser.borrow_mut() = Some(chooser.clone());
        chooser.show();
    }
    pub fn load(self: &Rc<Self>, path: PathBuf) {
        if self.busy.get() {
            return;
        }
        self.completed.set(false);
        self.name.set_text("");
        self.source_name
            .set_text(&path.file_name().unwrap_or_default().to_string_lossy());
        self.show_page("review");
        self.mappings.borrow_mut().clear();
        self.rows.borrow_mut().clear();
        self.package.borrow_mut().take();
        self.roots.remove_all();
        self.rebuild();
        self.preview.clear();
        self.trial.clear_import();
        self.trial.window.set_visible(false);
        let (tx, rx) = mpsc::sync_channel(1);
        let cancel = self.begin(rx, false);
        self.status.set_text("Reading and decoding cursor sources…");
        self.source_status
            .set_text("Reading and decoding cursor sources…");
        std::thread::spawn(move || {
            let result = files::load(&path, &|| cancel.load(Ordering::Relaxed));
            let _ = tx.send(ResultMessage::Read(result));
        });
    }
    fn begin(&self, rx: Receiver<ResultMessage>, installing: bool) -> Arc<AtomicBool> {
        let cancel = Arc::new(AtomicBool::new(false));
        *self.cancelled.borrow_mut() = cancel.clone();
        *self.receiver.borrow_mut() = Some(rx);
        self.busy.set(true);
        self.installing.set(installing);
        self.controls.set_sensitive(false);
        self.buttons();
        cancel
    }
    pub fn is_busy(&self) -> bool {
        self.busy.get()
    }
    pub fn assignments(&self) -> Vec<(usize, Asset)> {
        self.rows
            .borrow()
            .iter()
            .filter_map(|(a, c)| {
                c.active()
                    .filter(|i| *i > 0)
                    .map(|i| (i as usize - 1, a.clone()))
            })
            .collect()
    }
    fn rebuild(self: &Rc<Self>) {
        if self.package.borrow().is_some() {
            for (asset, choice) in self.rows.borrow().iter() {
                if let Some(role) = choice.active() {
                    self.mappings.borrow_mut().insert(asset.path.clone(), role);
                }
            }
        }
        self.rows.borrow_mut().clear();
        self.thumbnails.borrow_mut().clear();
        while let Some(c) = self.list.first_child() {
            self.list.remove(&c);
        }
        let root = self.roots.active_id().map(|s| PathBuf::from(s.as_str()));
        let assets = self
            .package
            .borrow()
            .as_ref()
            .map(|p| p.assets.clone())
            .unwrap_or_default();
        for asset in assets.into_iter().filter(|a| {
            root.as_ref()
                .is_none_or(|r| r.as_os_str().is_empty() || a.path.starts_with(r))
        }) {
            let row = gtk::Box::new(gtk::Orientation::Vertical, 4);

            row.add_css_class("import-file-row");
            let title = label(&asset.path.file_name().unwrap_or_default().to_string_lossy());
            title.set_tooltip_text(Some(&asset.path.display().to_string()));
            title.add_css_class("theme-name");
            title.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
            let heading = gtk::Box::new(gtk::Orientation::Horizontal, 12);
            let picture = gtk::Image::new();
            picture.set_pixel_size(40);
            picture.set_tooltip_text(Some(
                "Actual first frame. Select the file to inspect animation, size and hotspot.",
            ));
            heading.append(&picture);
            self.thumbnails.borrow_mut().push(picture);
            let names = gtk::Box::new(gtk::Orientation::Vertical, 4);
            names.set_hexpand(true);
            names.append(&title);

            if let Ok(data) = &asset.decoded {
                let frames = data.variants.first().map_or(0, |v| v.frames.len());
                let info = label(&format!(
                    "{frames} frame{}",
                    if frames == 1 { "" } else { "s" }
                ));
                info.add_css_class("dim-label");
                names.append(&info);
            }
            heading.append(&names);
            row.append(&heading);
            if let Err(e) = &asset.decoded {
                row.append(&label(&format!("Cannot decode: {e}")));
            }
            let choices = gtk::ComboBoxText::new();
            // GtkComboBox's built-in scroll controller changes its active item
            // even while closed. Let wheel events bubble to the file scroller;
            // click/keyboard selection and scrolling an open popup still work.
            let controllers = choices.observe_controllers();
            for i in 0..controllers.n_items() {
                if let Some(scroll) = controllers
                    .item(i)
                    .and_then(|c| c.downcast::<gtk::EventControllerScroll>().ok())
                {
                    scroll.set_propagation_phase(gtk::PropagationPhase::None);
                }
            }
            choices.append_text("Skip this file");
            for (index, (label, _)) in IMPORT_ROLES.iter().enumerate() {
                choices.append_text(match index {
                    9 => "Resize diagonal ↖ ↘",
                    13 => "Resize diagonal ↗ ↙",
                    _ => label,
                });
            }
            let suggested = model::suggestion(&asset.path);
            choices.set_active(Some(if asset.decoded.is_ok() {
                self.mappings
                    .borrow()
                    .get(&asset.path)
                    .copied()
                    .unwrap_or_else(|| suggested.map_or(0, |i| i as u32 + 1))
            } else {
                0
            }));
            choices.set_sensitive(root.is_some() && asset.decoded.is_ok());
            choices.set_tooltip_text(Some(
                "Filename suggestion only; review and confirm before installing",
            ));
            heading.append(&choices);
            choices.set_valign(gtk::Align::Center);
            let hint = label("");
            hint.add_css_class("dim-label");
            hint.set_visible(false);
            row.append(&hint);
            self.list.append(&row);
            self.rows.borrow_mut().push((asset, choices));
        }
        let count = self.rows.borrow().len();
        self.source_status.set_text(&if self.package.borrow().is_none() {
            "Open a cursor file, ZIP or folder to begin.".into()
        } else if count == 0 {
            "No CUR or ANI files found in this source.".into()
        } else if root.is_none() {
            format!("{count} cursor files loaded.")
        } else {
            format!("{count} cursor files shown. All folders are included unless you choose a filter.")
        });
        self.connect_rows();
        self.changed();
        if let Some(r) = self.list.row_at_index(0) {
            self.list.select_row(Some(&r));
        }
    }
    fn connect_rows(self: &Rc<Self>) {
        for (_, c) in self.rows.borrow().iter() {
            let weak = Rc::downgrade(self);
            c.connect_changed(move |_| {
                if let Some(t) = weak.upgrade() {
                    t.changed();
                }
            });
        }
    }
    fn changed(&self) {
        self.confirm.set_active(false);
        let assigned = self.assignments();
        let mut counts = [0usize; IMPORT_ROLES.len()];
        for (role, _) in &assigned {
            if let Some(count) = counts.get_mut(*role) {
                *count += 1;
            }
        }
        for (asset, choice) in self.rows.borrow().iter() {
            if let Some(hint) = choice
                .parent()
                .and_then(|p| p.parent())
                .and_then(|p| p.last_child())
                .and_then(|w| w.downcast::<gtk::Label>().ok())
            {
                let role = choice.active().filter(|i| *i > 0).map(|i| i as usize - 1);
                let message = if let Some(role) = role.filter(|i| counts[*i] > 1) {
                    format!(
                        "{} is assigned more than once. Keep one file for this role; remap or skip the others.",
                        IMPORT_ROLES[role].0
                    )
                } else if role.is_none()
                    && asset.decoded.is_ok()
                    && matches!(model::suggestion(&asset.path), Some(9 | 13))
                {
                    "Skipped diagonal: its window corners will use desktop fallback.".into()
                } else {
                    String::new()
                };
                hint.set_text(&message);
                hint.set_visible(!message.is_empty());
            }
        }
        let plan = model::plan("Preview", &assigned, true);
        let ignored = self.package.borrow().as_ref().map_or(0, |p| p.ignored);
        let skipped = self.rows.borrow().len().saturating_sub(assigned.len());
        let text = match &plan {
            Ok(p) => format!(
                "{} mapped · {skipped} skipped · {ignored} non-cursor files ignored\nMissing Windows roles: {}\n{}\nNo explicit parent theme. Desktop fallback may differ; inspect actual sources after installation.\n{}",
                p.mapped.len(),
                if p.missing.is_empty() {
                    "none".into()
                } else {
                    p.missing.join(", ")
                },
                p.system_coverage().details(),
                p.notes.join("\n")
            ),
            Err(e) => format!(
                "Mapping needs attention: {e}. Skipped: {skipped}; non-cursor files ignored: {ignored}."
            ),
        };
        self.summary.set_text(&text);
        self.install_overview.set_text(&match &plan {
            Ok(p) => format!("{} cursors selected · {skipped} files skipped.\nMissing Windows roles: {}.\n{}\nReview missing system names in Details below. No parent theme is included.", p.mapped.len(), if p.missing.is_empty() { "none".into() } else { p.missing.join(", ") }, p.system_coverage().summary()),
            Err(e) => format!("Return to Review to resolve: {e}"),
        });
        self.review_summary
            .set_text(&match model::plan("Preview", &assigned, true) {
                Ok(p) => format!(
                    "{} mapped · {skipped} skipped\n{}",
                    p.mapped.len(),
                    p.system_coverage().summary()
                ),
                Err(e) => format!(
                    "Review needed: {e}. Use each file’s role menu to fix conflicts or skip it."
                ),
            });
        self.completed.set(false);
        self.status.set_text("");
        self.buttons();
        self.trial.select_import(assigned);
    }
    fn buttons(&self) {
        let assigned = self.assignments();
        let mapping = model::plan("Preview", &assigned, true);
        let root_selected = self.roots.active_id().is_some();
        let ready = root_selected && mapping.is_ok();
        self.reset_roles
            .set_sensitive(!self.busy.get() && self.package.borrow().is_some());
        let page = self.pages.visible_child_name();
        let review = page.as_deref() == Some("review");
        let installing = page.as_deref() == Some("install");
        let busy = self.busy.get();
        self.step_title.set_text(if self.completed.get() {
            "Theme installed"
        } else if installing {
            "Install your theme"
        } else {
            "Import cursors"
        });
        self.step_title.set_visible(!review);
        self.back.set_visible(installing && !self.completed.get());
        self.back.set_sensitive(!busy);
        self.try_button.set_visible(review);
        self.try_button.set_sensitive(!busy && ready);
        self.confirm.set_sensitive(!busy && ready);
        self.next.set_visible(review || self.completed.get());
        self.next.set_label(if self.completed.get() {
            "Done"
        } else {
            "Continue to install"
        });
        self.next
            .set_sensitive(!busy && (ready || self.completed.get()));
        self.install
            .set_visible(installing && !self.completed.get());
        let name_plan = model::plan(self.name.text().as_str(), &assigned, true);
        self.install.set_sensitive(
            !busy
                && ready
                && name_plan.is_ok()
                && self.confirm.is_active()
                && self.target.active().is_some()
                && !self.completed.get(),
        );
        let reason = if self.completed.get() {
            "Your theme is now in Themes. Apply it from the main window when you are ready.".into()
        } else if busy {
            "Please wait. Close cancels this operation.".into()
        } else if self.package.borrow().is_none() {
            "Choose a file or folder to begin.".into()
        } else if !root_selected {
            "Open a cursor file or package to continue.".into()
        } else if matches!(
            mapping,
            Err(Error::Invalid("multiple files mapped to one role"))
        ) {
            "There are multiple files assigned to one role. Keep one file per role, or click Reset roles to restore filename suggestions.".into()
        } else if let Err(e) = mapping {
            format!(
                "Before continuing: {e}. Assign at least one cursor and use each role only once."
            )
        } else if !installing {
            "Try the mapped theme now, or continue to review its install location. Nothing has been installed.".into()
        } else if self.target.active().is_none() {
            "Installation is unavailable: no user theme directory is in the active lookup paths. You can still go back and try the theme.".into()
        } else if let Err(e) = name_plan {
            format!("Enter a valid theme name: {e}")
        } else if !self.confirm.is_active() {
            "Review the details above, then tick the confirmation to enable Install theme.".into()
        } else {
            "Ready to add a new theme. Existing themes will never be overwritten.".into()
        };
        self.requirements.set_text(&reason);
        self.requirements
            .set_visible(!ready || installing || self.completed.get());
        self.install.set_tooltip_text(Some(&reason));
        self.next.set_tooltip_text(Some(&reason));
        self.try_button.set_tooltip_text(Some(if ready {
            "Try real cursors without installing or applying"
        } else {
            &reason
        }));
    }
    fn preview_row(&self, index: Option<usize>) {
        self.preview.clear();
        let rows = self.rows.borrow();
        let Some((asset, _)) = index.and_then(|i| rows.get(i)) else {
            return;
        };
        self.preview
            .title
            .set_text(&asset.path.display().to_string());
        self.preview
            .subtitle
            .set_text("Uninstalled source · original frame order, sizes and hotspots");
        match model::preview(asset, "source file") {
            Ok(p) => self.preview.load(p, &Environment::capture()),
            Err(e) => {
                self.preview.warning.set_text(&e.to_string());
                self.preview.warning.set_visible(true);
            }
        }
    }
    pub fn start_install(&self) {
        if self.busy.get() {
            return;
        }
        if self.roots.active_id().is_none() {
            self.status
                .set_text("Open a cursor package before confirming an installation.");
            return;
        }
        let plan = match model::plan(
            self.name.text().as_str(),
            &self.assignments(),
            self.confirm.is_active(),
        ) {
            Ok(p) => p,
            Err(e) => {
                self.status.set_text(&e.to_string());
                return;
            }
        };
        let Some(root) = self
            .target
            .active()
            .and_then(|i| self.targets.get(i as usize))
            .cloned()
        else {
            self.status
                .set_text("No user installation directory is available in this mode.");
            return;
        };
        let allowed = self.targets.clone();
        let repo = self.repo.clone();
        let (tx, rx) = mpsc::sync_channel(1);
        let cancel = self.begin(rx, true);
        self.status.set_text(&format!(
            "Generating and validating {} in staging…",
            root.join(plan.name.as_str()).display()
        ));
        std::thread::spawn(move || {
            let cancelled = || cancel.load(Ordering::Relaxed);
            let result = repo.refreshed().scan(&cancelled).and_then(|catalog| {
                if catalog
                    .themes
                    .iter()
                    .chain(&catalog.candidates)
                    .any(|t| t.name == plan.name)
                {
                    Err(Error::Invalid(
                        "theme name already exists in discovered themes; choose a new name",
                    ))
                } else {
                    files::install(&plan, &root, &allowed, &cancelled)
                }
            });
            let _ = tx.send(ResultMessage::Installed(result));
        });
    }
    pub fn close(&self) {
        let chooser = self.chooser.borrow_mut().take();
        if let Some(c) = chooser {
            c.destroy();
        }
        self.cancelled.borrow().store(true, Ordering::Relaxed);
        self.trial.clear_import();
        self.trial.window.set_visible(false);
        if self.busy.get() {
            self.close_pending.set(true);
            self.status.set_text(if self.installing.get() {
                "Cancelling staging; waiting for the commit outcome…"
            } else {
                "Cancelling read…"
            });
        } else {
            self.window.set_visible(false);
            self.preview.clear();
            self.package.borrow_mut().take();
            self.rows.borrow_mut().clear();
            self.roots.remove_all();
            self.confirm.set_active(false);
            self.completed.set(false);
            self.mappings.borrow_mut().clear();
            self.thumbnails.borrow_mut().clear();
            self.name.set_text("");
            self.source_name
                .set_text("Choose a file or an extracted theme folder.");
            self.source_status
                .set_text("Your files stay unchanged. You can preview before installing.");
            self.show_page("review");
        }
    }
    fn tick(self: &Rc<Self>) {
        let msg = self
            .receiver
            .borrow()
            .as_ref()
            .and_then(|rx| match rx.try_recv() {
                Ok(m) => Some(m),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(_) => Some(ResultMessage::Read(Err(Error::Io(
                    "Import worker disconnected".into(),
                )))),
            });
        if let Some(msg) = msg {
            self.receiver.borrow_mut().take();
            self.busy.set(false);
            self.controls.set_sensitive(true);
            match msg {
                ResultMessage::Read(Ok(p)) if !self.cancelled.borrow().load(Ordering::Relaxed) => {
                    let roots = p.roots();
                    *self.package.borrow_mut() = Some(p);
                    self.roots.remove_all();
                    // The entire package is the visible default scope. Folder filtering
                    // is optional; final confirmation covers the included files and roles.
                    self.roots.append(Some(""), "All files in this package");
                    for r in &roots {
                        if !r.as_os_str().is_empty() {
                            self.roots
                                .append(Some(&r.to_string_lossy()), &r.display().to_string());
                        }
                    }
                    self.roots.set_active_id(Some(""));
                    self.rebuild();
                    self.show_page("review");
                    self.status.set_text("");
                }
                ResultMessage::Read(Ok(_)) => self.status.set_text("Read cancelled."),
                ResultMessage::Read(Err(e)) => {
                    self.status.set_text(&format!("Import stopped: {e}"));
                    self.source_status.set_text(&format!("Import stopped: {e}"));
                }
                ResultMessage::Installed(Err(e)) => {
                    self.status.set_text(&format!("Installation stopped: {e}"));
                }
                ResultMessage::Installed(Ok(p)) => {
                    // A cancellation arriving after atomic commit is a successful install.
                    // Keep its outcome visible rather than hiding it as a cancelled operation.
                    self.close_pending.set(false);
                    self.status.set_text(&format!("Installed {}. Desktop settings unchanged. Select Apply in the main window when ready.",p.display()));
                    self.confirm.set_active(false);
                    self.completed.set(true);
                    if let Some(n) = p
                        .file_name()
                        .and_then(|s| s.to_str())
                        .and_then(|s| ThemeName::new(s).ok())
                    {
                        (self.installed)(n);
                    }
                }
            }
            self.buttons();
            if self.close_pending.replace(false) {
                self.close();
            }
        }
        self.preview.tick(
            self.window.is_visible()
                && self.pages.visible_child_name().as_deref() == Some("review"),
        );
        self.tick_thumbnails();
        self.trial.tick();
    }
    fn tick_thumbnails(&self) {
        let adj = self.list_scroll.vadjustment();
        let lo = adj.value() - 80.;
        let hi = adj.value() + adj.page_size() + 80.;
        let visible = self.list.is_mapped();
        let mut kept = 0;
        let mut created = 0;
        for (index, picture) in self.thumbnails.borrow().iter().enumerate() {
            let near = visible
                && kept < 16
                && self
                    .list
                    .row_at_index(index as i32)
                    .and_then(|r| r.compute_bounds(&self.list))
                    .is_some_and(|b| (b.y() + b.height()) as f64 >= lo && (b.y() as f64) <= hi);
            if near {
                kept += 1;
                if picture.paintable().is_none()
                    && created < 2
                    && let Some(frame) = self
                        .rows
                        .borrow()
                        .get(index)
                        .and_then(|(a, _)| a.decoded.as_ref().ok())
                        .and_then(|d| d.variants.first())
                        .and_then(|v| v.frames.first())
                {
                    picture.set_paintable(Some(&super::texture(frame)));
                    created += 1;
                }
            } else {
                picture.set_paintable(None::<&gtk::gdk::Texture>);
            }
        }
    }
}

// Editable suggestion only; plan() still validates the final name at installation.
fn suggested_name(source: &str) -> String {
    let path = std::path::Path::new(source);
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let name: String = stem
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || "-_".contains(*c))
        .take(80)
        .collect();
    if name.is_empty() {
        "Imported-Cursors".into()
    } else {
        name
    }
}
