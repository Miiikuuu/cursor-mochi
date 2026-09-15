use super::*;
use std::collections::{BTreeSet, HashMap};
const PER_THEME: usize = 64;
const PER_SCAN: usize = 2048;
impl Repository {
    /// Classify candidates with the same per-role loader, explicitly excluding default fallback.
    pub(super) fn classify(
        &self,
        mut themes: Vec<Theme>,
        mut diagnostics: Vec<String>,
        cancel: &dyn Fn() -> bool,
    ) -> Result<Catalog, Error> {
        let records: HashMap<_, _> = themes
            .iter()
            .map(|t| (t.name.as_str().to_owned(), t))
            .collect();
        let mut role_sets = Vec::new();
        let mut role_budget = 65536usize;
        for theme in &themes {
            if cancel() {
                return Err(Error::Cancelled);
            }
            if role_budget == 0 {
                role_sets.push((vec![], vec![], true));
                continue;
            }
            let mut roles = BTreeSet::new();
            let mut stack = Vec::new();
            let mut issues = Vec::new();
            if let Availability::Invalid(e) = &theme.availability {
                issues.push(e.clone());
            }
            let mut visits = 0;
            let traversal = self.collect_roles(
                theme.name.as_str(),
                &records,
                &mut roles,
                &mut stack,
                &mut visits,
                &mut issues,
                cancel,
            );
            let traversal_limited = match traversal {
                Ok(()) => false,
                Err(Error::Cancelled) => return Err(Error::Cancelled),
                Err(e) => {
                    issues.push(e.to_string());
                    true
                }
            };
            let mut ordered = Vec::new();
            for name in [
                "left_ptr",
                "default",
                "pointer",
                "hand2",
                "text",
                "xterm",
                "watch",
                "wait",
                "sb_h_double_arrow",
                "sb_v_double_arrow",
                "ew-resize",
                "ns-resize",
                "nwse-resize",
                "nesw-resize",
                "h_double_arrow",
                "v_double_arrow",
                "fleur",
                "move",
                "not-allowed",
                "crossed_circle",
            ] {
                if roles.remove(name) {
                    ordered.push(name.to_owned());
                }
            }
            // Prefer customary extensionless role filenames; extensions are not a blacklist.
            let mut remaining: Vec<_> = roles.into_iter().collect();
            remaining.sort_by_key(|name| (name.contains('.'), name.clone()));
            ordered.extend(remaining);
            let limited =
                traversal_limited || ordered.len() > PER_THEME || ordered.len() > role_budget;
            ordered.truncate((PER_THEME + 1).min(role_budget));
            role_budget -= ordered.len();
            issues.sort();
            issues.dedup();
            issues.truncate(64);
            role_sets.push((ordered, issues, limited));
        }
        let mut attempts = 0;
        for (theme, (roles, mut issues, mut limited)) in themes.iter_mut().zip(role_sets) {
            let mut evidence = None;
            for role in roles.iter().take(PER_THEME) {
                if cancel() {
                    return Err(Error::Cancelled);
                }
                if attempts >= PER_SCAN {
                    limited = true;
                    break;
                }
                attempts += 1;
                match self.find(&theme.name, role, &mut vec![], cancel, &mut 0) {
                    Ok(p) => {
                        theme.verified_roles.push(role.clone());
                        let next = match p.resolution {
                            Resolution::Direct => Availability::Direct {
                                role: role.clone(),
                                source: p.source,
                            },
                            _ => Availability::Inherited {
                                role: role.clone(),
                                source: p.source,
                                chain: p.chain,
                            },
                        };
                        if evidence.is_none()
                            || matches!(evidence, Some(Availability::Inherited { .. }))
                                && matches!(next, Availability::Direct { .. })
                        {
                            evidence = Some(next)
                        }
                    }
                    Err(Error::Cancelled) => return Err(Error::Cancelled),
                    Err(Error::Missing) => (),
                    Err(e) => issues.push(format!("{role}: {e}")),
                }
            }
            if roles.len() > PER_THEME {
                limited = true
            }
            // Keep original filenames; only add the bounded inherited inspection set.
            theme.roles.extend(roles);
            theme.roles.sort();
            theme.roles.dedup();
            if limited {
                issues.push("Verification budget reached; remaining roles were not checked".into())
            }
            issues.sort();
            issues.dedup();
            issues.truncate(64);
            theme.issues = issues.clone();
            let outside_effective_paths = !theme.locations.iter().any(|l| {
                self.paths
                    .resolution
                    .iter()
                    .any(|r| l.directory == r.join(theme.name.as_str()))
            });
            theme.availability = evidence.unwrap_or_else(|| {
                if outside_effective_paths {
                    return Availability::Unverified(
                        "Discovered outside the effective lookup paths".into(),
                    );
                }
                if limited {
                    Availability::Unverified("Verification budget reached".into())
                } else if !issues.is_empty() {
                    Availability::Invalid(issues.join("; "))
                } else {
                    Availability::NoCursorSource
                }
            });
            if !theme.availability.usable() {
                diagnostics.push(format!(
                    "{}: {}. Automatic default fallback is not theme evidence.",
                    theme.name.as_str(),
                    theme.availability.label()
                ));
            }
            diagnostics.extend(
                issues
                    .into_iter()
                    .take(64)
                    .map(|issue| format!("{}: {issue}", theme.name.as_str())),
            );
        }
        let (themes, candidates) = themes.into_iter().partition(|t| t.availability.usable());
        Ok(Catalog {
            themes,
            candidates,
            diagnostics,
        })
    }
    #[allow(clippy::too_many_arguments)]
    fn collect_roles(
        &self,
        name: &str,
        records: &HashMap<String, &Theme>,
        roles: &mut BTreeSet<String>,
        stack: &mut Vec<String>,
        visits: &mut usize,
        issues: &mut Vec<String>,
        cancel: &dyn Fn() -> bool,
    ) -> Result<(), Error> {
        if cancel() {
            return Err(Error::Cancelled);
        }
        *visits += 1;
        if *visits > 4096 || stack.len() >= 32 {
            return Err(Error::LimitExceeded("inheritance traversal"));
        }
        if stack.iter().any(|s| s == name) {
            issues.push(format!("Inheritance cycle: {} → {name}", stack.join(" → ")));
            return Ok(());
        }
        let Some(theme) = records.get(name) else {
            issues.push(format!("Missing parent: {name}"));
            return Ok(());
        };
        roles.extend(theme.roles.iter().take(8192).cloned());
        if roles.len() > 8192 {
            return Err(Error::LimitExceeded("inherited role names"));
        }
        stack.push(name.into());
        // Match find(): first nonempty inheritance list in the effective roots.
        let mut parents = Vec::new();
        for root in &self.paths.resolution {
            if let Ok((_, inherits)) = self.index(&root.join(name))
                && !inherits.is_empty()
            {
                parents = inherits;
                break;
            }
        }
        for parent in parents {
            self.collect_roles(
                parent.as_str(),
                records,
                roles,
                stack,
                visits,
                issues,
                cancel,
            )?;
        }
        stack.pop();
        Ok(())
    }
}
