//! Source-pixel thumbnails: no intermediate cursor-size texture or blurred upscaling.
use cursormochi_core::Frame;
use gtk::{cairo, gdk, prelude::*, subclass::prelude::*};

mod imp {
    use super::*;
    use std::cell::OnceCell;

    #[derive(Default)]
    pub struct Thumbnail {
        pub surface: OnceCell<cairo::ImageSurface>,
    }
    #[glib::object_subclass]
    impl ObjectSubclass for Thumbnail {
        const NAME: &'static str = "CursorMochiThumbnail";
        type Type = super::Thumbnail;
        type Interfaces = (gdk::Paintable,);
    }
    impl ObjectImpl for Thumbnail {}
    impl PaintableImpl for Thumbnail {
        fn flags(&self) -> gdk::PaintableFlags {
            gdk::PaintableFlags::SIZE | gdk::PaintableFlags::CONTENTS
        }
        fn intrinsic_width(&self) -> i32 {
            self.surface.get().map_or(0, |s| s.width())
        }
        fn intrinsic_height(&self) -> i32 {
            self.surface.get().map_or(0, |s| s.height())
        }
        fn intrinsic_aspect_ratio(&self) -> f64 {
            self.surface
                .get()
                .map_or(1., |s| s.width() as f64 / s.height() as f64)
        }
        fn snapshot(&self, snapshot: &gdk::Snapshot, width: f64, height: f64) {
            let Some(surface) = self.surface.get() else {
                return;
            };
            let Some(snapshot) = snapshot.downcast_ref::<gtk::Snapshot>() else {
                return;
            };
            let cr = snapshot.append_cairo(&gtk::graphene::Rect::new(
                0.,
                0.,
                width as f32,
                height as f32,
            ));
            draw(&cr, surface, width, height);
        }
    }
}
fn draw(cr: &cairo::Context, surface: &cairo::ImageSurface, width: f64, height: f64) {
    cr.scale(
        width / surface.width() as f64,
        height / surface.height() as f64,
    );
    if cr.set_source_surface(surface, 0., 0.).is_ok() {
        cr.source().set_extend(cairo::Extend::Pad);
        cr.source().set_filter(
            if width >= surface.width() as f64 && height >= surface.height() as f64 {
                cairo::Filter::Nearest
            } else {
                cairo::Filter::Best
            },
        );
        let _ = cr.paint();
    }
}
glib::wrapper! {
    pub struct Thumbnail(ObjectSubclass<imp::Thumbnail>) @implements gdk::Paintable;
}
/// Pixels are already decoded and premultiplied. Cairo's ARGB32 uses native endian.
/// Only the visible frame allocates a surface; GTK snapshots reuse it.
fn source_surface(frame: &Frame) -> Option<cairo::ImageSurface> {
    let mut pixels = Vec::with_capacity(frame.rgba.len());
    for p in frame.rgba.as_chunks::<4>().0 {
        pixels.extend_from_slice(&u32::from_be_bytes([p[3], p[0], p[1], p[2]]).to_ne_bytes());
    }
    cairo::ImageSurface::create_for_data(
        pixels,
        cairo::Format::ARgb32,
        frame.width as i32,
        frame.height as i32,
        frame.width as i32 * 4,
    )
    .ok()
}

pub fn paintable(frame: &Frame) -> Option<Thumbnail> {
    let surface = source_surface(frame)?;
    let result: Thumbnail = glib::Object::new();
    result.imp().surface.set(surface).ok()?;
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn enlarged_pixels_keep_crisp_edges_and_premultiplied_alpha() {
        let frame = Frame {
            width: 2,
            height: 1,
            hotspot: (0, 0),
            delay: 0,
            rgba: vec![128, 0, 0, 128, 0, 255, 0, 255].into(),
        };
        let source = source_surface(&frame).unwrap();
        let mut output = cairo::ImageSurface::create(cairo::Format::ARgb32, 8, 4).unwrap();
        {
            let cr = cairo::Context::new(&output).unwrap();
            draw(&cr, &source, 8., 4.);
        }
        let data = output.data().unwrap();
        for y in 0..4 {
            for x in 0..8 {
                let i = (y * 8 + x) * 4;
                let pixel = u32::from_ne_bytes(data[i..i + 4].try_into().unwrap());
                assert_eq!(pixel, if x < 4 { 0x80800000 } else { 0xff00ff00 });
            }
        }
    }
}
