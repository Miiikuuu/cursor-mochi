//! Drag regression using real GTK allocations between synthetic motion samples.
//! This is not hardware-pointer or compositor rendering acceptance.
use super::trial::{Trial, position};
use gtk::{graphene::Point, prelude::*};
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct DragSmoke {
    case: usize,
    sample: usize,
    trace: Option<Trace>,
    at: Option<Instant>,
}
struct Trace {
    card: gtk::Box,
    handle: gtk::Widget,
    gesture: gtk::GestureDrag,
    local: Point,
    press: Point,
    initial: (f64, f64),
    expected: (f64, f64),
    role: usize,
}
const MOTION: &[f32] = &[24., 36., 48., 48., 60., 48., 24., 0.];
impl DragSmoke {
    pub fn tick(&mut self, t: &Trial) -> bool {
        if self
            .at
            .is_some_and(|at| at.elapsed() < Duration::from_millis(70))
        {
            return false;
        }
        if self.case == 12 {
            return true;
        }
        if self.trace.is_none() {
            t.reset.emit_clicked();
            t.snap.set_active(self.case >= 6);
            let card = if self.case % 6 == 2 {
                t.board
                    .last_child()
                    .unwrap()
                    .downcast::<gtk::Box>()
                    .unwrap()
            } else {
                t.card.clone()
            };
            let (handle, role) = match self.case % 6 {
                0 => (t.move_gesture.widget().unwrap(), 7),
                1 | 2 => (card.clone().upcast(), 7),
                3 => (find(&card, "card-resize-x"), 5),
                4 => (find(&card, "card-resize-y"), 6),
                _ => (find(&card, "card-resize-xy"), 9),
            };
            let gesture = handle
                .observe_controllers()
                .iter::<gtk::glib::Object>()
                .filter_map(Result::ok)
                .find_map(|c| c.downcast::<gtk::GestureDrag>().ok())
                .unwrap();
            self.trace = Some(Trace {
                initial: (0., 0.),
                expected: (0., 0.),
                card,
                handle,
                gesture,
                local: Point::new(4., 4.),
                press: Point::new(0., 0.),
                role,
            });
            self.sample = 0;
            self.at = Some(Instant::now());
            return false;
        }
        let trace = self.trace.as_mut().unwrap();
        if self.sample == 0 {
            trace.press = trace.handle.compute_point(&t.board, &trace.local).unwrap();
            trace.initial = if trace.role == 7 {
                position(&t.board, &trace.card)
            } else {
                (
                    trace.card.width_request() as f64,
                    trace.card.height_request() as f64,
                )
            };
            let other = if trace.gesture == t.resize_gesture {
                &t.move_gesture
            } else {
                &t.resize_gesture
            };
            other.emit_by_name::<()>("drag-begin", &[&0f64, &0f64]);
            trace.gesture.emit_by_name::<()>(
                "drag-begin",
                &[&(trace.local.x() as f64), &(trace.local.y() as f64)],
            );
            other.emit_by_name::<()>("cancel", &[&None::<gtk::gdk::EventSequence>]);
            assert!(
                t.dragging(),
                "cancel from another gesture must not clear the active drag"
            );
        } else {
            let actual = if trace.role == 7 {
                position(&t.board, &trace.card)
            } else {
                (
                    trace.card.width_request() as f64,
                    trace.card.height_request() as f64,
                )
            };
            assert!(
                (actual.0 - trace.expected.0).abs() < 0.001
                    && (actual.1 - trace.expected.1).abs() < 0.001,
                "drag case {} sample {}: expected {:?}, got {:?}; card must follow canvas pointer without feedback after allocation",
                self.case,
                self.sample,
                trace.expected,
                actual
            );
        }
        if self.sample == MOTION.len() {
            trace
                .gesture
                .emit_by_name::<()>("drag-end", &[&0f64, &0f64]);
            assert!(!t.dragging());
            let before = (position(&t.board, &trace.card), trace.card.size_request());
            trace
                .gesture
                .emit_by_name::<()>("drag-update", &[&99f64, &99f64]);
            assert_eq!(
                (position(&t.board, &trace.card), trace.card.size_request()),
                before,
                "late motion after end must be ignored"
            );
            self.trace = None;
            self.case += 1;
            self.at = Some(Instant::now());
            if self.case == 12 {
                t.reset.emit_clicked();
                println!(
                    "CANVAS_DRAG_SMOKE PASS: both card bodies, title, right/bottom/corner resize; repeated motion, stationary and reverse samples across GTK allocations; snap on/off. Synthetic input, not physical-pointer visual acceptance."
                );
            }
            return false;
        }
        let dx = MOTION[self.sample];
        let dy = dx / 2.;
        // A canvas-space pointer path independent of the moving/resizing handle.
        let pointer = Point::new(trace.press.x() + dx, trace.press.y() + dy);
        let local = t.board.compute_point(&trace.handle, &pointer).unwrap();
        let offset = (local.x() - trace.local.x(), local.y() - trace.local.y());
        assert!(
            t.dragging(),
            "case {} sample {} lost its drag before motion",
            self.case,
            self.sample
        );
        trace
            .gesture
            .emit_by_name::<()>("drag-update", &[&(offset.0 as f64), &(offset.1 as f64)]);
        let snap = |v: f64| {
            if t.snap.is_active() {
                (v / 12.).round() * 12.
            } else {
                v
            }
        };
        trace.expected = if trace.role == 7 {
            (
                snap(trace.initial.0 + dx as f64).clamp(
                    0.,
                    (t.board.width() - trace.card.width_request()).max(0) as f64,
                ),
                snap(trace.initial.1 + dy as f64).clamp(
                    0.,
                    (t.board.height() - trace.card.height_request()).max(0) as f64,
                ),
            )
        } else {
            let (cx, cy) = position(&t.board, &trace.card);
            (
                if trace.role == 5 || trace.role == 9 {
                    snap(trace.initial.0 + dx as f64)
                        .clamp(120., (t.board.width() as f64 - cx).max(120.))
                } else {
                    trace.initial.0
                },
                if trace.role == 6 || trace.role == 9 {
                    snap(trace.initial.1 + dy as f64)
                        .clamp(80., (t.board.height() as f64 - cy).max(80.))
                } else {
                    trace.initial.1
                },
            )
        };
        self.sample += 1;
        self.at = Some(Instant::now());
        false
    }
}
fn find(root: &impl IsA<gtk::Widget>, class: &str) -> gtk::Widget {
    fn search(root: &gtk::Widget, class: &str) -> Option<gtk::Widget> {
        if root.has_css_class(class) {
            return Some(root.clone());
        }
        let mut child = root.first_child();
        while let Some(w) = child {
            if let Some(found) = search(&w, class) {
                return Some(found);
            }
            child = w.next_sibling();
        }
        None
    }
    search(root.as_ref(), class).expect("drag handle")
}
