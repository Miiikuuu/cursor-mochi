//! Persistent role navigation; thumbnails use the existing bounded thumbnail worker.
use cursormochi_core::{Error, Frame, Resolution};
use gtk::{gdk, prelude::*};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
#[derive(Clone)]
pub struct RoleRow {
    pub row: gtk::ListBoxRow,
    pub image: gtk::Image,
    pub id: String,
    pub done: bool,
}
#[derive(Clone)]
pub struct RoleList {
    pub root: gtk::Box,
    pub list: gtk::ListBox,
    pub scroll: gtk::ScrolledWindow,
    pub rows: Rc<RefCell<Vec<RoleRow>>>,
    pub generation: Rc<Cell<u64>>,
    combo: gtk::ComboBoxText,
}
impl RoleList {
    pub fn new(combo: &gtk::ComboBoxText) -> Self {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 8);
        root.add_css_class("role-browser");
        let title = super::label("Cursors");
        title.add_css_class("section-label");
        root.append(&title);
        let list = gtk::ListBox::new();
        list.add_css_class("theme-list");
        list.set_selection_mode(gtk::SelectionMode::Single);
        let scroll = gtk::ScrolledWindow::builder()
            .child(&list)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build();
        root.append(&scroll);
        let rows: Rc<RefCell<Vec<RoleRow>>> = Rc::new(RefCell::new(Vec::new()));
        let data = rows.clone();
        let weak = combo.downgrade();
        list.connect_row_selected(move |_, row| {
            let id = row.and_then(|r| data.borrow().get(r.index() as usize).map(|v| v.id.clone()));
            if let (Some(id), Some(combo)) = (id, weak.upgrade())
                && combo.active_id().as_deref() != Some(&id)
            {
                combo.set_active_id(Some(&id));
            }
        });
        let data = rows.clone();
        let weak = list.downgrade();
        combo.connect_changed(move |c| {
            let selected = data
                .borrow()
                .iter()
                .find(|r| Some(r.id.as_str()) == c.active_id().as_deref())
                .map(|r| r.row.clone());
            if let Some(list) = weak.upgrade() {
                list.select_row(selected.as_ref());
            }
        });
        Self {
            root,
            list,
            scroll,
            rows,
            generation: Rc::new(Cell::new(0)),
            combo: combo.clone(),
        }
    }
    pub fn rebuild(&self, choices: &[(String, String)]) {
        self.generation.set(self.generation.get().wrapping_add(1));
        self.rows.borrow_mut().clear();
        while let Some(child) = self.list.first_child() {
            self.list.remove(&child);
        }
        self.combo.remove_all();
        for (id, title) in choices {
            self.combo.append(Some(id), title);
            let row = gtk::ListBoxRow::new();
            let content = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            content.add_css_class("role-row");
            content.set_can_target(false);
            let image = gtk::Image::new();
            image.set_pixel_size(44);
            image.set_size_request(44, 44);
            let labels = gtk::Box::new(gtk::Orientation::Vertical, 3);
            labels.set_hexpand(true);
            let label = super::label(title.split(" · ").next().unwrap_or(title));
            label.set_wrap(false);
            label.set_ellipsize(gtk::pango::EllipsizeMode::End);
            label.set_max_width_chars(16);
            let file = super::label(id);
            file.set_wrap(false);
            file.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
            file.set_max_width_chars(16);
            file.add_css_class("dim-label");
            labels.append(&label);
            labels.append(&file);
            content.append(&image);
            content.append(&labels);
            row.set_child(Some(&content));
            row.set_tooltip_text(Some(&format!(
                "{title}\nCursor file: {id}\nLoading thumbnail…"
            )));
            self.list.append(&row);
            self.rows.borrow_mut().push(RoleRow {
                row,
                image,
                id: id.clone(),
                done: false,
            });
        }
        self.scroll.vadjustment().set_value(0.);
        if !choices.is_empty() {
            self.combo.set_active(Some(0));
        }
    }
    pub fn near_indices(&self) -> Vec<usize> {
        let adj = self.scroll.vadjustment();
        let lo = adj.value() - 64.;
        let hi = adj.value() + adj.page_size() + 64.;
        let mut near = Vec::new();
        for (index, row) in self.rows.borrow_mut().iter_mut().enumerate() {
            if self.root.is_mapped()
                && near.len() < 16
                && row
                    .row
                    .compute_bounds(&self.list)
                    .is_some_and(|b| (b.y() + b.height()) as f64 >= lo && b.y() as f64 <= hi)
            {
                near.push(index);
            } else {
                row.image.set_paintable(None::<&gdk::Texture>);
                row.done = false;
            }
        }
        near
    }
    pub fn finish(
        &self,
        generation: u64,
        index: usize,
        result: Result<(Frame, Resolution), Error>,
    ) {
        if generation != self.generation.get() || !self.near_indices().contains(&index) {
            return;
        }
        if let Some(row) = self.rows.borrow_mut().get_mut(index) {
            row.done = true;
            match result {
                Ok((frame, source)) => {
                    row.image
                        .set_paintable(super::thumbnail::paintable(&frame).as_ref());
                    row.row.set_tooltip_text(Some(&format!("Cursor file: {}\n{source:?} · {}×{} source pixels\nSelect to inspect animation and full source details.", row.id, frame.width, frame.height)));
                }
                Err(e) => row.row.set_tooltip_text(Some(&format!(
                    "{}: thumbnail unavailable ({e}). Select for details or refresh.",
                    row.id
                ))),
            }
        }
    }
}
