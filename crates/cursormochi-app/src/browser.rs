use cursormochi_core::{Theme, ThemeName};
#[derive(Default, Debug)]
pub struct Browser {
    pub selected: Option<ThemeName>,
    pub query: String,
    pub initialized: bool,
}
impl Browser {
    pub fn reconcile(&mut self, themes: &[Theme], current: Option<&str>) -> Option<String> {
        if !self.initialized {
            self.initialized = true;
            self.selected = current
                .and_then(|n| themes.iter().find(|t| t.name.as_str() == n))
                .or_else(|| themes.first())
                .map(|t| t.name.clone());
            return current.filter(|n|!themes.iter().any(|t|t.name.as_str()==*n)).map(|n|format!("Current theme ‘{n}’ is not in the verified list. Your settings are unchanged."));
        }
        if let Some(selected) = &self.selected
            && !themes.iter().any(|t| &t.name == selected)
        {
            let notice = format!(
                "‘{}’ is no longer available. Select another theme.",
                selected.as_str()
            );
            self.selected = None;
            return Some(notice);
        }
        None
    }
}
pub fn matches(theme: &Theme, query: &str) -> bool {
    let q = query.trim().to_lowercase();
    theme.display.to_lowercase().contains(&q) || theme.name.as_str().to_lowercase().contains(&q)
}
/// Stable UI ordering: themes in a user installation root precede system themes.
/// Use discovered locations, so this also works after restarting the application.
pub fn user_themes_first(themes: &mut [Theme], roots: &[std::path::PathBuf]) {
    themes.sort_by_key(|theme| {
        !theme.locations.iter().any(|location| {
            location
                .directory
                .parent()
                .is_some_and(|parent| roots.iter().any(|root| root == parent))
        })
    });
}
/// Choose by actual source pixels: avoid enlarging a smaller nominal variant
/// when a sufficiently detailed source is available.
pub fn thumbnail_variant(
    preview: &cursormochi_core::Preview,
    pixels: u32,
) -> Option<&cursormochi_core::Variant> {
    preview
        .variants
        .iter()
        .filter(|v| !v.frames.is_empty())
        .min_by_key(|v| {
            let extent = v.frames[0].width.max(v.frames[0].height);
            (extent < pixels, extent.abs_diff(pixels))
        })
}

pub const ROLE_GROUPS: [(&str, &[&str]); 11] = [
    ("Normal", &["left_ptr", "default", "arrow"]),
    ("Link", &["pointer", "hand2", "hand1"]),
    ("Text", &["text", "xterm"]),
    ("Busy", &["watch", "wait"]),
    ("Working in background", &["progress", "left_ptr_watch"]),
    (
        "Resize horizontally",
        &[
            "ew-resize",
            "col-resize",
            "sb_h_double_arrow",
            "h_double_arrow",
            "e-resize",
            "w-resize",
            "right_side",
            "left_side",
        ],
    ),
    (
        "Resize vertically",
        &[
            "ns-resize",
            "row-resize",
            "sb_v_double_arrow",
            "v_double_arrow",
            "n-resize",
            "s-resize",
            "top_side",
            "bottom_side",
        ],
    ),
    ("Move", &["move", "fleur"]),
    ("Unavailable", &["not-allowed", "crossed_circle"]),
    (
        "Resize diagonally",
        &[
            "nwse-resize",
            "bd_double_arrow",
            "size_fdiag",
            "nw-resize",
            "se-resize",
            "top_left_corner",
            "bottom_right_corner",
        ],
    ),
    ("Crosshair", &["crosshair", "cross"]),
];

