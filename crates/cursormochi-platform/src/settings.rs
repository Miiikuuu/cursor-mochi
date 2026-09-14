use super::Environment;
use cursormochi_app::*;
use gio::prelude::*;
pub struct GnomeSettings {
    writer: gio::Settings,
    reader: gio::Settings,
    schema: gio::SettingsSchema,
    allowed: bool,
    details: Vec<String>,
}
impl GnomeSettings {
    pub fn host(env: &Environment) -> Result<Self, String> {
        let source =
            gio::SettingsSchemaSource::default().ok_or("GSettings schema source missing")?;
        let schema = source
            .lookup("org.gnome.desktop.interface", true)
            .ok_or("GNOME schema missing")?;
        let desktop = env.text("XDG_CURRENT_DESKTOP");
        let session = desktop.split(':').any(|s| s.eq_ignore_ascii_case("gnome"));
        let restricted = env.vars.contains_key("FLATPAK_ID") || env.vars.contains_key("SNAP");
        Self::new(
            schema,
            None,
            session && !restricted,
            vec![
                format!(
                    "desktop={desktop}; session={}; GNOME={session}; sandbox={restricted}",
                    env.text("XDG_SESSION_TYPE")
                ),
                "Host theme lookup paths are unverified. Settings readback does not confirm the pointer appearance in every application.".into(),
            ],
        )
    }
    pub fn injected(
        schema: gio::SettingsSchema,
        backend: &gio::SettingsBackend,
        writable: bool,
    ) -> Result<Self, String> {
        Self::new(
            schema,
            Some(backend),
            writable,
            vec!["explicit injected backend".into()],
        )
    }
    fn new(
        schema: gio::SettingsSchema,
        backend: Option<&gio::SettingsBackend>,
        allowed: bool,
        details: Vec<String>,
    ) -> Result<Self, String> {
        for (key, ty) in [(Key::Theme, "s"), (Key::Size, "i")] {
            if !schema.has_key(key.name()) {
                return Err(format!("{} missing", key.name()));
            }
            if schema.key(key.name()).value_type().as_str() != ty {
                return Err(format!("{} wrong type", key.name()));
            }
        }
        let writer = gio::Settings::new_full(&schema, backend, None);
        let reader = gio::Settings::new_full(&schema, backend, None);
        Ok(Self {
            writer,
            reader,
            schema,
            allowed,
            details,
        })
    }
    pub fn connect_changed(&self, f: impl Fn() + 'static) {
        self.reader.connect_changed(None, move |_, _| f());
    }
}
fn value(v: glib::Variant) -> Result<Value, String> {
    if let Some(s) = v.get::<String>() {
        Ok(Value::Text(s))
    } else if let Some(i) = v.get::<i32>() {
        Ok(Value::Int(i))
    } else {
        Err("unsupported settings value".into())
    }
}
fn variant(v: &Value) -> glib::Variant {
    match v {
        Value::Text(s) => s.to_variant(),
        Value::Int(n) => n.to_variant(),
    }
}
impl DesktopSettingsPort for GnomeSettings {
    fn capability(&self) -> Capability {
        let mut details = self.details.clone();
        let mut writable = self.allowed;
        for key in [Key::Theme, Key::Size] {
            let w = self.writer.is_writable(key.name());
            details.push(format!(
                "{}: readable=true writable={w} type={} range={}",
                key.name(),
                self.schema.key(key.name()).value_type(),
                self.schema.key(key.name()).range()
            ));
            writable &= w;
        }
        Capability { writable, details }
    }
    fn read(&self) -> Result<Snapshot, String> {
        let mut s = Snapshot::new();
        for key in [Key::Theme, Key::Size] {
            s.insert(
                key,
                Setting {
                    effective: value(self.reader.value(key.name()))?,
                    user: self.reader.user_value(key.name()).map(value).transpose()?,
                },
            );
        }
        Ok(s)
    }
    fn validate(&self, k: Key, v: &Value) -> Result<(), String> {
        let v = variant(v);
        let schema = self.schema.key(k.name());
        if v.type_() != schema.value_type() || !schema.range_check(&v) {
            return Err(format!("{} outside schema type/range", k.name()));
        }
        Ok(())
    }
    fn write(&self, k: Key, v: Option<&Value>) -> Result<(), String> {
        if !self.allowed || !self.writer.is_writable(k.name()) {
            return Err(format!("{} not writable", k.name()));
        }
        if let Some(v) = v {
            self.validate(k, v)?;
            self.writer
                .set_value(k.name(), &variant(v))
                .map_err(|e| format!("{}: {e}", k.name()))?
        } else {
            self.writer.reset(k.name())
        }
        Ok(())
    }
}
pub struct ReadOnlySettings(pub String);
impl DesktopSettingsPort for ReadOnlySettings {
    fn capability(&self) -> Capability {
        Capability {
            writable: false,
            details: vec![self.0.clone()],
        }
    }
    fn read(&self) -> Result<Snapshot, String> {
        Err(self.0.clone())
    }
    fn validate(&self, _: Key, _: &Value) -> Result<(), String> {
        Err(self.0.clone())
    }
    fn write(&self, _: Key, _: Option<&Value>) -> Result<(), String> {
        Err(self.0.clone())
    }
}
