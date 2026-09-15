use super::{
    label,
    preview::PreviewPane,
    theme_list::ThemeList,
    worker::{Job, Output, Worker},
};
use cursormochi_app::{
    browser::{Browser, role_choices},
    *,
};
use cursormochi_core::*;
use cursormochi_platform::*;
use gtk::{gdk, prelude::*};
use std::{
    cell::RefCell,
    path::PathBuf,
    rc::Rc,
    time::{Duration, Instant},
};
pub(super) struct State {
    pub catalog: Catalog,
    pub browser: Browser,
    pub request: u64,
    pub controller: Controller,
    pub scanning: bool,
    pub started: Instant,
    pub current: Snapshot,
    pub catalog_epoch: u64,
}
fn current_theme(snapshot: &Snapshot) -> Option<&str> {
    match snapshot.get(&Key::Theme).map(|s| &s.effective) {
        Some(Value::Text(s)) => Some(s),
        _ => None,
    }
}
fn size(snapshot: &Snapshot) -> i32 {
    match snapshot.get(&Key::Size).map(|s| &s.effective) {
        Some(Value::Int(n)) => *n,
        _ => 24,
    }
}
fn outcome(status: &Status, controller: &Controller) -> String {
    match status {
        Status::Observed => "Settings updated. Please check your actual pointer.".into(),
        Status::NoOp => "Already in use. No settings were changed.".into(),
        Status::Writing | Status::VerificationPending => {
            "Applying settings… waiting for verification.".into()
        }
        _ => format!("{status:?}: {}", controller.messages.join("; ")),
    }
}
pub fn build(app: &gtk::Application, fixture: bool, smoke: bool) {
    let env = Environment::capture();
    let paths = if fixture {
        Paths::fixture(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures"))
    } else {
        Paths::from_env(&env)
    };
    let settings = if fixture {
        super::fixture_settings()
    } else {
        GnomeSettings::host(&env)
    };
    let port: Rc<dyn DesktopSettingsPort> = match settings {
        Ok(s) => Rc::new(s),
        Err(e) => Rc::new(ReadOnlySettings(e)),
    };
    let repo = Repository::new(paths.clone());
    let (worker, rx) = Worker::new(repo.clone());
    let (thumb_worker, thumb_rx) = Worker::new(repo);
    let state = Rc::new(RefCell::new(State {
        catalog: Catalog::default(),
        browser: Browser::default(),
        request: 0,
        controller: Controller::default(),
        scanning: true,
        started: Instant::now(),
        current: port.read().unwrap_or_default(),
        catalog_epoch: 0,
    }));
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title(if fixture {
            "CursorMochi · Preview mode"
        } else {
            "CursorMochi"
        })
        .default_width(980)
        .default_height(760)
        .build();
    let css = gtk::CssProvider::new();
    css.load_from_data(include_str!("style.css"));
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &css,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
    let header = gtk::HeaderBar::new();
    header.set_title_widget(Some(&label("CursorMochi")));
    let refresh = gtk::Button::with_label("Refresh");
    header.pack_start(&refresh);
    let menu = gtk::MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .tooltip_text("Diagnostics and information")
        .build();
    let popover = gtk::Popover::new();
    let menu_box = gtk::Box::new(gtk::Orientation::Vertical, 8);
    menu_box.add_css_class("menu-content");
    let copy = gtk::Button::with_label("Copy diagnostics");
    let candidates = gtk::Button::with_label("Candidate diagnostics");
    menu_box.append(&copy);
    menu_box.append(&candidates);
    popover.set_child(Some(&menu_box));
    menu.set_popover(Some(&popover));
    header.pack_end(&menu);
    window.set_titlebar(Some(&header));
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    window.set_child(Some(&root));
    let current = label("Reading current settings…");
    current.add_css_class("current-setting");
    root.append(&current);
    root.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    let pane = gtk::Paned::new(gtk::Orientation::Horizontal);
    pane.set_vexpand(true);
    pane.set_position(280);
    root.append(&pane);
    let list = Rc::new(RefCell::new(ThemeList::new()));
    pane.set_start_child(Some(&list.borrow().root));
    let right = gtk::Box::new(gtk::Orientation::Vertical, 0);
    right.set_hexpand(true);
    pane.set_end_child(Some(&right));
    let preview = PreviewPane::new();
    let detail_scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .child(&preview.root)
        .build();
    right.append(&detail_scroll);
    right.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    let footer = gtk::Box::new(gtk::Orientation::Vertical, 10);
    footer.add_css_class("action-area");
    right.append(&footer);
    let capability = label(if fixture {
        "Preview mode · desktop settings are read-only."
    } else if port.capability().writable {
        "Theme lookup paths are unverified. Applying updates configuration; check your actual pointer."
    } else {
        "Desktop settings are unavailable in this session. You can still browse and preview."
    });
    capability.add_css_class("dim-label");
    capability.set_tooltip_text(Some(&port.capability().details.join("\n")));
    footer.append(&capability);
    let size_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let opt = gtk::CheckButton::with_label("Change cursor size");
    let setting_size = gtk::SpinButton::with_range(1., 256., 1.);
    setting_size.set_value(size(&state.borrow().current) as f64);
    setting_size.set_sensitive(false);
    size_row.append(&opt);
    size_row.append(&setting_size);
    footer.append(&size_row);
    let status = label("Finding verified cursor themes…");
    footer.append(&status);
    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let apply = gtk::Button::with_label("Select a theme");
    apply.add_css_class("suggested-action");
    apply.set_hexpand(true);
    apply.set_sensitive(false);
    let undo = gtk::Button::with_label("Undo last change");
    undo.set_sensitive(false);
    buttons.append(&undo);
    buttons.append(&apply);
    footer.append(&buttons);
    {
        let n = setting_size.clone();
        opt.connect_toggled(move |b| n.set_sensitive(b.is_active()));
    }
    let request: Rc<dyn Fn()> = {
        let s = state.clone();
        let worker = worker.clone();
        let p = preview.clone();
        let status = status.clone();
        Rc::new(move || {
            p.clear();
            let mut s = s.borrow_mut();
            if s.scanning {
                return;
            }
            if let (Some(t), Some(role)) = (s.browser.selected.clone(), p.roles.active_id()) {
                s.request = worker.submit(Job::Preview(t, role.to_string()));
                p.roles
                    .set_tooltip_text(Some(&format!("Cursor file: {role}")));
                status.set_text("Loading preview…");
            }
        })
    };
    {
        let request = request.clone();
        preview.roles.connect_changed(move |_| request());
    }
    {
        let s = state.clone();
        let p = preview.clone();
        let list_widget = list.borrow().list.clone();
        list_widget.connect_row_selected(move |_, row| {
            let Some(row) = row else { return };
            let theme = {
                let s = s.borrow();
                s.catalog.themes.get(row.index() as usize).cloned()
            };
            if let Some(t) = theme {
                s.borrow_mut().browser.selected = Some(t.name.clone());
                p.title.set_text(&t.display);
                p.title
                    .set_tooltip_text(Some(&format!("{} · {}", t.display, t.name.as_str())));
                p.subtitle.set_text(&format!(
                    "{} · {} source location(s){}",
                    t.availability.label(),
                    t.locations.len(),
                    if t.issues.is_empty() {
                        ""
                    } else {
                        " · Some roles could not be verified"
                    }
                ));
                p.subtitle.set_tooltip_text(
                    (!t.issues.is_empty())
                        .then(|| t.issues.join("\n"))
                        .as_deref(),
                );
                p.roles.remove_all();
                for (id, title) in role_choices(&t.verified_roles) {
                    p.roles.append(Some(&id), &title)
                }
                p.roles.set_active(Some(0));
            }
        });
    }
    {
        let s = state.clone();
        let list = list.clone();
        let search = list.borrow().search.clone();
        search.connect_search_changed(move |entry| {
            let mut s = s.borrow_mut();
            s.browser.query = entry.text().to_string();
            list.borrow().filter(&s.catalog.themes, &s.browser.query);
        });
    }
    let rescan: Rc<dyn Fn()> = {
        let s = state.clone();
        let worker = worker.clone();
        let p = preview.clone();
        let status = status.clone();
        Rc::new(move || {
            p.clear();
            let mut s = s.borrow_mut();
            s.scanning = true;
            s.request = worker.submit(Job::Scan);
            s.catalog_epoch += 1;
            status.set_text("Checking cursor sources…");
        })
    };
    {
        let rescan = rescan.clone();
        refresh.connect_clicked(move |_| rescan());
    }
    {
        let s = state.clone();
        let env = env.clone();
        let paths = paths.clone();
        let port = port.clone();
        let p = preview.clone();
        let w = window.downgrade();
        copy.connect_clicked(move |_|{if let Some(w)=w.upgrade(){let s=s.borrow();w.clipboard().set_text(&redact(&format!("CursorMochi {}\n{POLICY}\n{paths:?}\nGTK {}.{}.{}; backend={:?}\ncapability={:?}\nsettings={:?}\nselected={:?}\npreview={}\ncandidates={:?}\nscan={:?}\noperation={:?}\nmessages={:?}",env!("CARGO_PKG_VERSION"),gtk::major_version(),gtk::minor_version(),gtk::micro_version(),gdk::Display::default().map(|d|d.type_().name()),port.capability(),port.read(),s.browser.selected,p.source_summary(),s.catalog.candidates,s.catalog.diagnostics,s.controller.last_operation,s.controller.messages),&env));}});
    }
    {
        let s = state.clone();
        let env = env.clone();
        let w = window.downgrade();
        candidates.connect_clicked(move |_| {
            if let Some(w) = w.upgrade() {
                let dialog = gtk::Window::builder()
                    .title("Candidate diagnostics")
                    .transient_for(&w)
                    .modal(true)
                    .default_width(620)
                    .default_height(420)
                    .build();
                let text = {
                    let s = s.borrow();
                    format!(
                        "{} verified themes · {} excluded candidates\n\n{}\n\n{}",
                        s.catalog.themes.len(),
                        s.catalog.candidates.len(),
                        s.catalog
                            .candidates
                            .iter()
                            .map(|t| format!(
                                "{} · {}\n{:?}",
                                t.name.as_str(),
                                t.availability.label(),
                                t.availability
                            ))
                            .collect::<Vec<_>>()
                            .join("\n\n"),
                        s.catalog.diagnostics.join("\n")
                    )
                };
                let l = label(&redact(&text, &env));
                l.set_selectable(true);
                l.set_wrap_mode(gtk::pango::WrapMode::WordChar);
                l.add_css_class("menu-content");
                dialog.set_child(Some(&gtk::ScrolledWindow::builder().child(&l).build()));
                dialog.present();
            }
        });
    }
    {
        let s = state.clone();
        let port = port.clone();
        let p = preview.clone();
        let opt = opt.clone();
        let n = setting_size.clone();
        let status = status.clone();
        apply.connect_clicked(move |_| {
            let mut s = s.borrow_mut();
            let Some(theme) = s.browser.selected.clone() else {
                return;
            };
            let valid = s
                .catalog
                .themes
                .iter()
                .any(|t| t.name == theme && t.availability.usable())
                && p.has_data()
                && !s.scanning;
            match s.controller.prepare(
                port.as_ref(),
                ChangeIntent {
                    theme,
                    size: opt.is_active().then(|| n.value_as_int()),
                    resolved: valid,
                },
            ) {
                Ok(plan) => {
                    let result = s.controller.begin(port.as_ref(), plan, false);
                    s.started = Instant::now();
                    status.set_text(&outcome(&result, &s.controller));
                }
                Err(e) => status.set_text(&e),
            }
        });
    }
    {
        let s = state.clone();
        let port = port.clone();
        let status = status.clone();
        undo.connect_clicked(move |_| {
            let mut s = s.borrow_mut();
            let result = s.controller.begin_undo(port.as_ref());
            s.started = Instant::now();
            status.set_text(&outcome(&result, &s.controller));
        });
    }
    {
        let s = state.clone();
        let status = status.clone();
        window.connect_close_request(move |_| {
            if s.borrow().controller.busy() {
                status.set_text("A change is being verified. Please wait before closing.");
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
    }
    rescan();
    let weak = window.downgrade();
    let mut settings_at = Instant::now() - Duration::from_secs(1);
    let mut thumbs_at = settings_at;
    let mut pending_thumb: Option<(u64, usize, u64)> = None;
    let mut writable = port.capability().writable;
    let mut smoke_run = super::smoke::Smoke::new(smoke, &state.borrow().current);
    let app = app.clone();
    glib::timeout_add_local(Duration::from_millis(16), move || {
        let Some(window) = weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        if smoke_run.before_tick(&window, &root, &app) {
            return glib::ControlFlow::Continue;
        }
        while let Ok((id, out)) = rx.try_recv() {
            if id != state.borrow().request {
                continue;
            }
            match out {
                Output::Scan(Ok(catalog)) => {
                    pending_thumb = None;
                    let notice = {
                        let mut s = state.borrow_mut();
                        s.catalog = catalog;
                        s.scanning = false;
                        let current = current_theme(&s.current).map(str::to_owned);
                        let themes = s.catalog.themes.clone();
                        s.browser.reconcile(&themes, current.as_deref())
                    };
                    {
                        let s = state.borrow();
                        let mut l = list.borrow_mut();
                        l.rebuild(&s.catalog.themes, current_theme(&s.current));
                        l.filter(&s.catalog.themes, &s.browser.query);
                    }
                    let selection = {
                        let s = state.borrow();
                        s.browser
                            .selected
                            .as_ref()
                            .and_then(|name| s.catalog.themes.iter().position(|t| &t.name == name))
                    };
                    let target = selection.and_then(|i| list.borrow().list.row_at_index(i as i32));
                    if let Some(row) = target {
                        list.borrow().list.select_row(Some(&row));
                        let row = row.clone();
                        let scroll = list.borrow().scroll.clone();
                        let list_widget = list.borrow().list.clone();
                        glib::idle_add_local_once(move || {
                            if let Some(b) = row.compute_bounds(&list_widget) {
                                scroll
                                    .vadjustment()
                                    .clamp_page(b.y() as f64, (b.y() + b.height()) as f64);
                            }
                        });
                    } else {
                        preview.clear();
                        preview.title.set_text("Choose a cursor theme");
                        preview
                            .subtitle
                            .set_text("Select a verified theme from the list.");
                    }
                    status.set_text(&notice.unwrap_or_else(|| {
                        format!(
                            "{} verified themes · {} candidates excluded",
                            state.borrow().catalog.themes.len(),
                            state.borrow().catalog.candidates.len()
                        )
                    }));
                }
                Output::Preview(Ok(p)) => {
                    preview.load(p, &env);
                    status.set_text("Your desktop settings are unchanged.");
                }
                Output::Scan(Err(e)) => {
                    let mut s = state.borrow_mut();
                    s.scanning = false;
                    s.browser.selected = None;
                    s.catalog = Catalog::default();
                    preview.clear();
                    drop(s);
                    list.borrow_mut().rebuild(&[], None);
                    list.borrow().filter(&[], "");
                    list.borrow().empty.set_text(
                        "Could not read cursor themes.\nTry Refresh or open diagnostics.",
                    );
                    status.set_text(&format!("Scan failed: {e}"));
                }
                Output::Preview(Err(e)) => {
                    preview.clear();
                    preview.warning.set_text(&format!(
                        "Preview unavailable: {e}. Try another role or refresh."
                    ));
                    preview.warning.set_visible(true);
                    status
                        .set_text("Cannot apply this selection while its preview is unavailable.");
                }
                Output::Thumbnail(_) => (),
            }
        }
        while let Ok((id, out)) = thumb_rx.try_recv() {
            if let Some((expected, index, epoch)) = pending_thumb
                && id == expected
            {
                pending_thumb = None;
                if epoch == state.borrow().catalog_epoch {
                    let mut l = list.borrow_mut();
                    let near = l.near_indices();
                    if near.contains(&index)
                        && let Some(row) = l.rows.get_mut(index)
                    {
                        row.thumb_done = true;
                        if let Output::Thumbnail(Ok((frame, _))) = out {
                            row.image.set_paintable(Some(&super::texture(&frame)));
                        } else {
                            row.image.set_tooltip_text(Some(
                                "Thumbnail unavailable. Refresh to verify the source.",
                            ));
                        }
                    }
                }
            }
        }
        let now = Instant::now();
        let mut s = state.borrow_mut();
        if s.controller.busy() {
            let expired = s.started.elapsed() > Duration::from_secs(2);
            let result = s.controller.poll(port.as_ref(), expired);
            if result != Status::VerificationPending {
                status.set_text(&outcome(&result, &s.controller));
            }
        }
        if now.duration_since(settings_at) >= Duration::from_millis(250) {
            settings_at = now;
            writable = port.capability().writable;
            match port.read() {
                Ok(snapshot) => {
                    s.current = snapshot;
                    let name = current_theme(&s.current).unwrap_or("Unknown");
                    let missing = if !s.scanning
                        && !s.catalog.themes.iter().any(|t| t.name.as_str() == name)
                    {
                        " · Not in the verified list"
                    } else {
                        ""
                    };
                    current.set_text(&format!(
                        "Current setting: {name} · Cursor size {}{missing}",
                        size(&s.current)
                    ));
                    list.borrow().update_current(Some(name));
                    if !opt.is_active() {
                        setting_size.set_value(size(&s.current) as f64);
                    }
                }
                Err(e) => current.set_text(&format!("Current settings unavailable: {e}")),
            }
        }
        let selected = s.browser.selected.as_ref().map(ThemeName::as_str);
        let same = selected.is_some()
            && selected == current_theme(&s.current)
            && (!opt.is_active() || setting_size.value_as_int() == size(&s.current));
        let busy = s.controller.busy();
        apply.set_label(&if same {
            "Already in use".into()
        } else if let Some(name) = selected {
            format!("Apply {name}")
        } else {
            "Select a theme".into()
        });
        let reason = if busy {
            "A settings change is being verified."
        } else if s.scanning {
            "Wait for theme verification."
        } else if !writable {
            "Desktop settings are read-only in this session."
        } else if !preview.has_data() {
            "Select a verified theme and load its preview first."
        } else if same {
            "This theme and the requested size already match the current settings."
        } else {
            "Apply the selected theme to GNOME settings."
        };
        apply.set_tooltip_text(Some(reason));
        apply.set_sensitive(writable && !busy && !s.scanning && preview.has_data() && !same);
        undo.set_label(
            &s.controller
                .undo
                .as_ref()
                .and_then(|r| current_theme(&r.before))
                .map(|n| format!("Undo to {n}"))
                .unwrap_or_else(|| "Undo last change".into()),
        );
        undo.set_sensitive(writable && !busy && s.controller.undo.is_some());
        undo.set_tooltip_text(Some(if s.controller.undo.is_some() {
            "Restore this session's last successful change, unless settings changed externally."
        } else {
            "No change to undo in this session."
        }));
        refresh.set_sensitive(!busy && !s.scanning);
        list.borrow().root.set_sensitive(!s.scanning);
        preview.roles.set_sensitive(!s.scanning && !busy);
        opt.set_sensitive(!busy && writable);
        setting_size.set_sensitive(!busy && writable && opt.is_active());
        let visible = window.is_mapped()
            && window
                .surface()
                .and_then(|v| v.downcast::<gdk::Toplevel>().ok())
                .is_none_or(|v| !v.state().contains(gdk::ToplevelState::MINIMIZED));
        preview.tick(visible);
        if visible && !s.scanning && now.duration_since(thumbs_at) >= Duration::from_millis(80) {
            thumbs_at = now;
            let mut l = list.borrow_mut();
            let near = l.near_indices();
            if pending_thumb.is_some_and(|(_, i, _)| !near.contains(&i)) {
                pending_thumb = None;
            }
            if pending_thumb.is_none()
                && let Some(i) = near.into_iter().find(|i| !l.rows[*i].thumb_done)
                && let Some(t) = s.catalog.themes.get(i)
                && let Some(role) = t.availability.role()
            {
                let id = thumb_worker.submit(Job::Thumbnail(t.name.clone(), role.into()));
                pending_thumb = Some((id, i, s.catalog_epoch));
            }
        }
        drop(s);
        smoke_run.tick(
            &window,
            &state,
            &list,
            &preview,
            &apply,
            &undo,
            &menu,
            &copy,
            &opt,
            &setting_size,
            &rescan,
            &port,
        );
        glib::ControlFlow::Continue
    });
    window.present();
}