/// Friendly defaults followed by explicitly named alternatives.
/// Every verified file keeps its own selection ID, even when aliases share a role.
pub fn role_choices(roles: &[String]) -> Vec<(String, String)> {
    let groups = ROLE_GROUPS;
    let mut result = Vec::new();
    let mut mapped = Vec::new();
    for (label, names) in groups {
        for (index, name) in names
            .iter()
            .filter(|n| roles.iter().any(|r| r == **n))
            .enumerate()
        {
            let title = if index == 0 {
                label.to_owned()
            } else {
                format!("{label} · {name}")
            };
            result.push(((*name).to_owned(), title));
        }
        mapped.extend_from_slice(names);
    }
    for role in roles {
        if !mapped.contains(&role.as_str()) {
            result.push((role.clone(), format!("Other · {role}")))
        }
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    use cursormochi_core::Availability;
    fn theme(id: &str) -> Theme {
        Theme {
            name: ThemeName::new(id).unwrap(),
            display: id.into(),
            locations: vec![],
            roles: vec![],
            verified_roles: vec![],
            availability: Availability::NoCursorSource,
            issues: vec![],
        }
    }
    #[test]
    fn user_themes_precede_system_themes_without_losing_selection_or_locations() {
        use cursormochi_core::Location;
        let root = std::path::PathBuf::from("/user/icons");
        let mut themes = vec![
            theme("A-system"),
            theme("B-shared"),
            theme("C-system"),
            theme("Z-imported"),
        ];
        for t in &mut themes {
            t.locations.push(Location {
                directory: std::path::Path::new("/system/icons").join(t.name.as_str()),
                priority: 1,
                inherits: vec![],
            });
        }
        for index in [1, 3] {
            let directory = root.join(themes[index].name.as_str());
            themes[index].locations.push(Location {
                directory,
                priority: 0,
                inherits: vec![],
            });
        }
        // A similarly named directory must not be mistaken for the user root.
        themes[2].locations[0].directory = "/user/icons-other/C-system".into();
        let mut browser = Browser {
            selected: Some(ThemeName::new("C-system").unwrap()),
            initialized: true,
            query: "system".into(),
        };
        user_themes_first(&mut themes, std::slice::from_ref(&root));
        assert_eq!(
            themes.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(),
            ["B-shared", "Z-imported", "A-system", "C-system"]
        );
        assert_eq!(themes[0].locations.len(), 2);
        browser.reconcile(&themes, Some("A-system"));
        assert_eq!(browser.selected.unwrap().as_str(), "C-system");
        assert_eq!(browser.query, "system");
        user_themes_first(&mut themes, &[root]);
        assert_eq!(themes[1].name.as_str(), "Z-imported");
    }
    #[test]
    fn current_and_candidate_stay_independent_across_refresh() {
        let themes = vec![theme("A"), theme("B")];
        let mut b = Browser::default();
        b.reconcile(&themes, Some("B"));
        assert_eq!(b.selected.as_ref().unwrap().as_str(), "B");
        b.selected = Some(ThemeName::new("A").unwrap());
        b.query = "A".into();
        b.reconcile(&themes, Some("B"));
        assert_eq!(b.selected.as_ref().unwrap().as_str(), "A");
        assert_eq!(b.query, "A");
        assert!(b.reconcile(&[theme("B")], Some("B")).is_some());
        assert!(b.selected.is_none());
    }
    #[test]
    fn search_and_missing_current() {
        let mut b = Browser::default();
        assert!(b.reconcile(&[theme("A")], Some("missing")).is_some());
        assert!(matches(&theme("Example"), " EXAM "));
        assert!(!matches(&theme("Example"), "nothing"));
    }
    #[test]
    fn busy_and_background_work_have_distinct_choices() {
        let roles = vec!["watch".into(), "progress".into()];
        let choices = role_choices(&roles);
        assert!(choices.contains(&("watch".into(), "Busy".into())));
        assert!(choices.contains(&("progress".into(), "Working in background".into())));
    }
    #[test]
    fn every_verified_file_remains_selectable_with_unambiguous_labels() {
        let roles: Vec<String> = [
            "left_ptr",
            "default",
            "arrow",
            "pointer",
            "hand2",
            "hand1",
            "text",
            "xterm",
            "watch",
            "wait",
            "progress",
            "left_ptr_watch",
            "ew-resize",
            "col-resize",
            "sb_h_double_arrow",
            "h_double_arrow",
            "ns-resize",
            "row-resize",
            "sb_v_double_arrow",
            "v_double_arrow",
            "move",
            "fleur",
            "not-allowed",
            "crossed_circle",
            "custom-hash",
            "nwse-resize",
            "bd_double_arrow",
            "size_fdiag",
            "crosshair",
            "cross",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        let choices = role_choices(&roles);
        for role in &roles {
            assert_eq!(
                choices.iter().filter(|(id, _)| id == role).count(),
                1,
                "verified role {role} must have exactly one entry"
            );
        }
        assert_eq!(choices.len(), roles.len());
        let labels: std::collections::HashSet<_> = choices.iter().map(|(_, label)| label).collect();
        assert_eq!(labels.len(), choices.len());
        assert!(choices.contains(&("default".into(), "Normal · default".into())));
        assert!(choices.contains(&(
            "left_ptr_watch".into(),
            "Working in background · left_ptr_watch".into()
        )));
        assert_eq!(role_choices(&[]), Vec::<(String, String)>::new());
    }
    #[test]
    fn friendly_roles_use_only_verified_files() {
        let roles = vec!["hand2".into(), "watch".into(), "custom-hash".into()];
        let r = role_choices(&roles);
        assert_eq!(r[0], ("hand2".into(), "Link".into()));
        assert!(r.iter().any(|(_, l)| l == "Busy"));
        assert!(!r.iter().any(|(_, l)| l == "Normal"));
    }
}
