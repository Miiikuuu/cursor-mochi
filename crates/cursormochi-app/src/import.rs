//! Import planning has no desktop settings capability. Mappings always require review.
use crate::browser::ROLE_GROUPS;
use cursormochi_core::{Error, Preview, Resolution, ThemeName, windows::Decoded};
use std::{collections::BTreeSet, path::PathBuf};
/// Windows' common role set; the first eleven reuse the existing browser/trial roles.
pub const IMPORT_ROLES: [(&str, &[&str]); 15] = [
    ROLE_GROUPS[0],
    ROLE_GROUPS[1],
    ROLE_GROUPS[2],
    ROLE_GROUPS[3],
    ROLE_GROUPS[4],
    ROLE_GROUPS[5],
    ROLE_GROUPS[6],
    ROLE_GROUPS[7],
    ROLE_GROUPS[8],
    ROLE_GROUPS[9],
    ROLE_GROUPS[10],
    ("Help", &["help", "question_arrow", "whats_this"]),
    ("Handwriting", &["pencil", "draft_small", "draft_large"]),
    (
        "Resize other diagonal",
        &[
            "nesw-resize",
            "fd_double_arrow",
            "size_bdiag",
            "ne-resize",
            "sw-resize",
            "top_right_corner",
            "bottom_left_corner",
        ],
    ),
    ("Alternate select (up arrow)", &["up-arrow", "center_ptr"]),
];
#[derive(Clone, Debug)]
pub struct Asset {
    pub path: PathBuf,
    pub decoded: Result<Decoded, Error>,
}
#[derive(Clone, Debug, Default)]
pub struct Package {
    pub assets: Vec<Asset>,
    pub ignored: usize,
}
impl Package {
    pub fn roots(&self) -> Vec<PathBuf> {
        self.assets
            .iter()
            .flat_map(|a| {
                a.path
                    .parent()
                    .into_iter()
                    .flat_map(std::path::Path::ancestors)
            })
            .filter(|p| {
                !p.as_os_str().is_empty()
                    || self
                        .assets
                        .iter()
                        .any(|a| a.path.parent().is_some_and(|p| p.as_os_str().is_empty()))
            })
            .map(PathBuf::from)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
}
#[derive(Clone, Debug)]
pub struct Plan {
    pub name: ThemeName,
    pub mapped: Vec<(String, Asset)>,
    pub missing: Vec<&'static str>,
    pub notes: Vec<String>,
}
impl Plan {
    pub fn system_coverage(&self) -> crate::current_cursor::Coverage {
        crate::current_cursor::coverage(
            self.mapped
                .iter()
                .filter_map(|(role, _)| IMPORT_ROLES.iter().find(|(_, names)| names[0] == role))
                .flat_map(|(_, names)| names.iter().copied()),
        )
    }
}
pub fn suggestion(path: &std::path::Path) -> Option<usize> {
    let stem = path.file_stem()?.to_str()?.to_lowercase();
    let extra = [
        ("normal", 0),
        ("normal select", 0),
        ("text select", 2),
        ("work", 4),
        ("horizontal resize", 5),
        ("vertical resize", 6),
        // Windows' documented labels: 1 = SIZENWSE, 2 = SIZENESW.
        // Still only filename suggestions; users can inspect and change them.
        ("diagonal resize 1", 9),
        ("diagonal resize 2", 13),
        ("precision select", 10),
        ("help select", 11),
        ("handwriting", 12),
        ("alternate select", 14),
        ("arrow", 0),
        ("link", 1),
        ("hand", 1),
        ("ibeam", 2),
        ("busy", 3),
        ("working", 4),
        ("background", 4),
        ("unavailable", 8),
        ("precision", 10),
        ("appstarting", 4),
        ("help", 11),
        ("nwpen", 12),
        ("sizenesw", 13),
        ("uparrow", 14),
    ];
    IMPORT_ROLES
        .iter()
        .position(|(_, aliases)| aliases.contains(&stem.as_str()))
        .or_else(|| extra.iter().find(|(n, _)| *n == stem).map(|(_, i)| *i))
}
pub fn plan(name: &str, assignments: &[(usize, Asset)], confirmed: bool) -> Result<Plan, Error> {
    if !confirmed {
        return Err(Error::Invalid(
            "confirm root, mappings, missing roles and conversion differences",
        ));
    }
    // A portable basename also prevents index.theme line/section injection.
    if name.is_empty()
        || name.len() > 80
        || name.starts_with('.')
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
    {
        return Err(Error::Invalid(
            "theme name: 1–80 ASCII letters, digits, dash, underscore or dot; no leading dot",
        ));
    }
    let mut seen = BTreeSet::new();
    let mut mapped = Vec::new();
    let mut notes = vec!["Desktop sizes: 16, 24, 32, 48, 64, 96 and 128 px plus original sizes. Missing sizes are area-resampled in premultiplied RGBA; aspect ratio and hotspot positions are scaled with nearest-pixel rounding, with hotspots clamped inside the image. Resampling changes pixels; original-size variants stay unchanged. Frame order and delays are preserved. Other requested sizes use the loader’s nearest available size; visual size across every toolkit is not guaranteed.".into()];
    let mut bytes = 0;
    for (index, asset) in assignments {
        let (_, aliases) = IMPORT_ROLES
            .get(*index)
            .ok_or(Error::Invalid("unknown role"))?;
        if !seen.insert(*index) {
            return Err(Error::Invalid("multiple files mapped to one role"));
        }
        let data = asset.decoded.as_ref().map_err(Clone::clone)?;
        bytes += data
            .variants
            .iter()
            .flat_map(|v| &v.frames)
            .map(|f| f.rgba.len())
            .sum::<usize>();
        if bytes > 64 * 1024 * 1024 {
            return Err(Error::LimitExceeded("theme decoded pixels"));
        }
        notes.extend(
            data.notes
                .iter()
                .map(|n| format!("{}: {n}", asset.path.display())),
        );
        mapped.push((aliases[0].to_string(), asset.clone()));
        if [5, 6, 9, 13].contains(index) {
            notes.push(format!("{}: window-edge/corner names reuse the confirmed resize image on the same axis; pixels and hotspots are not rotated or mirrored.", IMPORT_ROLES[*index].0));
        }
        if [5, 6].contains(index) {
            let pane = if *index == 5 {
                "col-resize"
            } else {
                "row-resize"
            };
            notes.push(format!("{}: pane divider ({pane}) reuses the confirmed resize image on the same axis; no separate divider artwork was supplied.", IMPORT_ROLES[*index].0));
        }
    }
    if mapped.is_empty() {
        return Err(Error::Missing);
    }
    let missing = IMPORT_ROLES
        .iter()
        .enumerate()
        .filter(|(i, _)| !seen.contains(i))
        .map(|(_, g)| g.0)
        .collect();
    Ok(Plan {
        name: ThemeName::new(name)?,
        mapped,
        missing,
        notes,
    })
}
pub fn preview(asset: &Asset, role: &str) -> Result<Preview, Error> {
    Ok(Preview {
        requested: ThemeName::new("Uninstalled-preview")?,
        role: role.into(),
        source: asset.path.clone(),
        chain: vec!["Uninstalled import · direct file; no inheritance".into()],
        resolution: Resolution::Direct,
        variants: asset
            .decoded
            .as_ref()
            .map_err(Clone::clone)?
            .variants
            .clone(),
    })
}
/// Reuse trial preparation on uninstalled decoded assets; missing means missing.
pub fn trial(
    assignments: &[(usize, Asset)],
    size: u32,
    cancel: &dyn Fn() -> bool,
) -> Result<crate::trial::TrialSet, Error> {
    let mut bytes = 0;
    let mut roles = Vec::new();
    let mut thumbnails = Vec::new();
    let mut thumbnail_bytes = 0;
    for (index, (label, aliases)) in ROLE_GROUPS.iter().enumerate() {
        if cancel() {
            return Err(Error::Cancelled);
        }
        let p = assignments
            .iter()
            .find(|(i, _)| *i == index)
            .ok_or(Error::Missing)
            .and_then(|(_, a)| preview(a, aliases[0]));
        thumbnails.push(
            p.as_ref()
                .map_err(Clone::clone)
                .and_then(|p| crate::trial::source_thumbnail(p, &mut thumbnail_bytes)),
        );
        let p = p
            .and_then(|p| crate::trial::prepare(p, size, cancel))
            .and_then(|p| {
                bytes += p.variants[0]
                    .frames
                    .iter()
                    .map(|f| f.rgba.len())
                    .sum::<usize>();
                if bytes > 16 * 1024 * 1024 {
                    Err(Error::LimitExceeded("trial pixels"))
                } else {
                    Ok(p)
                }
            });
        roles.push((*label, p));
    }
    Ok(crate::trial::TrialSet {
        theme: ThemeName::new("Uninstalled-preview")?,
        size,
        thumbnails,
        roles,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn asset() -> Asset {
        Asset {
            path: PathBuf::from("busy.ani"),
            decoded: cursormochi_core::windows::decode(
                include_bytes!("../../../tests/import-fixtures/busy.ani"),
                &|| false,
            ),
        }
    }
    #[test]
    fn mapping_is_explicit_busy_is_distinct_and_missing_is_visible() {
        assert_eq!(suggestion(std::path::Path::new("busy.ani")), Some(3));
        assert_eq!(suggestion(std::path::Path::new("working.ani")), Some(4));
        assert!(plan("Demo", &[(3, asset())], false).is_err());
        let p = plan("Demo", &[(3, asset())], true).unwrap();
        assert_eq!(p.mapped[0].0, "watch");
        assert!(p.missing.contains(&"Working in background"));
        assert!(plan("Demo", &[(3, asset()), (3, asset())], true).is_err());
        let all = (0..IMPORT_ROLES.len())
            .map(|i| (i, asset()))
            .collect::<Vec<_>>();
        let p = plan("All-Roles", &all, true).unwrap();
        assert!(p.missing.is_empty());
        assert_eq!(p.mapped.len(), 15);
        let names = IMPORT_ROLES
            .iter()
            .flat_map(|(_, names)| names.iter())
            .collect::<Vec<_>>();
        assert_eq!(
            names.len(),
            names
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
        );
        let nested = Package {
            assets: ["One/Sub/a.cur", "One/Other/b.cur", "Two/a.cur"]
                .into_iter()
                .map(|p| {
                    let mut a = asset();
                    a.path = PathBuf::from(p);
                    a
                })
                .collect(),
            ignored: 0,
        };
        assert_eq!(
            nested.roots(),
            ["One", "One/Other", "One/Sub", "Two"].map(PathBuf::from)
        );
        for name in ["../escape", "A\nInherits=evil", ".hidden", ""] {
            assert!(plan(name, &[(3, asset())], true).is_err());
        }
    }
    #[test]
    fn uninstalled_trial_preserves_animation_and_has_no_ambient_fallback() {
        let t = trial(&[(3, asset())], 16, &|| false).unwrap();
        assert!(t.roles[0].1.is_err());
        assert_eq!(t.roles[3].1.as_ref().unwrap().variants[0].frames.len(), 3);
    }
    #[test]
    fn import_layers_cannot_call_settings() {
        // Architecture tripwire complements tests using no settings object at all.
        let identifier = |code: &str, name: &str| {
            code.split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                .any(|word| word == name)
        };
        // GTK event controllers are unrelated to app::Controller/settings.
        assert!(identifier("use app::Controller;", "Controller"));
        assert!(!identifier("gtk::EventControllerScroll", "Controller"));
        for source in [
            include_str!("import.rs"),
            include_str!("../../cursormochi-platform/src/import.rs"),
            include_str!("../../cursormochi-gtk/src/import.rs"),
        ] {
            let code = source.split("#[cfg(test)]").next().unwrap();
            for forbidden in [
                "DesktopSettingsPort",
                "GnomeSettings",
                "write_theme(",
                "write_size(",
                "Controller",
                ".set_string(",
                ".set_int(",
            ] {
                let present = if forbidden.contains('(') {
                    code.contains(forbidden)
                } else {
                    identifier(code, forbidden)
                };
                assert!(!present, "{forbidden}");
            }
        }
    }
}

#[cfg(test)]
mod windows_names {
    use super::*;
    #[test]
    fn common_windows_labels_are_only_unambiguous_suggestions() {
        for (name, role) in [
            ("Normal Select", 0),
            ("Text Select", 2),
            ("Busy", 3),
            ("Work", 4),
            ("Horizontal Resize", 5),
            ("Vertical Resize", 6),
            ("Diagonal Resize 1", 9),
            ("Diagonal Resize 2", 13),
            ("Precision Select", 10),
            ("Help Select", 11),
            ("handwriting", 12),
            ("Alternate Select", 14),
        ] {
            assert_eq!(
                suggestion(&PathBuf::from(format!("{name}.ani"))),
                Some(role)
            );
        }
        for name in [
            "Diagonal Resize.ani",
            "Diagonal Resize 3.ani",
            "Person Select.ani",
            "主鼠标 替换.ani",
        ] {
            assert_eq!(suggestion(std::path::Path::new(name)), None);
        }
    }
}
