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
/// Friendly labels with explicit file aliases; input contains successfully decoded roles only.
pub fn role_choices(roles: &[String]) -> Vec<(String, String)> {
    let groups: [(&str, &[&str]); 8] = [
        ("Normal", &["left_ptr", "default", "arrow"]),
        ("Link", &["pointer", "hand2", "hand1"]),
        ("Text", &["text", "xterm"]),
        ("Busy", &["watch", "wait", "progress", "left_ptr_watch"]),
        (
            "Resize horizontally",
            &["ew-resize", "sb_h_double_arrow", "h_double_arrow"],
        ),
        (
            "Resize vertically",
            &["ns-resize", "sb_v_double_arrow", "v_double_arrow"],
        ),
        ("Move", &["move", "fleur"]),
        ("Unavailable", &["not-allowed", "crossed_circle"]),
    ];
    let mut result = Vec::new();
    let mut mapped = Vec::new();
    for (label, names) in groups {
        if let Some(name) = names.iter().find(|n| roles.iter().any(|r| r == **n)) {
            result.push(((*name).to_string(), label.to_owned()));
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
    fn friendly_roles_use_only_verified_files() {
        let roles = vec!["hand2".into(), "watch".into(), "custom-hash".into()];
        let r = role_choices(&roles);
        assert_eq!(r[0], ("hand2".into(), "Link".into()));
        assert!(r.iter().any(|(_, l)| l == "Busy"));
        assert!(!r.iter().any(|(_, l)| l == "Normal"));
    }
}
