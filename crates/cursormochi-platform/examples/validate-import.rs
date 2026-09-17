//! Development-only generator for independent libXcursor comparison.
use cursormochi_app::import::{plan, suggestion};
use cursormochi_platform::import::{install, load};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("temporary output directory required")?,
    );
    let source = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/import-fixtures/theme.zip")
        .canonicalize()?;
    let package = load(&source, &|| false)?;
    let mut assignments = package
        .assets
        .into_iter()
        .map(|a| Ok((suggestion(&a.path).ok_or("fixture role")?, a)))
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    let compatibility = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/ani-compat/busy.ani")
        .canonicalize()?;
    for asset in load(&compatibility, &|| false)?.assets {
        assignments.push((4, asset));
    }
    let resize_asset = assignments
        .iter()
        .find(|(role, _)| *role == 3)
        .ok_or("animated resize fixture")?
        .1
        .clone();
    for role in [5, 6, 9, 13] {
        assignments.push((role, resize_asset.clone()));
    }
    let plan = plan("Independent-Import", &assignments, true)?;
    println!(
        "{}",
        install(&plan, &root, std::slice::from_ref(&root), &|| false)?.display()
    );
    Ok(())
}
