use cursormochi_app::browser;
use cursormochi_core::Theme;
use gtk::{gdk, prelude::*};
#[derive(Clone)]
pub struct Row {
    pub row: gtk::ListBoxRow,
    pub image: gtk::Picture,
    pub badge: gtk::Label,
    pub base: String,
    pub id: String,
    pub thumb_done: bool,
}
#[derive(Clone)]
pub struct ThemeList {
    pub root: gtk::Box,
    pub search: gtk::SearchEntry,
    pub list: gtk::ListBox,
    pub scroll: gtk::ScrolledWindow,
    pub empty: gtk::Label,
    pub count: gtk::Label,
    pub rows: Vec<Row>,
}
impl ThemeList {
    pub fn new() -> Self {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 12);
        root.add_css_class("browser-pane");
        let search = gtk::SearchEntry::new();
        search.set_placeholder_text(Some("Search themes"));
        root.append(&search);
        let count = super::label("Finding cursor themes…");
        count.add_css_class("dim-label");
        root.append(&count);
        let list = gtk::ListBox::new();
        list.add_css_class("theme-list");
        list.set_selection_mode(gtk::SelectionMode::Single);
        let scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .child(&list)
            .build();
        root.append(&scroll);
        let empty = super::label("");
        empty.set_halign(gtk::Align::Center);
        root.append(&empty);
        Self {
            root,
            search,
            list,
            scroll,
            empty,
            count,
            rows: vec![],
        }
    }
    pub fn rebuild(&mut self, themes: &[Theme], current: Option<&str>) {
        while let Some(child) = self.list.first_child() {
            self.list.remove(&child)
        }
        self.rows.clear();
        for t in themes {
            let row = gtk::ListBoxRow::new();
            let box_ = gtk::Box::new(gtk::Orientation::Horizontal, 12);
            box_.set_can_target(false);
            box_.add_css_class("theme-row");
            let image = gtk::Picture::new();
            image.set_size_request(40, 40);
            image.add_css_class("thumbnail-tile");
            image.set_halign(gtk::Align::Center);
            image.set_valign(gtk::Align::Center);
            box_.append(&image);
            let text = gtk::Box::new(gtk::Orientation::Vertical, 3);
            text.set_hexpand(true);
            let name = super::label(&t.display);
            name.set_wrap(false);
            name.set_ellipsize(gtk::pango::EllipsizeMode::End);
            name.set_max_width_chars(22);
            name.add_css_class("theme-name");
            text.append(&name);
            let base = if themes.iter().filter(|v| v.display == t.display).count() > 1
                || t.display != t.name.as_str()
            {
                format!("{} · {}", t.name.as_str(), t.availability.label())
            } else {
                t.availability.label().into()
            };
            let badge = super::label(&base);
            badge.set_wrap(false);
            badge.set_ellipsize(gtk::pango::EllipsizeMode::End);
            badge.add_css_class("dim-label");
            text.append(&badge);
            box_.append(&text);
            row.set_child(Some(&box_));
            row.set_tooltip_text(Some(&format!(
                "{}\nID: {}\n{} source(s)",
                t.display,
                t.name.as_str(),
                t.locations.len()
            )));
            self.list.append(&row);
            self.rows.push(Row {
                row,
                image,
                badge,
                base,
                id: t.name.as_str().into(),
                thumb_done: false,
            });
        }
        self.update_current(current);
    }
    pub fn update_current(&self, current: Option<&str>) {
        for row in &self.rows {
            let text = if current == Some(row.id.as_str()) {
                format!("In use · {}", row.base)
            } else {
                row.base.clone()
            };
            if row.badge.text() != text {
                row.badge.set_text(&text)
            }
        }
    }
    pub fn filter(&self, themes: &[Theme], query: &str) {
        let mut visible = 0;
        for (r, t) in self.rows.iter().zip(themes) {
            let show = browser::matches(t, query);
            r.row.set_visible(show);
            if show {
                visible += 1
            }
        }
        self.count
            .set_text(&format!("{visible} of {} cursor themes", themes.len()));
        self.empty.set_text(if themes.is_empty() {
            "No verified cursor themes found.\nSee candidate diagnostics in the menu."
        } else if visible == 0 {
            "No matching themes.\nTry another search."
        } else {
            ""
        });
        self.empty.set_visible(visible == 0);
        self.scroll.set_visible(visible > 0);
    }
    /// Keep only viewport-adjacent textures, capped independently of the window size.
    pub fn near_indices(&mut self) -> Vec<usize> {
        let adj = self.scroll.vadjustment();
        let lo = adj.value() - 80.;
        let hi = adj.value() + adj.page_size() + 80.;
        let mut near = vec![];
        for (i, r) in self.rows.iter_mut().enumerate() {
            let inside = r.row.is_visible()
                && r.row
                    .compute_bounds(&self.list)
                    .is_some_and(|b| (b.y() + b.height()) as f64 >= lo && (b.y() as f64) <= hi)
                && near.len() < 16;
            if inside {
                near.push(i)
            } else {
                r.image.set_paintable(None::<&gdk::Texture>);
                r.thumb_done = false
            }
        }
        near
    }
}
