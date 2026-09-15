#![forbid(unsafe_code)]
pub mod browser;
use cursormochi_core::{Error, Preview, Theme, ThemeName};
use std::collections::BTreeMap;
pub trait ThemeRepository {
    fn scan(&self, cancel: &dyn Fn() -> bool) -> Result<Catalog, Error>;
    fn preview(
        &self,
        theme: &ThemeName,
        role: &str,
        cancel: &dyn Fn() -> bool,
    ) -> Result<Preview, Error>;
}
#[derive(Clone, Debug, Default)]
pub struct Catalog {
    pub themes: Vec<Theme>,
    pub candidates: Vec<Theme>,
    pub diagnostics: Vec<String>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Key {
    Theme,
    Size,
}
impl Key {
    pub fn name(self) -> &'static str {
        match self {
            Self::Theme => "cursor-theme",
            Self::Size => "cursor-size",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Text(String),
    Int(i32),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Setting {
    pub effective: Value,
    pub user: Option<Value>,
}
pub type Snapshot = BTreeMap<Key, Setting>;
#[derive(Clone, Debug, Default)]
pub struct Capability {
    pub writable: bool,
    pub details: Vec<String>,
}
pub trait DesktopSettingsPort {
    fn capability(&self) -> Capability;
    fn read(&self) -> Result<Snapshot, String>;
    fn validate(&self, key: Key, value: &Value) -> Result<(), String>;
    fn write(&self, key: Key, value: Option<&Value>) -> Result<(), String>;
}
#[derive(Clone, Debug)]
pub struct ChangeIntent {
    pub theme: ThemeName,
    pub size: Option<i32>,
    pub resolved: bool,
}
#[derive(Clone, Debug)]
pub struct Plan {
    pub before: Snapshot,
    pub changes: BTreeMap<Key, Option<Value>>,
}
#[derive(Clone, Debug)]
pub struct Receipt {
    pub before: Snapshot,
    pub after: Snapshot,
    pub keys: Vec<Key>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    Idle,
    Prepared,
    Writing,
    Observed,
    NoOp,
    WriteFailed,
    Conflict,
    VerificationPending,
    Indeterminate,
}
#[derive(Clone, Debug)]
struct Pending {
    plan: Plan,
    expected: Snapshot,
    undo: bool,
    compensating: bool,
}
#[derive(Clone, Debug)]
pub struct OperationReport {
    pub before: Snapshot,
    pub changes: BTreeMap<Key, Option<Value>>,
    pub observed: Option<Snapshot>,
    pub status: Status,
}
#[derive(Default)]
pub struct Controller {
    pub undo: Option<Receipt>,
    pub messages: Vec<String>,
    pub last_operation: Option<OperationReport>,
    pending: Option<Pending>,
}
impl Controller {
    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }
    pub fn prepare(
        &self,
        port: &dyn DesktopSettingsPort,
        intent: ChangeIntent,
    ) -> Result<Plan, String> {
        if self.busy() {
            return Err("A settings change is already in progress".into());
        }
        if !port.capability().writable {
            return Err("This session supports browsing and previewing only".into());
        }
        if !intent.resolved {
            return Err("Load a valid preview of the selected theme first".into());
        }
        let before = port.read()?;
        let mut changes = BTreeMap::new();
        for (key, value) in [
            (Key::Theme, Some(Value::Text(intent.theme.as_str().into()))),
            (Key::Size, intent.size.map(Value::Int)),
        ] {
            if let Some(value) = value {
                port.validate(key, &value)?;
                if before.get(&key).map(|s| &s.effective) != Some(&value) {
                    changes.insert(key, Some(value));
                }
            }
        }
        Ok(Plan { before, changes })
    }
    pub fn begin(&mut self, port: &dyn DesktopSettingsPort, plan: Plan, undo: bool) -> Status {
        if self.busy() {
            return Status::Writing;
        }
        self.last_operation = Some(OperationReport {
            before: plan.before.clone(),
            changes: plan.changes.clone(),
            observed: None,
            status: Status::Prepared,
        });
        let status = self.start_plan(port, plan, undo);
        if let Some(report) = self.last_operation.as_mut() {
            report.status = status.clone();
            report.observed = port.read().ok();
        }
        status
    }
    fn start_plan(&mut self, port: &dyn DesktopSettingsPort, plan: Plan, undo: bool) -> Status {
        if self.busy() {
            self.messages
                .push("An operation is already in progress".into());
            return Status::Writing;
        }
        self.messages.clear();
        if !port.capability().writable {
            self.messages
                .push("Settings are read-only in this session".into());
            return Status::WriteFailed;
        }
        match port.read() {
            Ok(now) if now == plan.before => (),
            Ok(_) => {
                self.messages.push(
                    "Settings changed externally. Review the current values and apply again."
                        .into(),
                );
                return Status::Conflict;
            }
            Err(e) => {
                self.messages.push(e);
                return Status::WriteFailed;
            }
        }
        if plan.changes.is_empty() {
            return Status::NoOp;
        }
        let mut expected = plan.before.clone();
        let mut written = Vec::new();
        for (&key, value) in &plan.changes {
            if let Err(e) = port.write(key, value.as_ref()) {
                self.messages
                    .push(format!("Write to {} failed: {e}", key.name()));
                // Include the failed key if its setter changed state before returning an error.
                if let Ok(now) = port.read()
                    && now.get(&key).and_then(|s| s.user.as_ref()) == value.as_ref()
                {
                    written.push(key);
                }
                return self.compensate(port, &plan, &written, &expected);
            }
            written.push(key);
            if let Some(s) = expected.get_mut(&key) {
                s.user = value.clone();
                if let Some(v) = value {
                    s.effective = v.clone();
                }
            }
        }
        self.pending = Some(Pending {
            plan,
            expected,
            undo,
            compensating: false,
        });
        Status::Writing
    }
    fn compensate(
        &mut self,
        port: &dyn DesktopSettingsPort,
        plan: &Plan,
        keys: &[Key],
        expected: &Snapshot,
    ) -> Status {
        let mut restored = true;
        for key in keys {
            let now = match port.read() {
                Ok(n) => n,
                Err(e) => {
                    self.messages.push(e);
                    restored = false;
                    continue;
                }
            };
            let target = plan.changes.get(key);
            if now.get(key).map(|s| &s.user) != target {
                self.messages.push(format!(
                    "{} changed externally; compensation was skipped",
                    key.name()
                ));
                restored = false;
                continue;
            }
            if let Some(old) = plan.before.get(key)
                && let Err(e) = port.write(*key, old.user.as_ref())
            {
                self.messages
                    .push(format!("Compensation for {} failed: {e}", key.name()));
                restored = false;
            }
        }
        if restored && !keys.is_empty() {
            self.pending = Some(Pending {
                plan: plan.clone(),
                expected: expected.clone(),
                undo: false,
                compensating: true,
            });
            Status::VerificationPending
        } else {
            Status::WriteFailed
        }
    }
    pub fn poll(&mut self, port: &dyn DesktopSettingsPort, expired: bool) -> Status {
        if !self.busy() {
            return Status::Idle;
        }
        let status = self.observe(port, expired);
        if let Some(report) = self.last_operation.as_mut() {
            report.status = status.clone();
        }
        status
    }
    fn observe(&mut self, port: &dyn DesktopSettingsPort, expired: bool) -> Status {
        let Some(p) = self.pending.as_ref() else {
            return Status::Idle;
        };
        let now = match port.read() {
            Ok(n) => n,
            Err(e) => {
                self.messages.push(e);
                if expired {
                    self.pending = None;
                    return Status::Indeterminate;
                }
                return Status::VerificationPending;
            }
        };
        if let Some(report) = self.last_operation.as_mut() {
            report.observed = Some(now.clone());
        }
        let matches = if p.compensating {
            now == p.plan.before
        } else {
            now.iter().all(|(k, s)| match p.plan.changes.get(k) {
                Some(Some(v)) => s.effective == *v && s.user.as_ref() == Some(v),
                Some(None) => s.user.is_none(),
                None => p.expected.get(k) == Some(s),
            }) && now.len() == p.expected.len()
        };
        if matches {
            let Some(p) = self.pending.take() else {
                return Status::Idle;
            };
            if p.compensating {
                self.messages.push(
                    "Compensation restored the original settings; the requested write failed"
                        .into(),
                );
                return Status::WriteFailed;
            }
            if p.undo {
                self.undo = None
            } else {
                self.undo = Some(Receipt {
                    before: p.plan.before,
                    after: now,
                    keys: p.plan.changes.keys().copied().collect(),
                })
            }
            Status::Observed
        } else if expired {
            self.messages
                .push("Verification was inconclusive. Settings may have partially changed; check the current values.".into());
            self.pending = None;
            Status::Indeterminate
        } else {
            Status::VerificationPending
        }
    }
    pub fn begin_undo(&mut self, port: &dyn DesktopSettingsPort) -> Status {
        let Some(r) = &self.undo else {
            return Status::NoOp;
        };
        let changes = r
            .keys
            .iter()
            .filter_map(|k| r.before.get(k).map(|s| (*k, s.user.clone())))
            .collect();
        self.begin(
            port,
            Plan {
                before: r.after.clone(),
                changes,
            },
            true,
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    struct Fake {
        state: RefCell<Snapshot>,
        writes: RefCell<Vec<Key>>,
        fail: Cell<Option<Key>>,
        silent: Cell<bool>,
        fail_reset: Cell<bool>,
        external_on_failure: Cell<bool>,
        read_error: Cell<bool>,
    }
    impl Fake {
        fn new() -> Self {
            Self {
                state: RefCell::new(
                    [
                        (
                            Key::Theme,
                            Setting {
                                effective: Value::Text("A".into()),
                                user: None,
                            },
                        ),
                        (
                            Key::Size,
                            Setting {
                                effective: Value::Int(24),
                                user: None,
                            },
                        ),
                    ]
                    .into(),
                ),
                writes: RefCell::new(vec![]),
                fail: Cell::new(None),
                silent: Cell::new(false),
                fail_reset: Cell::new(false),
                external_on_failure: Cell::new(false),
                read_error: Cell::new(false),
            }
        }
    }
    impl DesktopSettingsPort for Fake {
        fn capability(&self) -> Capability {
            Capability {
                writable: true,
                details: vec![],
            }
        }
        fn read(&self) -> Result<Snapshot, String> {
            if self.read_error.get() {
                return Err("injected read error".into());
            }
            Ok(self.state.borrow().clone())
        }
        fn validate(&self, _: Key, _: &Value) -> Result<(), String> {
            Ok(())
        }
        fn write(&self, k: Key, v: Option<&Value>) -> Result<(), String> {
            self.writes.borrow_mut().push(k);
            if self.fail.get() == Some(k) {
                if self.external_on_failure.get() {
                    self.state.borrow_mut().insert(
                        Key::Theme,
                        Setting {
                            effective: Value::Text("external".into()),
                            user: Some(Value::Text("external".into())),
                        },
                    );
                }
                return Err("injected".into());
            }
            if v.is_none() && self.fail_reset.get() {
                return Err("injected reset failure".into());
            }
            if self.silent.get() {
                return Ok(());
            }
            self.state.borrow_mut().insert(
                k,
                Setting {
                    effective: v.cloned().unwrap_or_else(|| match k {
                        Key::Theme => Value::Text("A".into()),
                        Key::Size => Value::Int(24),
                    }),
                    user: v.cloned(),
                },
            );
            Ok(())
        }
    }
    fn apply(c: &mut Controller, f: &Fake, t: &str) -> Status {
        let p = c
            .prepare(
                f,
                ChangeIntent {
                    theme: ThemeName::new(t).unwrap(),
                    size: None,
                    resolved: true,
                },
            )
            .unwrap();
        let s = c.begin(f, p, false);
        if s == Status::Writing {
            c.poll(f, false)
        } else {
            s
        }
    }
    #[test]
    fn single_undo_and_reset() {
        let f = Fake::new();
        let mut c = Controller::default();
        assert_eq!(apply(&mut c, &f, "B"), Status::Observed);
        assert_eq!(apply(&mut c, &f, "C"), Status::Observed);
        c.begin_undo(&f);
        assert_eq!(c.poll(&f, false), Status::Observed);
        assert_eq!(
            f.read().unwrap()[&Key::Theme].effective,
            Value::Text("B".into())
        );
        assert!(c.undo.is_none());
        assert!(f.writes.borrow().iter().all(|k| *k == Key::Theme));
        let f = Fake::new();
        apply(&mut c, &f, "B");
        c.begin_undo(&f);
        c.poll(&f, false);
        assert!(f.read().unwrap()[&Key::Theme].user.is_none());
    }
    #[test]
    fn noop_preserves_history_and_external_conflicts() {
        let f = Fake::new();
        let mut c = Controller::default();
        apply(&mut c, &f, "B");
        assert_eq!(apply(&mut c, &f, "B"), Status::NoOp);
        assert!(c.undo.is_some());
        f.write(Key::Size, Some(&Value::Int(32))).unwrap();
        assert_eq!(c.begin_undo(&f), Status::Conflict);
    }
    #[test]
    fn plan_conflict() {
        let f = Fake::new();
        let mut c = Controller::default();
        let p = c
            .prepare(
                &f,
                ChangeIntent {
                    theme: ThemeName::new("B").unwrap(),
                    size: None,
                    resolved: true,
                },
            )
            .unwrap();
        f.write(Key::Theme, Some(&Value::Text("D".into()))).unwrap();
        assert_eq!(c.begin(&f, p, false), Status::Conflict);
    }
    #[test]
    fn partial_failure_compensates() {
        let f = Fake::new();
        f.fail.set(Some(Key::Size));
        let mut c = Controller::default();
        let p = c
            .prepare(
                &f,
                ChangeIntent {
                    theme: ThemeName::new("B").unwrap(),
                    size: Some(32),
                    resolved: true,
                },
            )
            .unwrap();
        assert_eq!(c.begin(&f, p, false), Status::VerificationPending);
        assert_eq!(c.poll(&f, false), Status::WriteFailed);
        assert!(f.read().unwrap()[&Key::Theme].user.is_none());
        assert!(c.undo.is_none());
    }
    #[test]
    fn delayed_and_serialized() {
        let f = Fake::new();
        f.silent.set(true);
        let mut c = Controller::default();
        assert_eq!(apply(&mut c, &f, "B"), Status::VerificationPending);
        assert!(c.busy());
        assert_eq!(c.poll(&f, true), Status::Indeterminate);
        assert!(c.undo.is_none());
    }
    #[test]
    fn setter_failure_and_failed_compensation_visible() {
        let f = Fake::new();
        let mut c = Controller::default();
        f.fail.set(Some(Key::Theme));
        assert_eq!(apply(&mut c, &f, "B"), Status::WriteFailed);
        assert!(c.undo.is_none());
        f.fail.set(Some(Key::Size));
        f.fail_reset.set(true);
        let p = c
            .prepare(
                &f,
                ChangeIntent {
                    theme: ThemeName::new("B").unwrap(),
                    size: Some(32),
                    resolved: true,
                },
            )
            .unwrap();
        assert_eq!(c.begin(&f, p, false), Status::WriteFailed);
        assert!(c.messages.iter().any(|s| s.contains("Write")));
        assert!(c.messages.iter().any(|s| s.contains("Compensation")));
        assert_eq!(
            f.read().unwrap()[&Key::Theme].effective,
            Value::Text("B".into())
        );
    }
    #[test]
    fn external_change_during_partial_failure_is_not_overwritten() {
        let f = Fake::new();
        let mut c = Controller::default();
        f.fail.set(Some(Key::Size));
        f.external_on_failure.set(true);
        let p = c
            .prepare(
                &f,
                ChangeIntent {
                    theme: ThemeName::new("B").unwrap(),
                    size: Some(32),
                    resolved: true,
                },
            )
            .unwrap();
        assert_eq!(c.begin(&f, p, false), Status::WriteFailed);
        assert_eq!(
            f.read().unwrap()[&Key::Theme].effective,
            Value::Text("external".into())
        );
    }
    #[test]
    fn delayed_observation_and_read_failure() {
        let f = Fake::new();
        let mut c = Controller::default();
        f.silent.set(true);
        assert_eq!(apply(&mut c, &f, "B"), Status::VerificationPending);
        f.silent.set(false);
        f.write(Key::Theme, Some(&Value::Text("B".into()))).unwrap();
        assert_eq!(c.poll(&f, false), Status::Observed);
        f.silent.set(true);
        assert_eq!(apply(&mut c, &f, "C"), Status::VerificationPending);
        f.read_error.set(true);
        assert_eq!(c.poll(&f, true), Status::Indeterminate);
    }
    #[test]
    fn read_only_work_has_no_writes_and_no_restart_history() {
        let f = Fake::new();
        let c = Controller::default();
        f.read().unwrap();
        f.capability();
        c.prepare(
            &f,
            ChangeIntent {
                theme: ThemeName::new("B").unwrap(),
                size: None,
                resolved: true,
            },
        )
        .unwrap();
        assert!(f.writes.borrow().is_empty());
        assert!(c.undo.is_none());
    }
}
