use std::fs;
use std::path::Path;

// The observer speaks the untagged NCP 1.0.0-rc.1 candidate (wire 1.0, compact
// contract 163acc57d8a62b66). The commit is the candidate's identity until NCP cuts
// the v1.0.0 tag. Wire 0.8 is retired; its last pin is in the git history.
const NCP_LABEL: &str = "v1.0.0-rc.1";
const NCP_VERSION: &str = "1.0.0-rc.1";
const NCP_REVISION: &str = "2819dae3b6338bb1df6d105ebb5b7433936a993d";
const NCP_GIT_URL: &str = "https://github.com/sepahead/NCP";
// The reviewed Zenoh 1.9.0 security backport (RUSTSEC-2026-0041) that NCP applies at
// every consuming root; a root patch does not propagate from a library dependency.
const ZENOH_TRANSPORT_GIT_URL: &str = "https://github.com/sepahead/zenoh-transport-lz4-backport";
const ZENOH_TRANSPORT_REVISION: &str = "9045545b72a77602a87f40203cb614b48157b4bc";

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

fn verify_exact_ncp_pin_text(manifest: &str, lock: &str) -> Result<(), String> {
    for package in ["ncp-core", "ncp-zenoh"] {
        let exact_dependency =
            format!("{package} = {{ git = \"{NCP_GIT_URL}\", rev = \"{NCP_REVISION}\" }}");
        if dependency_entry(manifest, package)? != exact_dependency {
            return Err(format!(
                "{package} must be the exact NCP {NCP_LABEL} git revision {NCP_REVISION}"
            ));
        }

        let block = package_block(lock, package)?;
        let expected_version = format!("version = \"{NCP_VERSION}\"");
        if !block.lines().any(|line| line == expected_version) {
            return Err(format!(
                "Cargo.lock resolved {package} away from NCP {NCP_VERSION}"
            ));
        }
        let expected_source =
            format!("source = \"git+{NCP_GIT_URL}?rev={NCP_REVISION}#{NCP_REVISION}\"");
        if !block.lines().any(|line| line == expected_source) {
            return Err(format!(
                "Cargo.lock resolved {package} away from the immutable {NCP_LABEL} commit {NCP_REVISION}"
            ));
        }
    }

    let transport = package_block(lock, "zenoh-transport")?;
    let expected_transport = format!(
        "source = \"git+{ZENOH_TRANSPORT_GIT_URL}?rev={ZENOH_TRANSPORT_REVISION}#{ZENOH_TRANSPORT_REVISION}\""
    );
    if !transport.lines().any(|line| line == expected_transport) {
        return Err(format!(
            "Cargo.lock must resolve zenoh-transport to the reviewed backport {ZENOH_TRANSPORT_REVISION}"
        ));
    }
    Ok(())
}

