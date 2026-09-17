//! Read-only import diagnostics. Never installs or constructs a settings backend.
use cursormochi_platform::import::load;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::path::PathBuf::from(std::env::args_os().nth(1).ok_or("input path required")?);
    let package = load(&path, &|| false)?;
    let mut failures = 0;
    let mut frames = 0;
    for a in &package.assets {
        match &a.decoded {
            Ok(d) => {
                let count: usize = d.variants.iter().map(|v| v.frames.len()).sum();
                frames += count;
                // Verify conversion in memory, without installing or writing artwork.
                let output = cursormochi_core::export::xcursor(&d.variants)?;
                let suggested = cursormochi_app::import::suggestion(&a.path)
                    .map(|i| cursormochi_app::import::IMPORT_ROLES[i].1[0])
                    .unwrap_or("unassigned");
                println!(
                    "OK {}: {} variants, {count} frames, {} output bytes; suggested role: {suggested}",
                    a.path.display(),
                    d.variants.len(),
                    output.len()
                );
            }
            Err(e) => {
                failures += 1;
                println!("FAIL {}: {e}", a.path.display());
            }
        }
    }
    println!(
        "{} files, {failures} failures, {frames} frames; {} non-cursor files ignored",
        package.assets.len(),
        package.ignored
    );
    Ok(())
}
