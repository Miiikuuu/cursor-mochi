//! Read-only trial preparation. Accepts the same decoded Preview used by inspection;
//! no installation requirement and no desktop settings capability.
use crate::{ThemeRepository, browser::ROLE_GROUPS};
use cursormochi_core::{Error, Preview, Resolution, ThemeName};

pub struct TrialSet {
    pub theme: ThemeName,
    pub size: u32,
    pub thumbnails: Vec<Result<cursormochi_core::Variant, Error>>,
    pub roles: Vec<(&'static str, Result<Preview, Error>)>,
}
const MAX_BYTES: usize = 16 * 1024 * 1024;
const MAX_FRAMES: usize = 256;
/// Bounded alias probes prefer direct/inherited evidence over automatic fallback.
pub fn load(
    repo: &dyn ThemeRepository,
    theme: &ThemeName,
    size: u32,
    cancel: &dyn Fn() -> bool,
) -> Result<TrialSet, Error> {
    let mut roles = Vec::new();
    let mut thumbnails = Vec::new();
    let mut thumbnail_bytes = 0;
    let mut bytes = 0;
    for (label, aliases) in ROLE_GROUPS {
        let mut found = Err(Error::Missing);
        for alias in aliases {
            if cancel() {
                return Err(Error::Cancelled);
            }
            match repo.preview(theme, alias, cancel) {
                Ok(p) if p.resolution != Resolution::Fallback => {
                    found = Ok(p);
                    break;
                }
                Ok(p) if found.is_err() => found = Ok(p),
                Err(Error::Cancelled) => return Err(Error::Cancelled),
                Err(e) if matches!(found, Err(Error::Missing)) => found = Err(e),
                _ => (),
            }
        }
        thumbnails.push(
            found
                .as_ref()
                .map_err(Clone::clone)
                .and_then(|p| source_thumbnail(p, &mut thumbnail_bytes)),
        );
        let prepared = found.and_then(|p| prepare(p, size, cancel)).and_then(|p| {
            let n: usize = p.variants[0].frames.iter().map(|f| f.rgba.len()).sum();
            if n > MAX_BYTES - bytes {
                return Err(Error::LimitExceeded("trial total pixels"));
            }
            bytes += n;
            Ok(p)
        });
        roles.push((label, prepared));
    }
    Ok(TrialSet {
        theme: theme.clone(),
        size,
        thumbnails,
        roles,
    })
}
/// Retain original pixels independently of the small widget cursor. Arc-backed
/// frames reuse decoding; the additional retained source set has a hard budget.
pub fn source_thumbnail(
    p: &Preview,
    bytes: &mut usize,
) -> Result<cursormochi_core::Variant, Error> {
    let v = crate::browser::thumbnail_variant(p, 128).ok_or(Error::Missing)?;
    if v.frames.len() > MAX_FRAMES {
        return Err(Error::LimitExceeded("thumbnail animation frames"));
    }
    let n = v.frames.iter().map(|f| f.rgba.len()).sum::<usize>();
    if n > (64 * 1024 * 1024usize).saturating_sub(*bytes) {
        return Err(Error::LimitExceeded("source thumbnail pixels"));
    }
    *bytes += n;
    Ok(v.clone())
}

/// Future importers can supply decoded Preview directly, before installation.
/// Preserve aspect ratio and scale hotspot by the same nominal-size ratio.
pub fn prepare(mut p: Preview, size: u32, cancel: &dyn Fn() -> bool) -> Result<Preview, Error> {
    if !(16..=64).contains(&size) {
        return Err(Error::Invalid("trial size must be 16–64"));
    }
    let v = p
        .variants
        .iter()
        .min_by_key(|v| v.nominal.abs_diff(size))
        .ok_or(Error::Missing)?;
    if v.nominal == 0 || v.frames.is_empty() {
        return Err(Error::Invalid("empty trial variant"));
    }
    if v.frames.len() > MAX_FRAMES {
        return Err(Error::LimitExceeded("trial animation frames"));
    }
    p.variants = vec![cursormochi_core::resize::variant(
        v, size, MAX_BYTES, cancel,
    )?];
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cursormochi_core::{Frame, Variant};
    fn preview() -> Preview {
        Preview {
            requested: ThemeName::new("not-installed").unwrap(),
            role: "left_ptr".into(),
            source: "staging/cursor".into(),
            chain: vec!["parent".into()],
            resolution: Resolution::Inherited,
            variants: vec![Variant {
                nominal: 32,
                frames: vec![
                    Frame {
                        width: 4,
                        height: 2,
                        hotspot: (2, 1),
                        delay: 80,
                        rgba: vec![255; 32].into()
                    };
                    2
                ],
            }],
        }
    }
    #[test]
    fn thumbnails_choose_sufficient_actual_pixels_and_keep_original_animation() {
        let mut p = preview();
        p.variants.clear();
        for size in [24, 48, 96] {
            p.variants.push(Variant {
                nominal: size,
                frames: vec![
                    Frame {
                        width: size,
                        height: size / 2,
                        hotspot: (1, 1),
                        delay: 80,
                        rgba: vec![255; (size * size / 2 * 4) as usize].into(),
                    };
                    2
                ],
            });
        }
        assert_eq!(
            crate::browser::thumbnail_variant(&p, 44).unwrap().nominal,
            48
        );
        assert_eq!(
            crate::browser::thumbnail_variant(&p, 88).unwrap().nominal,
            96
        );
        assert_eq!(
            crate::browser::thumbnail_variant(&p, 128).unwrap().nominal,
            96
        );
        let source = source_thumbnail(&p, &mut 0).unwrap();
        let cursor = prepare(p, 24, &|| false).unwrap();
        assert_eq!(cursor.variants[0].frames[0].width, 24);
        assert_eq!(source.frames[0].width, 96);
        assert_eq!(source.frames.len(), 2);
        assert_eq!(source.frames[1].delay, 80);
        assert_eq!(source.frames[0].hotspot, (1, 1));
    }
    #[test]
    fn source_thumbnail_retention_is_bounded_and_reuses_decoded_pixels() {
        let p = preview();
        let v = source_thumbnail(&p, &mut 0).unwrap();
        assert!(std::sync::Arc::ptr_eq(
            &v.frames[0].rgba,
            &p.variants[0].frames[0].rgba
        ));
        assert!(matches!(
            source_thumbnail(&p, &mut (64 * 1024 * 1024)),
            Err(Error::LimitExceeded(_))
        ));
    }
    #[test]
    fn preinstallation_frames_keep_hotspots_delays_and_provenance() {
        let p = prepare(preview(), 64, &|| false).unwrap();
        let f = &p.variants[0].frames[0];
        assert_eq!((f.width, f.height, f.hotspot, f.delay), (8, 4, (4, 2), 80));
        assert_eq!(p.variants[0].frames.len(), 2);
        assert_eq!(p.resolution, Resolution::Inherited);
        assert_eq!(p.chain, vec!["parent"]);
        assert_eq!(p.requested.as_str(), "not-installed");
    }
    #[test]
    fn trial_budgets_and_cancellation_are_explicit_errors() {
        assert!(matches!(
            prepare(preview(), 100, &|| false),
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            prepare(preview(), 24, &|| true),
            Err(Error::Cancelled)
        ));
        let mut p = preview();
        p.variants[0].frames = vec![p.variants[0].frames[0].clone(); 257];
        assert!(matches!(
            prepare(p, 24, &|| false),
            Err(Error::LimitExceeded(_))
        ));
    }
    struct Repo;
    impl ThemeRepository for Repo {
        fn scan(&self, _: &dyn Fn() -> bool) -> Result<crate::Catalog, Error> {
            unreachable!()
        }
        fn preview(&self, _: &ThemeName, r: &str, _: &dyn Fn() -> bool) -> Result<Preview, Error> {
            let mut p = preview();
            p.role = r.into();
            match r {
                "left_ptr" => p.resolution = Resolution::Fallback,
                "default" => p.resolution = Resolution::Direct,
                "watch" => p.resolution = Resolution::Fallback,
                _ => return Err(Error::Missing),
            };
            Ok(p)
        }
    }
    #[test]
    fn alias_probe_prefers_selected_theme_and_reports_fallback_and_missing() {
        let data = load(
            &Repo,
            &ThemeName::new("not-installed").unwrap(),
            24,
            &|| false,
        )
        .unwrap();
        assert_eq!(data.roles[0].1.as_ref().unwrap().role, "default");
        assert_eq!(
            data.roles[3].1.as_ref().unwrap().resolution,
            Resolution::Fallback
        );
        assert!(matches!(data.roles[4].1, Err(Error::Missing)));
    }
}
