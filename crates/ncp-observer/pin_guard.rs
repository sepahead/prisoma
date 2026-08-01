use std::fs;
use std::path::Path;

const LEGACY_NCP_TAG: &str = "v0.8.0";
const LEGACY_NCP_VERSION: &str = "0.8.0";
const LEGACY_NCP_REVISION: &str = "2f5bd586d4bb20c90362bb6f5698b7f64057ba4e";
const NCP_GIT_URL: &str = "https://github.com/sepahead/NCP";

fn package_block<'a>(lock: &'a str, name: &str) -> Result<&'a str, String> {
    let expected_name = format!("name = \"{name}\"");
    let mut matches = lock
        .split("[[package]]")
        .filter(|block| block.lines().any(|line| line == expected_name));
    let block = matches
        .next()
        .ok_or_else(|| format!("Cargo.lock is missing {name}"))?;
    if matches.next().is_some() {
        return Err(format!("Cargo.lock contains duplicate {name} packages"));
    }
    Ok(block)
}

fn dependency_entry<'a>(manifest: &'a str, package: &str) -> Result<&'a str, String> {
    let mut in_dependencies = false;
    let mut saw_dependencies = false;
    let mut matches = Vec::new();

    for raw_line in manifest.lines() {
        let line = raw_line.trim();
        if line.starts_with('[') {
            in_dependencies = line == "[dependencies]";
            if in_dependencies {
                if saw_dependencies {
                    return Err("Cargo.toml contains duplicate [dependencies] tables".to_owned());
                }
                saw_dependencies = true;
            }
            continue;
        }

        if !in_dependencies || line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, _)) = line.split_once('=') {
            if key.trim() == package {
                matches.push(line);
            }
        }
    }

    if !saw_dependencies {
        return Err("Cargo.toml is missing its top-level [dependencies] table".to_owned());
    }
    match matches.as_slice() {
        [entry] => Ok(entry),
        [] => Err(format!(
            "Cargo.toml [dependencies] is missing the {package} dependency"
        )),
        _ => Err(format!(
            "Cargo.toml [dependencies] contains duplicate {package} dependencies"
        )),
    }
}

fn verify_frozen_legacy_ncp_pin_text(manifest: &str, lock: &str) -> Result<(), String> {
    for package in ["ncp-core", "ncp-zenoh"] {
        let exact_dependency =
            format!("{package} = {{ git = \"{NCP_GIT_URL}\", tag = \"{LEGACY_NCP_TAG}\" }}");
        if dependency_entry(manifest, package)? != exact_dependency {
            return Err(format!(
                "{package} must remain an exact {LEGACY_NCP_TAG} git dependency; wire migrations require a separate consumer surface"
            ));
        }

        let block = package_block(lock, package)?;
        let expected_version = format!("version = \"{LEGACY_NCP_VERSION}\"");
        if !block.lines().any(|line| line == expected_version) {
            return Err(format!(
                "Cargo.lock resolved {package} away from NCP {LEGACY_NCP_VERSION}"
            ));
        }
        let expected_source =
            format!("source = \"git+{NCP_GIT_URL}?tag={LEGACY_NCP_TAG}#{LEGACY_NCP_REVISION}\"");
        if !block.lines().any(|line| line == expected_source) {
            return Err(format!(
                "Cargo.lock resolved {package} away from the immutable {LEGACY_NCP_TAG} commit {LEGACY_NCP_REVISION}"
            ));
        }
    }
    Ok(())
}

pub fn verify_frozen_legacy_ncp_pin(manifest_dir: &Path) {
    let manifest_path = manifest_dir.join("Cargo.toml");
    let lock_path = manifest_dir.join("Cargo.lock");
    let manifest = fs::read_to_string(&manifest_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", manifest_path.display()));
    let lock = fs::read_to_string(&lock_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", lock_path.display()));
    verify_frozen_legacy_ncp_pin_text(&manifest, &lock).unwrap_or_else(|error| panic!("{error}"));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> String {
        let dependencies = ["ncp-core", "ncp-zenoh"]
            .map(|package| {
                format!("{package} = {{ git = \"{NCP_GIT_URL}\", tag = \"{LEGACY_NCP_TAG}\" }}")
            })
            .join("\n");
        format!("[dependencies]\n{dependencies}\n")
    }

    fn lock() -> String {
        ["ncp-core", "ncp-zenoh"].map(lock_package).join("\n")
    }

    fn lock_package(package: &str) -> String {
        format!(
            "[[package]]\nname = \"{package}\"\nversion = \"{LEGACY_NCP_VERSION}\"\nsource = \"git+{NCP_GIT_URL}?tag={LEGACY_NCP_TAG}#{LEGACY_NCP_REVISION}\"\n"
        )
    }

    #[test]
    fn exact_frozen_pin_passes() {
        verify_frozen_legacy_ncp_pin_text(&manifest(), &lock()).unwrap();
    }

    #[test]
    fn manifest_tag_drift_fails() {
        let manifest = manifest().replace("tag = \"v0.8.0\"", "tag = \"v1.0.0\"");
        assert!(verify_frozen_legacy_ncp_pin_text(&manifest, &lock()).is_err());
    }

    #[test]
    fn locked_version_drift_fails() {
        let lock = lock().replacen("version = \"0.8.0\"", "version = \"1.0.0\"", 1);
        assert!(verify_frozen_legacy_ncp_pin_text(&manifest(), &lock).is_err());
    }

    #[test]
    fn locked_revision_drift_fails() {
        let lock = lock().replacen(LEGACY_NCP_REVISION, &"0".repeat(40), 1);
        assert!(verify_frozen_legacy_ncp_pin_text(&manifest(), &lock).is_err());
    }

    #[test]
    fn missing_locked_package_fails() {
        assert!(verify_frozen_legacy_ncp_pin_text(&manifest(), &lock_package("ncp-core")).is_err());
    }

    #[test]
    fn duplicate_locked_package_fails() {
        let lock = format!("{}\n{}", lock(), lock_package("ncp-core"));
        assert!(verify_frozen_legacy_ncp_pin_text(&manifest(), &lock).is_err());
    }

    #[test]
    fn manifest_git_url_drift_fails() {
        let manifest = manifest().replacen(NCP_GIT_URL, "https://example.invalid/NCP", 1);
        assert!(verify_frozen_legacy_ncp_pin_text(&manifest, &lock()).is_err());
    }

    #[test]
    fn metadata_decoy_cannot_hide_dependency_drift() {
        let exact_core =
            format!("ncp-core = {{ git = \"{NCP_GIT_URL}\", tag = \"{LEGACY_NCP_TAG}\" }}");
        let drifted_dependencies = manifest().replacen(
            &exact_core,
            &format!("ncp-core = {{ git = \"{NCP_GIT_URL}\", tag = \"v1.0.0\" }}"),
            1,
        );
        let manifest = format!("[package.metadata]\n{exact_core}\n{drifted_dependencies}");

        assert!(verify_frozen_legacy_ncp_pin_text(&manifest, &lock()).is_err());
    }

    #[test]
    fn mixed_core_and_zenoh_revision_drift_fails() {
        let zenoh = lock_package("ncp-zenoh");
        let drifted_zenoh = zenoh.replace(LEGACY_NCP_REVISION, &"f".repeat(40));
        let mixed_lock = lock().replace(&zenoh, &drifted_zenoh);
        assert!(verify_frozen_legacy_ncp_pin_text(&manifest(), &mixed_lock).is_err());
    }
}
