//! Read-only, exact-name file evidence for the system cursor test.
//! This is not proof of what GTK or the compositor actually displays.
use crate::ThemeRepository;
use cursormochi_core::{Error, Resolution, ThemeName};
use std::path::PathBuf;

// First eleven match the existing playground's role indices. Keep Busy and
// Working separate, and test every native edge/corner name independently.
pub const ROLES: &[(&str, &str)] = &[
    ("Normal", "default"),
    ("Link", "pointer"),
    ("Text", "text"),
    ("Busy", "wait"),
    ("Working in background", "progress"),
    ("Horizontal resize", "ew-resize"),
    ("Vertical resize", "ns-resize"),
    ("Move", "move"),
    ("Unavailable", "not-allowed"),
    ("Diagonal ↖ ↘", "nwse-resize"),
    ("Crosshair", "crosshair"),
    ("Diagonal ↗ ↙", "nesw-resize"),
    ("Left edge", "w-resize"),
    ("Right edge", "e-resize"),
    ("Top edge", "n-resize"),
    ("Bottom edge", "s-resize"),
    ("Top left corner", "nw-resize"),
    ("Top right corner", "ne-resize"),
    ("Bottom left corner", "sw-resize"),
    ("Bottom right corner", "se-resize"),
    ("Help", "help"),
    ("Grab", "grab"),
    ("Grabbing", "grabbing"),
    ("Copy", "copy"),
    ("Alias", "alias"),
    ("No drop", "no-drop"),
    ("Context menu", "context-menu"),
    ("Cell", "cell"),
    ("Vertical text", "vertical-text"),
    ("Column resize", "col-resize"),
    ("Row resize", "row-resize"),
    ("All scroll", "all-scroll"),
    ("Zoom in", "zoom-in"),
    ("Zoom out", "zoom-out"),
];
/// Exact file-name coverage for this application's explicit system test set.
/// No inheritance, alias substitution or visual success is inferred here.
#[derive(Debug, PartialEq, Eq)]
pub struct Coverage {
    pub missing: Vec<(&'static str, &'static str)>,
}
pub fn coverage<'a>(names: impl IntoIterator<Item = &'a str>) -> Coverage {
    let names = names.into_iter().collect::<std::collections::BTreeSet<_>>();
    Coverage {
        missing: ROLES
            .iter()
            .copied()
            .filter(|(_, name)| !names.contains(name))
            .collect(),
    }
}
impl Coverage {
    pub fn summary(&self) -> String {
        format!(
            "System files: {}/{} covered · {} missing",
            ROLES.len() - self.missing.len(),
            ROLES.len(),
            self.missing.len()
        )
    }
    pub fn details(&self) -> String {
        let missing = if self.missing.is_empty() {
            "None".to_string()
        } else {
            self.missing
                .iter()
                .map(|(label, name)| format!("{label} ({name})"))
                .collect::<Vec<_>>()
                .join("\n")
        };
        format!(
            "{}\nNot provided by this theme:\n{missing}\nMissing files may use desktop fallback. This checks the 34 names in Test current cursor, not every application or the actual moving pointer.",
            self.summary()
        )
    }
}
#[derive(Debug)]
pub struct Evidence {
    pub source: PathBuf,
    pub chain: Vec<String>,
    pub resolution: Resolution,
    pub frames: usize,
}
pub type Audit = Vec<Result<Evidence, Error>>;

pub fn audit(
    repo: &dyn ThemeRepository,
    theme: &ThemeName,
    cancel: &dyn Fn() -> bool,
) -> Result<Audit, Error> {
    let mut results = Vec::new();
    for (_, name) in ROLES {
        if cancel() {
            return Err(Error::Cancelled);
        }
        let result = repo.preview(theme, name, cancel);
        if matches!(result, Err(Error::Cancelled)) {
            return Err(Error::Cancelled);
        }
        results.push(result.map(|p| Evidence {
            source: p.source,
            chain: p.chain,
            resolution: p.resolution,
            frames: p.variants.iter().map(|v| v.frames.len()).max().unwrap_or(0),
        }));
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Catalog;
    use cursormochi_core::Preview;
    use std::cell::RefCell;
    struct Files(RefCell<Vec<String>>);
    impl ThemeRepository for Files {
        fn scan(&self, _: &dyn Fn() -> bool) -> Result<Catalog, Error> {
            unreachable!()
        }
        fn preview(
            &self,
            t: &ThemeName,
            role: &str,
            _: &dyn Fn() -> bool,
        ) -> Result<Preview, Error> {
            self.0.borrow_mut().push(role.into());
            let resolution = match role {
                "ew-resize" => Resolution::Direct,
                "progress" => Resolution::Inherited,
                "wait" => Resolution::Fallback,
                _ => return Err(Error::Missing),
            };
            Ok(Preview {
                requested: t.clone(),
                role: role.into(),
                source: format!("/parent/cursors/{role}").into(),
                chain: vec!["parent".into()],
                resolution,
                variants: vec![],
            })
        }
    }
    #[test]
    fn generic_resize_does_not_hide_missing_edges_or_corners() {
        let repo = Files(RefCell::new(vec![]));
        let results = audit(&repo, &ThemeName::new("Test").unwrap(), &|| false).unwrap();
        assert!(results[5].is_ok());
        for result in results[12..20].iter().chain(&results[29..31]) {
            assert!(matches!(result, Err(Error::Missing)));
        }
        assert_eq!(
            *repo.0.borrow(),
            ROLES.iter().map(|(_, n)| n.to_string()).collect::<Vec<_>>()
        );
        assert_eq!(
            results[3].as_ref().unwrap().resolution,
            Resolution::Fallback
        );
        assert_eq!(
            results[4].as_ref().unwrap().resolution,
            Resolution::Inherited
        );
        assert_eq!(results[4].as_ref().unwrap().chain, ["parent"]);
    }
    #[test]
    fn coverage_counts_exact_names_without_inventing_aliases_or_fallbacks() {
        let c = coverage(["default", "default", "ew-resize", "wait", "custom"]);
        assert_eq!(c.summary(), "System files: 3/34 covered · 31 missing");
        for name in [
            "col-resize",
            "row-resize",
            "progress",
            "e-resize",
            "w-resize",
        ] {
            assert!(c.missing.iter().any(|(_, n)| *n == name));
            assert!(c.details().contains(name));
        }
        assert!(coverage(ROLES.iter().map(|(_, n)| *n)).missing.is_empty());
        assert_eq!(coverage([]).missing.len(), ROLES.len());
    }
    #[test]
    fn cancelled_audit_does_not_probe_files() {
        let repo = Files(RefCell::new(vec![]));
        assert!(matches!(
            audit(&repo, &ThemeName::new("Test").unwrap(), &|| true),
            Err(Error::Cancelled)
        ));
        assert!(repo.0.borrow().is_empty());
    }
}
