//! Maintenance tool: create a new desktop-sized theme from existing confirmed
//! import mappings. No guessing from Windows filenames, overwrite or Apply.
use cursormochi_app::{
    ThemeRepository,
    import::{Asset, IMPORT_ROLES, plan},
};
use cursormochi_core::{Error, Resolution, ThemeName, windows::Decoded};
use cursormochi_platform::{Paths, Repository, import::install};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let source =
        PathBuf::from(args.next().ok_or("source theme directory required")?).canonicalize()?;
    let output = PathBuf::from(args.next().ok_or("confirmed output root required")?);
    let name = args.next().ok_or("new theme name required")?;
    let name = name.to_str().ok_or("UTF-8 name required")?;
    if args.next().is_some() {
        return Err("unexpected argument".into());
    }
    let root = source
        .parent()
        .ok_or("source parent required")?
        .to_path_buf();
    let theme = ThemeName::new(
        source
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or("source theme name")?,
    )?;
    let repo = Repository::new(Paths {
        discovery: vec![root.clone()],
        resolution: vec![root],
        diagnostics: vec![],
    });
    // This maintenance tool is only for the application's generated imports.
    // Refuse unfamiliar roles rather than silently discarding user assets.
    let allowed = IMPORT_ROLES
        .iter()
        .flat_map(|(_, names)| names.iter().copied())
        .collect::<std::collections::BTreeSet<_>>();
    for entry in std::fs::read_dir(source.join("cursors"))?.take(1025) {
        let entry = entry?;
        if !entry
            .file_name()
            .to_str()
            .is_some_and(|n| allowed.contains(n))
        {
            return Err("unrecognized source role; explicit mapping required".into());
        }
    }
    let mut assignments = Vec::new();
    for (role, (_, names)) in IMPORT_ROLES.iter().enumerate() {
        match repo.preview(&theme, names[0], &|| false) {
            Ok(p)
                if p.resolution == Resolution::Direct
                    && p.source.parent() == Some(source.join("cursors").as_path()) =>
            {
                // Confirm every existing alias really is the same asset before reuse.
                for alias in &names[1..] {
                    if !source.join("cursors").join(alias).exists() {
                        continue;
                    }
                    let a = repo.preview(&theme, alias, &|| false)?;
                    if a.resolution != Resolution::Direct
                        || !cursormochi_core::export::equivalent(&p.variants, &a.variants)
                    {
                        return Err("different alias artwork; explicit mapping required".into());
                    }
                }
                assignments.push((
                    role,
                    Asset {
                        path: p.source,
                        decoded: Ok(Decoded {
                            variants: p.variants,
                            notes: vec![
                                "Reused the existing theme's confirmed role mapping.".into(),
                            ],
                        }),
                    },
                ));
            }
            Err(Error::Missing) => {
                if names
                    .iter()
                    .any(|n| source.join("cursors").join(n).exists())
                {
                    return Err("alias without canonical role; explicit mapping required".into());
                }
            }
            Err(e) => return Err(e.into()),
            _ => return Err("inherited/fallback source requires explicit mapping".into()),
        }
    }
    let plan = plan(name, &assignments, true)?;
    println!(
        "{}",
        install(&plan, &output, std::slice::from_ref(&output), &|| false)?.display()
    );
    println!(
        "{}; no desktop settings changed",
        plan.system_coverage().summary()
    );
    Ok(())
}