pub fn verify_exact_ncp_pin(manifest_dir: &Path) {
    let manifest_path = manifest_dir.join("Cargo.toml");
    let lock_path = manifest_dir.join("Cargo.lock");
    let manifest = fs::read_to_string(&manifest_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", manifest_path.display()));
    let lock = fs::read_to_string(&lock_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", lock_path.display()));
    verify_exact_ncp_pin_text(&manifest, &lock).unwrap_or_else(|error| panic!("{error}"));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> String {
        let dependencies = ["ncp-core", "ncp-zenoh"]
            .map(|package| {
                format!("{package} = {{ git = \"{NCP_GIT_URL}\", rev = \"{NCP_REVISION}\" }}")
            })
            .join("\n");
        format!("[dependencies]\n{dependencies}\n")
    }

    fn lock() -> String {
        let mut packages = ["ncp-core", "ncp-zenoh"].map(lock_package).join("\n");
        packages.push('\n');
        packages.push_str(&transport_package());
        packages
    }

    fn lock_package(package: &str) -> String {
        format!(
            "[[package]]\nname = \"{package}\"\nversion = \"{NCP_VERSION}\"\nsource = \"git+{NCP_GIT_URL}?rev={NCP_REVISION}#{NCP_REVISION}\"\n"
        )
    }

    fn transport_package() -> String {
        format!(
            "[[package]]\nname = \"zenoh-transport\"\nversion = \"1.9.0\"\nsource = \"git+{ZENOH_TRANSPORT_GIT_URL}?rev={ZENOH_TRANSPORT_REVISION}#{ZENOH_TRANSPORT_REVISION}\"\n"
        )
    }

    #[test]
    fn exact_pin_passes() {
        verify_exact_ncp_pin_text(&manifest(), &lock()).unwrap();
    }

    #[test]
    fn manifest_revision_drift_fails() {
        let manifest = manifest().replacen(NCP_REVISION, &"0".repeat(40), 1);
        assert!(verify_exact_ncp_pin_text(&manifest, &lock()).is_err());
    }

    #[test]
    fn movable_tag_or_retired_wire_fails() {
        for selector in [
            "tag = \"v1.0.0-rc.1\"",
            "tag = \"v0.8.0\"",
            "branch = \"main\"",
        ] {
            let manifest = manifest().replacen(&format!("rev = \"{NCP_REVISION}\""), selector, 1);
            assert!(
                verify_exact_ncp_pin_text(&manifest, &lock()).is_err(),
                "{selector}"
            );
        }
    }

    #[test]
    fn locked_version_drift_fails() {
        let lock = lock().replacen("version = \"1.0.0-rc.1\"", "version = \"0.8.0\"", 1);
        assert!(verify_exact_ncp_pin_text(&manifest(), &lock).is_err());
    }

    #[test]
    fn locked_revision_drift_fails() {
        let lock = lock().replacen(
            &format!("#{NCP_REVISION}"),
            &format!("#{}", "0".repeat(40)),
            1,
        );
        assert!(verify_exact_ncp_pin_text(&manifest(), &lock).is_err());
    }

    #[test]
    fn missing_locked_package_fails() {
        let lock = format!("{}\n{}", lock_package("ncp-core"), transport_package());
        assert!(verify_exact_ncp_pin_text(&manifest(), &lock).is_err());
    }

    #[test]
    fn duplicate_locked_package_fails() {
        let lock = format!("{}\n{}", lock(), lock_package("ncp-core"));
        assert!(verify_exact_ncp_pin_text(&manifest(), &lock).is_err());
    }

    #[test]
    fn manifest_git_url_drift_fails() {
        let manifest = manifest().replacen(NCP_GIT_URL, "https://example.invalid/NCP", 1);
        assert!(verify_exact_ncp_pin_text(&manifest, &lock()).is_err());
    }

    #[test]
    fn metadata_decoy_cannot_hide_dependency_drift() {
        let exact_core =
            format!("ncp-core = {{ git = \"{NCP_GIT_URL}\", rev = \"{NCP_REVISION}\" }}");
        let drifted_dependencies = manifest().replacen(
            &exact_core,
            &format!("ncp-core = {{ git = \"{NCP_GIT_URL}\", tag = \"v0.8.0\" }}"),
            1,
        );
        let manifest = format!("[package.metadata]\n{exact_core}\n{drifted_dependencies}");

        assert!(verify_exact_ncp_pin_text(&manifest, &lock()).is_err());
    }

    #[test]
    fn mixed_core_and_zenoh_revision_drift_fails() {
        let zenoh = lock_package("ncp-zenoh");
        let drifted_zenoh = zenoh.replace(NCP_REVISION, &"f".repeat(40));
        let mixed_lock = lock().replace(&zenoh, &drifted_zenoh);
        assert!(verify_exact_ncp_pin_text(&manifest(), &mixed_lock).is_err());
    }

    #[test]
    fn registry_zenoh_transport_without_the_backport_fails() {
        let registry = "[[package]]\nname = \"zenoh-transport\"\nversion = \"1.9.0\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n";
        let lock = lock().replace(&transport_package(), registry);
        assert!(verify_exact_ncp_pin_text(&manifest(), &lock).is_err());
    }

    #[test]
    fn missing_zenoh_transport_fails() {
        let lock = lock().replace(&transport_package(), "");
        assert!(verify_exact_ncp_pin_text(&manifest(), &lock).is_err());
    }
}
