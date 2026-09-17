use cursormochi_core::{Frame, Preview, Resolution};
use cursormochi_platform::{Environment, redact};
use gtk::{gdk, prelude::*};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};
#[derive(Default)]
struct Playback {
    data: Option<Preview>,
    elapsed: Duration,
    last: Option<Instant>,
    rendered: Option<(usize, usize, i32)>,
    frame: Option<Frame>,
    frames_shown: usize,
}
#[derive(Clone)]
pub struct PreviewPane {
    pub root: gtk::Box,
    pub title: gtk::Label,
    pub subtitle: gtk::Label,
    pub roles: gtk::ComboBoxText,
    pub role_browser: super::role_list::RoleList,
    pub sizes: gtk::ComboBoxText,
    pub zoom: gtk::SpinButton,
    pub hotspot: gtk::CheckButton,
    pub play: gtk::ToggleButton,
    pub background: gtk::ComboBoxText,
    pub technical: gtk::Expander,
    pub warning: gtk::Label,
    pub canvas: gtk::Overlay,
    pub picture: gtk::Picture,
    pub image_scroll: gtk::ScrolledWindow,
    meta: gtk::Label,
    source: gtk::Label,
    caption: gtk::Label,
    mark: gtk::DrawingArea,
    state: Rc<RefCell<Playback>>,
}
impl PreviewPane {
    pub fn new() -> Self {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 10);
        root.add_css_class("detail-pane");
        let title = super::label("Choose a cursor theme");
        title.add_css_class("title-1");
        title.set_wrap(false);
        title.set_ellipsize(gtk::pango::EllipsizeMode::End);
        root.append(&title);
        let subtitle =
            super::label("Browse installed cursor themes without changing your desktop.");
        subtitle.add_css_class("dim-label");
        root.append(&subtitle);
        let warning = super::label("");
        warning.add_css_class("warning-text");
        warning.set_visible(false);
        root.append(&warning);
        let controls = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let roles = gtk::ComboBoxText::new();
        roles.set_hexpand(true);
        roles.set_tooltip_text(Some("Cursor role"));
        controls.append(&roles);
        let background = gtk::ComboBoxText::new();
        for v in ["Light background", "Dark background", "Transparency grid"] {
            background.append_text(v)
        }
        background.set_active(Some(0));
        controls.append(&background);
        root.append(&controls);
        let canvas = gtk::Overlay::new();
        canvas.add_css_class("preview-canvas");
        let bg = gtk::DrawingArea::new();
        bg.set_content_width(280);
        bg.set_content_height(180);
        canvas.set_child(Some(&bg));
        {
            let choice = background.clone();
            bg.set_draw_func(move |_, cr, w, h| match choice.active().unwrap_or(0) {
                3 => (), // Transparent canvas: use the surrounding window background.
                1 => {
                    cr.set_source_rgb(0.15, 0.16, 0.18);
                    let _ = cr.paint();
                }
                2 => {
                    for y in (0..h).step_by(16) {
                        for x in (0..w).step_by(16) {
                            let c = if (x / 16 + y / 16) % 2 == 0 {
                                0.90
                            } else {
                                0.95
                            };
                            cr.set_source_rgb(c, c, c);
                            cr.rectangle(x as f64, y as f64, 16., 16.);
                            let _ = cr.fill();
                        }
                    }
                }
                _ => {
                    cr.set_source_rgb(0.95, 0.96, 0.97);
                    let _ = cr.paint();
                }
            });
        }
        {
            let bg = bg.clone();
            background.connect_changed(move |_| bg.queue_draw());
        }
        let picture = gtk::Picture::new();
        picture.set_can_shrink(false);
        picture.set_halign(gtk::Align::Center);
        picture.set_valign(gtk::Align::Center);
        canvas.add_overlay(&picture);
        canvas.set_measure_overlay(&picture, true);
        let mark = gtk::DrawingArea::new();
        mark.set_can_target(false);
        canvas.add_overlay(&mark);
        let scroller = gtk::ScrolledWindow::builder()
            .min_content_height(180)
            .max_content_height(180)
            .child(&canvas)
            .build();

