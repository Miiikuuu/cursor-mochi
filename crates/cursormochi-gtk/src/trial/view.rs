//! Native view translated from the supplied design reference.
use super::*;
pub(super) struct View {
    pub root: gtk::Box,
    pub notes: gtk::TextView,
    pub reset: gtk::Button,
    pub snap: gtk::CheckButton,
    pub scale: gtk::Scale,
    pub backgrounds: gtk::ComboBoxText,
    pub source_details: gtk::Expander,
    pub icon: gtk::Image,
    pub active_label: gtk::Label,
    pub layout: gtk::Paned,
    pub popout: gtk::Button,
    pub issues: gtk::MenuButton,
    pub playground: gtk::Box,
    pub title: gtk::Label,
    pub active: Rc<Cell<usize>>,
    pub size: gtk::SpinButton,
    pub sources: gtk::Label,
    pub entry: gtk::Entry,
    pub link: gtk::Button,
    pub board: gtk::Fixed,
    pub card: gtk::Box,
    pub move_gesture: gtk::GestureDrag,
    pub resize_gesture: gtk::GestureDrag,
    pub live: Rc<RefCell<Live>>,
    pub description: gtk::Label,
}
pub(super) fn build(window: &gtk::Window) -> View {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 16);
    root.add_css_class("playground-page");
    window.set_child(Some(&root));
    let heading = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    heading.add_css_class("playground-heading");
    let icon = gtk::Image::new();
    icon.set_pixel_size(64);
    icon.set_size_request(64, 64);
    icon.set_halign(gtk::Align::Center);
    icon.set_valign(gtk::Align::Center);
    icon.set_tooltip_text(Some("Actual cursor frame from the active playground role"));
    heading.append(&icon);
    let lockup = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    lockup.set_hexpand(true);
    let title = label("Choose a theme");
    title.add_css_class("theme-title");
    title.set_wrap(false);
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    let active_label = label("Normal");
    active_label.add_css_class("dim-label");
    lockup.append(&title);
    lockup.append(&active_label);
    heading.append(&lockup);
    let size = gtk::SpinButton::with_range(16., 64., 8.);
    size.set_value(24.);
    size.set_valign(gtk::Align::Center);
    size.set_tooltip_text(Some(
        "Trial nominal size. Independent of inspection zoom and desktop size.",
    ));

    let backgrounds = gtk::ComboBoxText::new();
    for (id, text) in [("light", "Light"), ("dark", "Dark"), ("grid", "Grid")] {
        backgrounds.append(Some(id), text);
    }
    backgrounds.set_active(Some(0));
    backgrounds.set_valign(gtk::Align::Center);
    backgrounds.set_tooltip_text(Some("Playground background"));
    let swatches = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let mut first: Option<gtk::ToggleButton> = None;
    for (id, title) in [
        ("light", "Light background"),
        ("dark", "Dark background"),
        ("grid", "Checkerboard background"),
    ] {
        let b = gtk::ToggleButton::new();
        b.set_tooltip_text(Some(title));
        b.set_valign(gtk::Align::Center);
        b.add_css_class("background-swatch");
        b.add_css_class("flat");
        b.add_css_class(&format!("swatch-{id}"));
        if let Some(first) = &first {
            b.set_group(Some(first));
        } else {
            first = Some(b.clone());
            b.set_active(true);
        }
        let combo = backgrounds.clone();
        b.connect_toggled(move |b| {
            if b.is_active() {
                combo.set_active_id(Some(id));
            }
        });
        let weak = b.downgrade();
        backgrounds.connect_changed(move |c| {
            if let Some(b) = weak.upgrade() {
                b.set_active(c.active_id().as_deref() == Some(id));
            }
        });
        swatches.append(&b);
    }

    root.append(&heading);
    let playground = gtk::Box::new(gtk::Orientation::Vertical, 0);
    playground.add_css_class("playground");
    playground.add_css_class("playground-light");
    root.append(&playground);
    {
        let weak = playground.downgrade();
        backgrounds.connect_changed(move |b| {
            if let Some(w) = weak.upgrade() {
                for c in ["playground-light", "playground-dark", "playground-grid"] {
                    w.remove_css_class(c);
                }
                w.add_css_class(&format!(
                    "playground-{}",
                    b.active_id().as_deref().unwrap_or("light")
                ));
            }
        });
    }
    let chrome = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    chrome.add_css_class("playground-chrome");
    chrome.append(&gtk::Image::from_icon_name("view-grid-symbolic"));
    let name = label("Playground");
    name.set_hexpand(true);
    chrome.append(&name);
    chrome.append(&gtk::Label::new(Some("Cursor size")));
    chrome.append(&size);
    chrome.append(&swatches);
    let reset = gtk::Button::with_label("↶  Reset");
    reset.add_css_class("flat");
    chrome.append(&reset);
    let popout = gtk::Button::from_icon_name("window-new-symbolic");
    popout.add_css_class("flat");
    popout.set_tooltip_text(Some("Open trial window"));
    chrome.append(&popout);
    playground.append(&chrome);
    let content = gtk::Box::new(gtk::Orientation::Vertical, 18);
    content.add_css_class("playground-content");
    playground.append(&content);
    let layout = gtk::Paned::new(gtk::Orientation::Horizontal);
    layout.set_shrink_start_child(false);
    layout.set_shrink_end_child(false);
    layout.add_css_class("editor-layout");
    content.append(&layout);
    let note_panel = gtk::Box::new(gtk::Orientation::Vertical, 10);
    note_panel.set_hexpand(true);
    layout.set_start_child(Some(&note_panel));
    note_panel.add_css_class("notes-panel");
    let note_top = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    note_top.add_css_class("editor-toolbar");
    let notes_title = label("Notes");
    notes_title.set_hexpand(true);
    notes_title.add_css_class("section-label");
    note_top.append(&notes_title);
    let bold = gtk::ToggleButton::with_label("B");
    bold.set_tooltip_text(Some("Bold note text"));
    bold.add_css_class("flat");
    let italic = gtk::ToggleButton::with_label("I");
    italic.set_tooltip_text(Some("Italic note text"));
    italic.add_css_class("flat");
    note_top.append(&bold);
    note_top.append(&italic);
    note_panel.append(&note_top);
    let entry = gtk::Entry::builder()
        .text("Hello, cursor.")
        .placeholder_text("Note title")
        .build();
    entry.add_css_class("note-title");
    note_panel.append(&entry);
    let notes = gtk::TextView::new();
    notes.set_wrap_mode(gtk::WrapMode::WordChar);
    notes.add_css_class("note-editor");
    notes
        .buffer()
        .set_text("A little space for ideas.\nSelect, edit, and make it yours.");
    let note_scroll = gtk::ScrolledWindow::builder()
        .min_content_height(150)
        .vexpand(true)
        .child(&notes)
        .build();
    note_panel.append(&note_scroll);
    {
        let notes = notes.downgrade();
        bold.connect_toggled(move |b| {
            if let Some(n) = notes.upgrade() {
                if b.is_active() {
                    n.add_css_class("note-bold")
                } else {
                    n.remove_css_class("note-bold")
                }
            }
        });
    }
    {
        let notes = notes.downgrade();
        italic.connect_toggled(move |b| {
            if let Some(n) = notes.upgrade() {
                if b.is_active() {
                    n.add_css_class("note-italic")
                } else {
                    n.remove_css_class("note-italic")
                }
            }
        });
    }
    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let link = gtk::Button::with_label("Open link ↗");
    link.add_css_class("flat");
    link.add_css_class("demo-link");
    link.connect_clicked(|b| b.set_label("Link previewed"));
    actions.append(&link);
    let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    actions.append(&spacer);
    let save = gtk::Button::with_label("Save note");
    save.connect_clicked(|b| b.set_label("Saved"));
    actions.append(&save);
    note_panel.append(&actions);
    let snap = gtk::CheckButton::with_label("Snap to grid");
    let scale_row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    scale_row.append(&label("Scale"));
    let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, 75., 125., 5.);
    scale.set_value(100.);
    scale.set_draw_value(false);
    scale.set_hexpand(true);
    scale.set_tooltip_text(Some("Demo card scale; does not change cursor size"));
    scale_row.append(&scale);
    let canvas_panel = gtk::Box::new(gtk::Orientation::Vertical, 10);
    canvas_panel.set_hexpand(true);
    layout.set_end_child(Some(&canvas_panel));
    canvas_panel.add_css_class("canvas-panel");
    let canvas_label = label("Canvas");
    canvas_label.add_css_class("section-label");
    let canvas_toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    canvas_toolbar.add_css_class("editor-toolbar");
    canvas_label.set_hexpand(true);
    canvas_toolbar.append(&canvas_label);
    canvas_toolbar.append(&snap);
    canvas_panel.append(&canvas_toolbar);
    let canvas = gtk::Overlay::new();
    canvas.add_css_class("demo-canvas");
    canvas.set_vexpand(true);
    canvas_panel.append(&canvas);
    canvas_panel.append(&scale_row);
    let grid = gtk::DrawingArea::new();
    grid.set_content_height(300);
    grid.set_vexpand(true);
    grid.set_content_width(310);
    let point = Rc::new(Cell::new(None::<(f64, f64)>));
    let hit = point.clone();
    let snapping = snap.clone();
    grid.set_draw_func(move |_, cr, w, h| {
        cr.set_source_rgba(0.5, 0.55, 0.62, 0.28);
        for y in (0..h).step_by(12) {
            for x in (0..w).step_by(12) {
                if snapping.is_active() {
                    cr.rectangle(x as f64, y as f64, 12., 12.);
                    cr.set_line_width(0.5);
                    let _ = cr.stroke();
                } else {
                    cr.arc(x as f64 + 6., y as f64 + 6., 0.8, 0., std::f64::consts::TAU);
                    let _ = cr.fill();
                }
            }
        }
        if let Some((x, y)) = hit.get() {
            cr.set_source_rgb(0.5, 0.55, 0.62);
            cr.arc(x, y, 8., 0., std::f64::consts::TAU);
            cr.move_to(x - 12., y);
            cr.line_to(x + 12., y);
            cr.move_to(x, y - 12.);
            cr.line_to(x, y + 12.);
            let _ = cr.stroke();
        }
    });
    {
        let grid = grid.downgrade();
        snap.connect_toggled(move |_| {
            if let Some(g) = grid.upgrade() {
                g.queue_draw();
            }
        });
    }
    canvas.set_child(Some(&grid));
    let board = gtk::Fixed::new();
    board.set_hexpand(true);
    board.set_vexpand(true);
    canvas.add_overlay(&board);
    let card = gtk::Box::new(gtk::Orientation::Vertical, 0);
    card.add_css_class("demo-card");
    card.set_size_request(180, 156);
    let mover = label("⠿  Cover · drag");
    mover.add_css_class("card-handle");
    mover.set_can_focus(true);
    card.append(&mover);
    let body = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    body.set_vexpand(true);
    let art = gtk::DrawingArea::new();
    art.set_hexpand(true);
    art.set_vexpand(true);
    art.add_css_class("cover-art");
    art.set_draw_func(|_, cr, w, h| {
        cr.set_source_rgb(0.58, 0.62, 0.68);
        cr.set_line_width(1.);
        let (x, y) = (w as f64 / 2., h as f64 / 2.);
        cr.move_to(x - 20., y + 14.);
        cr.line_to(x - 5., y - 12.);
        cr.line_to(x + 2., y + 1.);
        cr.line_to(x + 10., y - 8.);
        cr.line_to(x + 23., y + 14.);
        cr.close_path();
        let _ = cr.stroke();
    });
    body.append(&art);
    let horizontal = label("↔");
    horizontal.set_width_request(18);
    horizontal.set_tooltip_text(Some("Resize width"));
    body.append(&horizontal);
    card.append(&body);
    let vertical = label("↕");
    vertical.set_height_request(18);
    vertical.set_tooltip_text(Some("Resize height"));
    let bottom = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    vertical.set_hexpand(true);
    bottom.append(&vertical);
    let diagonal = label("⤡");
    diagonal.set_tooltip_text(Some("Resize width and height"));
    diagonal.set_width_request(24);
    bottom.append(&diagonal);
    card.append(&bottom);
    board.put(&card, 18., 23.);
    let idea = gtk::Box::new(gtk::Orientation::Vertical, 0);
    idea.add_css_class("demo-card");
    idea.set_size_request(160, 126);
    let idea_mover = label("⠿  Idea · drag");
    idea_mover.add_css_class("card-handle");
    idea_mover.set_can_focus(true);
    idea.append(&idea_mover);
    let idea_body = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    idea_body.set_vexpand(true);
    let idea_text = label("✧\nRoom to explore.");
    idea_text.add_css_class("idea-text");
    idea_text.set_hexpand(true);
    idea_body.append(&idea_text);
    let idea_x = label("↔");
    idea_x.set_width_request(18);
    idea_body.append(&idea_x);
    idea.append(&idea_body);
    let idea_y = label("↕");
    idea_y.set_height_request(18);
    let idea_bottom = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    idea_y.set_hexpand(true);
    idea_bottom.append(&idea_y);
    let idea_diagonal = label("⤡");
    idea_diagonal.set_width_request(24);
    idea_bottom.append(&idea_diagonal);
    idea.append(&idea_bottom);
    board.put(&idea, 140., 123.);
    {
        let (b, c, i) = (board.downgrade(), card.downgrade(), idea.downgrade());
        scale.connect_value_changed(move |v| {
            if let (Some(b), Some(c), Some(i)) = (b.upgrade(), c.upgrade(), i.upgrade()) {
                let factor = v.value() / 100.;
                c.set_size_request((180. * factor) as i32, (156. * factor) as i32);
                i.set_size_request((160. * factor) as i32, (126. * factor) as i32);
                b.move_(&c, 18., 23.);
                b.move_(&i, 140., 123.);
            }
        });
    }
    let click = gtk::GestureClick::new();
    let area = grid.downgrade();
    let hit = point.clone();
    click.connect_pressed(move |_, _, x, y| {
        hit.set(Some((x, y)));
        if let Some(a) = area.upgrade() {
            a.queue_draw();
        }
    });
    board.add_controller(click);
    content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    let states = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    states.set_homogeneous(true);
    states.add_css_class("demo-states");
    let run = gtk::Button::with_label("Busy · run task");

    states.append(&run);
    let working = gtk::Button::with_label("Working · start");
    working.connect_clicked(|b| {
        b.set_label(if b.label().as_deref() == Some("Working · start") {
            "Working · stop"
        } else {
            "Working · start"
        });
    });
    working.set_hexpand(true);
    states.append(&working);
    let forbidden = gtk::Button::with_label("Unavailable");

    forbidden.connect_clicked(|b| b.set_label("Unavailable · blocked"));
    states.append(&forbidden);
    content.append(&states);
    {
        let working = working.downgrade();
        let forbidden = forbidden.downgrade();
        reset.connect_clicked(move |_| {
            if let Some(w) = working.upgrade() {
                w.set_label("Working · start");
            }
            if let Some(w) = forbidden.upgrade() {
                w.set_label("Unavailable");
            }
        });
    }
    let description = label("Choose a theme. No desktop changes.");
    description.add_css_class("source-summary");
    let issues = gtk::MenuButton::builder()
        .label("Cursor availability")
        .build();
    issues.set_valign(gtk::Align::Center);
    issues.add_css_class("flat");
    let reasons = gtk::Popover::new();
    description.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    description.set_max_width_chars(48);
    description.add_css_class("menu-content");
    reasons.set_child(Some(
        &gtk::ScrolledWindow::builder()
            .child(&description)
            .min_content_width(320)
            .max_content_height(280)
            .propagate_natural_height(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build(),
    ));
    issues.set_popover(Some(&reasons));
    issues.set_visible(false);
    heading.append(&issues);
    let sources = label("Loading…");
    sources.set_selectable(true);
    let scroll = gtk::ScrolledWindow::builder()
        .min_content_height(150)
        .child(&sources)
        .build();
    let source_details = gtk::Expander::builder()
        .label("Cursor sources & hotspot details")
        .child(&scroll)
        .build();

    for (handle, class, title) in [
        (
            &horizontal,
            "card-resize-x",
            "Drag the right edge to resize width",
        ),
        (
            &idea_x,
            "card-resize-x",
            "Drag the right edge to resize width",
        ),
        (
            &vertical,
            "card-resize-y",
            "Drag the bottom edge to resize height",
        ),
        (
            &idea_y,
            "card-resize-y",
            "Drag the bottom edge to resize height",
        ),
        (
            &diagonal,
            "card-resize-xy",
            "Drag the bottom-right corner to resize",
        ),
        (
            &idea_diagonal,
            "card-resize-xy",
            "Drag the bottom-right corner to resize",
        ),
    ] {
        handle.add_css_class("card-resize-handle");
        handle.add_css_class(class);
        handle.set_xalign(0.5);
        handle.set_tooltip_text(Some(title));
    }
    let active = Rc::new(Cell::new(0));
    let mut regions = Vec::new();
    for (w, role) in [
        (chrome.clone().upcast::<gtk::Widget>(), 0),
        (reset.clone().upcast(), 1),
        (board.clone().upcast::<gtk::Widget>(), 10),
        (card.clone().upcast(), 7),
        (idea.clone().upcast(), 7),
        (diagonal.clone().upcast(), 9),
        (idea_diagonal.clone().upcast(), 9),
        (entry.clone().upcast(), 2),
        (notes.clone().upcast(), 2),
        (link.clone().upcast(), 1),
        (save.clone().upcast(), 1),
        (bold.clone().upcast(), 1),
        (italic.clone().upcast(), 1),
        (snap.clone().upcast(), 1),
        (scale.clone().upcast(), 5),
        (mover.clone().upcast(), 7),
        (art.upcast(), 7),
        (idea_text.upcast(), 7),
        (horizontal.clone().upcast(), 5),
        (vertical.clone().upcast(), 6),
        (idea_mover.clone().upcast(), 7),
        (idea_x.clone().upcast(), 5),
        (idea_y.clone().upcast(), 6),
        (run.clone().upcast(), 3),
        (working.upcast(), 4),
        (forbidden.upcast(), 8),
    ] {
        let desired = Rc::new(RefCell::new(None));
        // Containers own their background only. Card cursors must not overwrite
        // the independent resize handles nested inside them.
        let recursive = w != board.clone().upcast::<gtk::Widget>()
            && w != chrome.clone().upcast::<gtk::Widget>()
            && !w.has_css_class("demo-card");
        if recursive {
            bind_tree(&w, &desired);
        }
        let motion = gtk::EventControllerMotion::new();
        let a = active.clone();
        let index = regions.len();
        motion.connect_enter(move |_, _, _| a.set(index));
        let a = active.clone();
        motion.connect_leave(move |_| a.set(0));
        w.add_controller(motion);
        regions.push(Region {
            widget: w,
            role,
            desired,
            recursive,
        });
    }
    let live = Rc::new(RefCell::new(Live {
        roles: Vec::new(),
        regions,
        elapsed: Duration::ZERO,
        last: Instant::now(),
        drag: None,
        frames_shown: 0,
    }));
    drag(&card, &board, &card, &live, &snap, 7);
    drag(&idea, &board, &idea, &live, &snap, 7);
    let move_gesture = drag(&mover, &board, &card, &live, &snap, 7);
    let resize_gesture = drag(&horizontal, &board, &card, &live, &snap, 5);
    drag(&vertical, &board, &card, &live, &snap, 6);
    drag(&diagonal, &board, &card, &live, &snap, 9);
    drag(&idea_diagonal, &board, &idea, &live, &snap, 9);
    drag(&idea_mover, &board, &idea, &live, &snap, 7);
    drag(&idea_x, &board, &idea, &live, &snap, 5);
    drag(&idea_y, &board, &idea, &live, &snap, 6);
    let run_epoch = Rc::new(Cell::new(0u64));
    {
        let epoch = run_epoch.clone();
        let l = Rc::downgrade(&live);
        run.connect_clicked(move |b| {
            if b.label().as_deref() == Some("Busy · running…") {
                return;
            }
            b.set_label("Busy · running…");
            let id = epoch.get() + 1;
            epoch.set(id);
            if let Some(l) = l.upgrade() {
                for r in &mut l.borrow_mut().regions {
                    if r.widget == b.clone().upcast::<gtk::Widget>() {
                        r.role = 3;
                    }
                }
            }
            let b = b.downgrade();
            let l = l.clone();
            let epoch = epoch.clone();
            glib::timeout_add_local_once(Duration::from_millis(1800), move || {
                if epoch.get() != id {
                    return;
                }
                if let Some(b) = b.upgrade() {
                    b.set_label("Busy · run task");
                    if let Some(l) = l.upgrade() {
                        for r in &mut l.borrow_mut().regions {
                            if r.widget == b.clone().upcast::<gtk::Widget>() {
                                r.role = 3;
                            }
                        }
                    }
                }
            });
        });
    }
    {
        let run = run.downgrade();
        let l = Rc::downgrade(&live);
        reset.connect_clicked(move |_| {
            run_epoch.set(run_epoch.get() + 1);
            if let Some(run) = run.upgrade() {
                run.set_label("Busy · run task");
                if let Some(l) = l.upgrade() {
                    for r in &mut l.borrow_mut().regions {
                        if r.widget == run.clone().upcast::<gtk::Widget>() {
                            r.role = 3;
                        }
                    }
                }
            }
        });
    }
    {
        let (e, n, s, b, i, c, g, bo, it, sv, ln) = (
            entry.clone(),
            notes.clone(),
            snap.clone(),
            board.downgrade(),
            idea.downgrade(),
            card.downgrade(),
            grid.downgrade(),
            bold,
            italic,
            save,
            link.clone(),
        );
        let sc = scale.clone();
        reset.connect_clicked(move |_| {
            e.set_text("Hello, cursor.");
            n.buffer()
                .set_text("A little space for ideas.\nSelect, edit, and make it yours.");
            s.set_active(false);
            sc.set_value(100.);
            bo.set_active(false);
            it.set_active(false);
            sv.set_label("Save note");
            ln.set_label("Open link ↗");
            point.set(None);
            if let Some(g) = g.upgrade() {
                g.queue_draw();
            }
            if let (Some(b), Some(i), Some(c)) = (b.upgrade(), i.upgrade(), c.upgrade()) {
                b.move_(&c, 18., 23.);
                c.set_size_request(180, 156);
                b.move_(&i, 140., 123.);
                i.set_size_request(160, 126);
            }
        });
    }
    View {
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
    }
}