        let inspect = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let sizes = gtk::ComboBoxText::new();
        sizes.set_tooltip_text(Some(
            "Nominal size in the cursor file. This does not change GNOME settings.",
        ));
        inspect.append(&super::label("Size"));
        inspect.append(&sizes);
        let zoom = gtk::SpinButton::with_range(1., 8., 1.);
        zoom.set_value(3.);
        inspect.append(&super::label("Zoom"));
        inspect.append(&zoom);
        let play = gtk::ToggleButton::with_label("Pause");
        play.set_active(true);
        play.set_visible(false);
        inspect.append(&play);
        root.append(&inspect);
        let caption = super::label("3× inspection · logical pixels, not desktop physical size");
        caption.add_css_class("dim-label");
        root.append(&caption);
        root.append(&scroller);
        let technical = gtk::Expander::builder().label("Technical details").build();
        let tech = gtk::Box::new(gtk::Orientation::Vertical, 10);
        let hotspot = gtk::CheckButton::with_label("Show hotspot");
        hotspot.set_active(false);
        tech.append(&hotspot);
        let meta = super::label("");
        meta.set_selectable(true);
        tech.append(&meta);
        let source = super::label("");
        source.set_selectable(true);
        source.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        tech.append(&source);
        technical.set_child(Some(&tech));
        root.append(&technical);
        let state = Rc::new(RefCell::new(Playback::default()));
        {
            let state = state.clone();
            let zoom = zoom.clone();
            let hotspot = hotspot.clone();
            mark.set_draw_func(move |_, cr, w, h| {
                if !hotspot.is_active() {
                    return;
                }
                let s = state.borrow();
                let Some(f) = &s.frame else { return };
                let z = zoom.value();
                let x = (w as f64 - f.width as f64 * z) / 2. + (f.hotspot.0 as f64 + 0.5) * z;
                let y = (h as f64 - f.height as f64 * z) / 2. + (f.hotspot.1 as f64 + 0.5) * z;
                cr.set_source_rgb(0.94, 0.23, 0.34);
                cr.set_line_width(1.5);
                cr.move_to(x - 6., y);
                cr.line_to(x + 6., y);
                cr.move_to(x, y - 6.);
                cr.line_to(x, y + 6.);
                let _ = cr.stroke();
            });
        }
        {
            let mark = mark.clone();
            hotspot.connect_toggled(move |_| mark.queue_draw());
        }
        play.connect_toggled(|b| b.set_label(if b.is_active() { "Pause" } else { "Play" }));
        let role_browser = super::role_list::RoleList::new(&roles);
        Self {
            root,
            role_browser,
            title,
            subtitle,
            roles,
            sizes,
            zoom,
            hotspot,
            play,
            background,
            technical,
            warning,
            canvas,
            picture,
            image_scroll: scroller,
            meta,
            source,
            caption,
            mark,
            state,
        }
    }
    /// The importer and inspector share the same spacious preview layout.
    pub fn configure_import(&self) {
        self.configure_spacious();
    }
    pub fn configure_inspect(&self) {
        self.configure_spacious();
        if let Some(controls) = self
            .roles
            .parent()
            .and_then(|w| w.downcast::<gtk::Box>().ok())
        {
            controls.remove(&self.roles);
        }
        if let Some(toolbar) = self
            .title
            .parent()
            .and_then(|w| w.downcast::<gtk::Box>().ok())
        {
            toolbar.insert_child_after(&self.roles, Some(&self.title));
        }
        self.roles.set_visible(false);
        let active = super::label("");
        active.add_css_class("dim-label");
        active.set_wrap(false);
        active.set_ellipsize(gtk::pango::EllipsizeMode::End);
        active.set_max_width_chars(24);
        if let Some(toolbar) = self
            .title
            .parent()
            .and_then(|w| w.downcast::<gtk::Box>().ok())
        {
            toolbar.insert_child_after(&active, Some(&self.title));
        }
        let weak = active.downgrade();
        self.roles.connect_changed(move |c| {
            if let Some(active) = weak.upgrade() {
                active.set_text(c.active_text().as_deref().unwrap_or(""));
                active.set_tooltip_text(c.active_id().as_deref());
            }
        });
        self.root.remove(&self.subtitle);
        self.append_details(&self.subtitle);
        if let Some(details) = self.technical.child() {
            self.technical.set_child(None::<&gtk::Widget>);
            self.technical.set_child(Some(
                &gtk::ScrolledWindow::builder()
                    .child(&details)
                    .max_content_height(220)
                    .propagate_natural_height(true)
                    .hscrollbar_policy(gtk::PolicyType::Never)
                    .build(),
            ));
        }
    }
    pub fn append_details(&self, widget: &impl IsA<gtk::Widget>) {
        let details = self.technical.child().and_then(|w| {
            w.downcast_ref::<gtk::ScrolledWindow>()
                .and_then(|s| s.child())
                .and_then(|w| w.downcast::<gtk::Viewport>().ok())
                .and_then(|v| v.child())
                .or(Some(w))
        });
        if let Some(details) = details.and_then(|w| w.downcast::<gtk::Box>().ok()) {
            details.append(widget);
        }
    }
    fn configure_spacious(&self) {
        self.background.append_text("No background");
        self.background.set_active(Some(3));
        self.zoom.set_value(1.);
        self.root.set_vexpand(true);
        self.image_scroll.set_max_content_height(-1);
        self.image_scroll.set_min_content_height(240);
        self.image_scroll.set_vexpand(true);
        let toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        self.root.remove(&self.title);
        self.title.set_hexpand(true);
        self.title.remove_css_class("title-1");
        self.title.add_css_class("theme-name");
        toolbar.append(&self.title);
        if let Some(inspect) = self
            .play
            .parent()
            .and_then(|p| p.downcast::<gtk::Box>().ok())
        {
            inspect.remove(&self.play);
        }
        toolbar.append(&self.play);
        self.root.prepend(&toolbar);
        self.technical.set_label(Some("Preview settings & details"));
        if let Some(details) = self
            .technical
            .child()
            .and_then(|c| c.downcast::<gtk::Box>().ok())
        {
            for widget in [
                self.background.parent(),
                self.sizes.parent(),
                Some(self.caption.clone().upcast()),
            ]
            .into_iter()
            .flatten()
            {
                self.root.remove(&widget);
                details.prepend(&widget);
            }
        }
    }
    pub fn clear(&self) {
        let mut s = self.state.borrow_mut();
        s.data = None;
        s.frame = None;
        s.rendered = None;
        self.picture.set_paintable(None::<&gdk::Texture>);
        self.meta.set_text("");
        self.source.set_text("");
        self.play.set_visible(false);
        self.warning.set_visible(false);
        self.mark.queue_draw();
    }
    pub fn load(&self, p: Preview, env: &Environment) {
        self.sizes.remove_all();
        for v in &p.variants {
            self.sizes.append_text(&v.nominal.to_string())
        }
        let nearest = p
            .variants
            .iter()
            .enumerate()
            .min_by_key(|(_, v)| v.nominal.abs_diff(24))
            .map(|(i, _)| i as u32)
            .unwrap_or(0);
        self.sizes.set_active(Some(nearest));
        self.source.set_text(&redact(
            &format!(
                "Role file: {}\nResolution: {:?}\nSource: {}\nChain: {}",
                p.role,
                p.resolution,
                p.source.display(),
                p.chain.join(" → ")
            ),
            env,
        ));
        let warning = match p.resolution {
            Resolution::Fallback => {
                "This role comes from the default theme, not the selected theme."
            }
            Resolution::Inherited => {
                "This role is inherited. See Preview settings & details for its actual source."
            }
            Resolution::Direct => "",
        };
        self.warning.set_text(warning);
        self.warning.set_visible(!warning.is_empty());
        let mut s = self.state.borrow_mut();
        s.data = Some(p);
        s.elapsed = Duration::ZERO;
        s.last = None;
        s.rendered = None;
        self.play.set_active(true);
    }
    pub fn has_data(&self) -> bool {
        self.state.borrow().data.is_some()
    }
    pub fn source_summary(&self) -> String {
        self.source.text().to_string()
    }
    pub fn frames_shown(&self) -> usize {
        self.state.borrow().frames_shown
    }
    pub fn elapsed(&self) -> Duration {
        self.state.borrow().elapsed
    }
    pub fn tick(&self, visible: bool) {
        let now = Instant::now();
        let mut s = self.state.borrow_mut();
        let delta = s
            .last
            .map(|last| now.duration_since(last))
            .unwrap_or_default();
        s.last = Some(now);
        if !visible {
            return;
        }
        if self.play.is_active() {
            s.elapsed += delta
        }
        let Some(p) = &s.data else { return };
        let vi = self.sizes.active().unwrap_or(0) as usize;
        let Some(v) = p.variants.get(vi) else { return };
        self.play.set_visible(v.frames.len() > 1);
        let fi = v.frame_at(s.elapsed.as_millis() as u64);
        let z = self.zoom.value_as_int();
        let key = (vi, fi, z);
        if s.rendered == Some(key) {
            return;
        }
        let f = &v.frames[fi];
        self.picture.set_paintable(Some(&super::texture(f)));
        self.picture
            .set_size_request(f.width as i32 * z, f.height as i32 * z);
        self.caption.set_text(&if z == 1 {
            "1× logical size · independent of GNOME cursor size".into()
        } else {
            format!("{z}× inspection · logical pixels, not desktop physical size")
        });
        self.meta.set_text(&format!("Nominal size: {}\nActual frame: {} × {}\nFrame: {} / {} · Hotspot: {}, {}\nOriginal delay: {} ms{}",v.nominal,f.width,f.height,fi+1,v.frames.len(),f.hotspot.0,f.hotspot.1,f.delay,if f.delay<16||f.delay>10_000{" (playback limited to 16–10000 ms)"}else{""}));
        s.frame = Some(f.clone());
        s.rendered = Some(key);
        s.frames_shown += 1;
        self.mark.queue_draw();
    }
}
