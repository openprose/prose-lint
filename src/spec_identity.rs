use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

const SUPPORTED_SCHEMA: &str = "openprose.spec-identity";
const SUPPORTED_SCHEMA_VERSION: u32 = 1;
const BASE_REQUIRED_ARTIFACTS: &[&str] =
    &["SKILL.md", "contract-markdown.md", "forme.md", "prose.md"];
const RUNTIME_CONTRACT_2_ARTIFACTS: &[&str] = &["prosescript.md", "responsibility-runtime.md"];

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SpecIdentityManifest {
    pub schema: String,
    pub schema_version: u32,
    pub spec_id: String,
    pub source: SourceIdentity,
    pub skill: SkillIdentity,
    #[serde(default)]
    pub packages: BTreeMap<String, String>,
    pub artifacts: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceIdentity {
    pub repo: String,
    #[serde(default)]
    pub commit: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SkillIdentity {
    pub version: String,
    pub runtime_contract: u32,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SpecIdentityOptions {
    pub root: Option<PathBuf>,
    pub git_repo: Option<PathBuf>,
    pub expected_repo: Option<String>,
    pub expected_commit: Option<String>,
    pub package_jsons: Vec<PathBuf>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SpecIdentityReport {
    pub valid: bool,
    pub manifest: PathBuf,
    pub root: PathBuf,
    pub checks: Vec<SpecIdentityCheck>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SpecIdentityCheck {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

#[derive(Clone, Debug, Deserialize)]
struct PackageJson {
    name: String,
    version: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct SkillDocumentIdentity {
    version: Option<String>,
    runtime_contract: Option<u32>,
}

impl SpecIdentityReport {
    fn new(manifest: PathBuf, root: PathBuf) -> Self {
        Self {
            valid: true,
            manifest,
            root,
            checks: Vec::new(),
        }
    }

    fn check(&mut self, name: impl Into<String>, passed: bool, detail: impl Into<String>) {
        if !passed {
            self.valid = false;
        }
        self.checks.push(SpecIdentityCheck {
            name: name.into(),
            passed,
            detail: detail.into(),
        });
    }
}

pub fn verify_spec_identity(
    manifest_path: &Path,
    options: SpecIdentityOptions,
) -> Result<SpecIdentityReport> {
    let manifest_path = manifest_path
        .canonicalize()
        .with_context(|| format!("canonicalize {}", manifest_path.display()))?;
    let root = match &options.root {
        Some(root) => root
            .canonicalize()
            .with_context(|| format!("canonicalize {}", root.display()))?,
        None => manifest_path
            .parent()
            .expect("canonicalized manifest has a parent")
            .to_path_buf(),
    };
    let manifest = load_manifest(&manifest_path)?;
    let mut report = SpecIdentityReport::new(manifest_path, root.clone());

    validate_manifest_shape(&manifest, &mut report);
    verify_source_repo(&manifest, &options, &mut report);
    verify_required_artifacts(&manifest, &mut report);
    verify_artifacts(&manifest, &root, &mut report);
    verify_skill_document(&manifest, &root, &mut report);
    let git_toplevel = verify_git_root(&root, &options, &mut report)?;
    verify_expected_commit(&manifest, &options, &mut report)?;
    verify_git_artifacts(
        &manifest,
        &root,
        git_toplevel.as_deref(),
        &options,
        &mut report,
    )?;
    verify_packages(&manifest, &options.package_jsons, &mut report)?;

    Ok(report)
}

pub fn artifact_digest(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let digest = Sha256::digest(&bytes);
    Ok(format!("sha256:{digest:x}"))
}

fn load_manifest(path: &Path) -> Result<SpecIdentityManifest> {
    let source = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let manifest: SpecIdentityManifest =
        serde_json::from_str(&source).with_context(|| format!("parse {}", path.display()))?;
    Ok(manifest)
}

fn validate_manifest_shape(manifest: &SpecIdentityManifest, report: &mut SpecIdentityReport) {
    report.check(
        "schema",
        manifest.schema == SUPPORTED_SCHEMA,
        format!("expected {SUPPORTED_SCHEMA}, got {}", manifest.schema),
    );
    report.check(
        "schema_version",
        manifest.schema_version == SUPPORTED_SCHEMA_VERSION,
        format!(
            "expected {}, got {}",
            SUPPORTED_SCHEMA_VERSION, manifest.schema_version
        ),
    );
    report.check(
        "spec_id",
        !manifest.spec_id.trim().is_empty(),
        format!("spec_id={}", manifest.spec_id),
    );
    report.check(
        "source.repo",
        !manifest.source.repo.trim().is_empty(),
        format!("source.repo={}", manifest.source.repo),
    );
    report.check(
        "skill.version",
        !manifest.skill.version.trim().is_empty(),
        format!("skill.version={}", manifest.skill.version),
    );
    report.check(
        "skill.runtime_contract",
        manifest.skill.runtime_contract > 0,
        format!("runtime_contract={}", manifest.skill.runtime_contract),
    );
    report.check(
        "artifacts",
        !manifest.artifacts.is_empty(),
        format!("{} artifact(s)", manifest.artifacts.len()),
    );
}

fn verify_source_repo(
    manifest: &SpecIdentityManifest,
    options: &SpecIdentityOptions,
    report: &mut SpecIdentityReport,
) {
    if let Some(expected) = &options.expected_repo {
        report.check(
            "source.repo.expected",
            manifest.source.repo == *expected,
            format!("manifest={}, expected={expected}", manifest.source.repo),
        );
    } else {
        report.check(
            "source.repo.expected",
            false,
            "spec identity requires --expect-repo or spec registry repo identity",
        );
    }
}

fn verify_required_artifacts(manifest: &SpecIdentityManifest, report: &mut SpecIdentityReport) {
    for required in required_artifacts(manifest) {
        report.check(
            format!("artifact.required:{required}"),
            manifest.artifacts.contains_key(required),
            format!("required artifact {required}"),
        );
    }
}

fn required_artifacts(manifest: &SpecIdentityManifest) -> Vec<&'static str> {
    let mut required = BASE_REQUIRED_ARTIFACTS.to_vec();
    if manifest.skill.runtime_contract >= 2 {
        required.extend_from_slice(RUNTIME_CONTRACT_2_ARTIFACTS);
    }
    required
}

fn verify_artifacts(manifest: &SpecIdentityManifest, root: &Path, report: &mut SpecIdentityReport) {
    for (relative, expected) in &manifest.artifacts {
        let name = format!("artifact:{relative}");
        let Ok(path) = safe_join(root, relative) else {
            report.check(name, false, "artifact path escapes root");
            continue;
        };
        match artifact_digest(&path) {
            Ok(actual) => report.check(
                name,
                &actual == expected,
                format!("expected {expected}, got {actual}"),
            ),
            Err(error) => report.check(name, false, error.to_string()),
        }
    }
}

fn verify_skill_document(
    manifest: &SpecIdentityManifest,
    root: &Path,
    report: &mut SpecIdentityReport,
) {
    if !manifest.artifacts.contains_key("SKILL.md") {
        report.check(
            "skill.document",
            false,
            "SKILL.md artifact is required to verify skill metadata",
        );
        return;
    }

    let Ok(path) = safe_join(root, "SKILL.md") else {
        report.check("skill.document", false, "SKILL.md path escapes root");
        return;
    };
    match load_skill_document_identity(&path) {
        Ok(identity) => {
            let actual_version = identity.version.unwrap_or_default();
            report.check(
                "skill.version.document",
                actual_version == manifest.skill.version,
                format!(
                    "manifest={}, SKILL.md={actual_version}",
                    manifest.skill.version
                ),
            );
            let actual_contract = identity.runtime_contract.unwrap_or_default();
            report.check(
                "skill.runtime_contract.document",
                actual_contract == manifest.skill.runtime_contract,
                format!(
                    "manifest={}, SKILL.md={actual_contract}",
                    manifest.skill.runtime_contract
                ),
            );
        }
        Err(error) => report.check("skill.document", false, error.to_string()),
    }
}

fn verify_git_root(
    root: &Path,
    options: &SpecIdentityOptions,
    report: &mut SpecIdentityReport,
) -> Result<Option<PathBuf>> {
    let Some(repo) = &options.git_repo else {
        return Ok(None);
    };

    let toplevel = git_toplevel(repo)?;
    report.check(
        "git.root",
        root.starts_with(&toplevel),
        format!(
            "root={}, git_toplevel={}",
            root.display(),
            toplevel.display()
        ),
    );
    Ok(Some(toplevel))
}

fn verify_expected_commit(
    manifest: &SpecIdentityManifest,
    options: &SpecIdentityOptions,
    report: &mut SpecIdentityReport,
) -> Result<()> {
    if let Some(expected) = &options.expected_commit {
        if options.git_repo.is_none() {
            report.check(
                "git.repo",
                false,
                "--expect-commit requires --git-repo so the commit can be checked",
            );
        }
        if let Some(manifest_commit) = &manifest.source.commit {
            report.check(
                "source.commit",
                manifest_commit == expected,
                format!("manifest={manifest_commit}, expected={expected}"),
            );
        } else {
            report.check(
                "source.commit",
                true,
                format!("manifest omitted commit; external expected commit={expected}"),
            );
        }
    }

    if let Some(repo) = &options.git_repo {
        let actual = git_head(repo)?;
        let expected = options
            .expected_commit
            .as_deref()
            .or(manifest.source.commit.as_deref());
        match expected {
            Some(expected) => report.check(
                "git.head",
                actual == expected,
                format!("git HEAD={actual}, expected={expected}"),
            ),
            None => report.check(
                "git.head",
                true,
                format!("git HEAD={actual}; no expected commit supplied"),
            ),
        }
    }

    Ok(())
}

fn verify_git_artifacts(
    manifest: &SpecIdentityManifest,
    root: &Path,
    git_toplevel: Option<&Path>,
    options: &SpecIdentityOptions,
    report: &mut SpecIdentityReport,
) -> Result<()> {
    let Some(repo) = &options.git_repo else {
        return Ok(());
    };
    let expected_commit = options
        .expected_commit
        .as_deref()
        .or(manifest.source.commit.as_deref());
    let Some(expected_commit) = expected_commit else {
        report.check(
            "git.commit",
            false,
            "git artifact proof requires --expect-commit or source.commit",
        );
        return Ok(());
    };
    let Some(toplevel) = git_toplevel else {
        return Ok(());
    };
    if !root.starts_with(toplevel) {
        return Ok(());
    }

    let root_relative = root
        .strip_prefix(toplevel)
        .with_context(|| format!("strip git root prefix from {}", root.display()))?;
    for (relative, expected) in &manifest.artifacts {
        let name = format!("git.artifact:{relative}");
        let Ok(worktree_relative) = safe_join(root_relative, relative) else {
            report.check(name, false, "artifact path escapes root");
            continue;
        };
        let git_path = match git_tree_path(&worktree_relative) {
            Ok(path) => path,
            Err(error) => {
                report.check(name, false, error.to_string());
                continue;
            }
        };
        match git_blob_digest(repo, expected_commit, &git_path) {
            Ok(actual) => report.check(
                name,
                &actual == expected,
                format!("{expected_commit}:{git_path} expected {expected}, got {actual}"),
            ),
            Err(error) => report.check(name, false, error.to_string()),
        }
    }
    Ok(())
}

fn verify_packages(
    manifest: &SpecIdentityManifest,
    package_jsons: &[PathBuf],
    report: &mut SpecIdentityReport,
) -> Result<()> {
    let mut seen = BTreeMap::new();
    for path in package_jsons {
        let source =
            fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        let package: PackageJson =
            serde_json::from_str(&source).with_context(|| format!("parse {}", path.display()))?;
        seen.insert(package.name.clone(), package.version.clone());
        let name = format!("package:{}@{}", package.name, package.version);
        match manifest.packages.get(&package.name) {
            Some(expected) => report.check(
                name,
                expected == &package.version,
                format!("manifest={expected}, package.json={}", package.version),
            ),
            None => report.check(name, false, "package missing from manifest"),
        }
    }
    for (name, expected) in &manifest.packages {
        if !seen.contains_key(name) {
            report.check(
                format!("package:{name}"),
                false,
                format!(
                    "manifest declares {name}@{expected}, but no matching package.json was supplied"
                ),
            );
        }
    }
    Ok(())
}

fn safe_join(root: &Path, relative: &str) -> Result<PathBuf> {
    let path = Path::new(relative);
    if path.is_absolute() {
        bail!("artifact path must be relative: {relative}");
    }
    let mut joined = root.to_path_buf();
    for component in path.components() {
        match component {
            Component::Normal(part) => joined.push(part),
            Component::CurDir => {}
            _ => bail!("artifact path escapes root: {relative}"),
        }
    }
    Ok(joined)
}

fn load_skill_document_identity(path: &Path) -> Result<SkillDocumentIdentity> {
    let source = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let mut lines = source.lines();
    if lines.next().map(str::trim) != Some("---") {
        bail!("SKILL.md is missing YAML frontmatter");
    }

    let mut identity = SkillDocumentIdentity::default();
    for line in lines {
        let line = line.trim();
        if line == "---" {
            return Ok(identity);
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        match key.trim() {
            "version" => identity.version = Some(clean_frontmatter_scalar(value)),
            "runtime_contract" => {
                let value = clean_frontmatter_scalar(value);
                identity.runtime_contract = Some(
                    value
                        .parse()
                        .with_context(|| format!("parse runtime_contract={value}"))?,
                );
            }
            _ => {}
        }
    }
    bail!("SKILL.md frontmatter is not closed")
}

fn clean_frontmatter_scalar(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string()
}

fn git_toplevel(repo: &Path) -> Result<PathBuf> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .arg("rev-parse")
        .arg("--show-toplevel")
        .output()
        .with_context(|| format!("run git rev-parse --show-toplevel in {}", repo.display()))?;
    if !output.status.success() {
        bail!(
            "git rev-parse --show-toplevel failed in {}: {}",
            repo.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let path = String::from_utf8(output.stdout)
        .context("git output was not UTF-8")?
        .trim()
        .to_string();
    PathBuf::from(path)
        .canonicalize()
        .with_context(|| format!("canonicalize git toplevel for {}", repo.display()))
}

fn git_tree_path(path: &Path) -> Result<String> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => {
                let Some(part) = part.to_str() else {
                    bail!("git path component is not UTF-8: {}", path.display());
                };
                parts.push(part);
            }
            Component::CurDir => {}
            _ => bail!("git path escapes repository: {}", path.display()),
        }
    }
    Ok(parts.join("/"))
}

fn git_blob_digest(repo: &Path, commit: &str, path: &str) -> Result<String> {
    let spec = format!("{commit}:{path}");
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .arg("show")
        .arg(&spec)
        .output()
        .with_context(|| format!("run git show {spec} in {}", repo.display()))?;
    if !output.status.success() {
        bail!(
            "git show failed for {} in {}: {}",
            spec,
            repo.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let digest = Sha256::digest(&output.stdout);
    Ok(format!("sha256:{digest:x}"))
}

fn git_head(repo: &Path) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .with_context(|| format!("run git rev-parse in {}", repo.display()))?;
    if !output.status.success() {
        bail!(
            "git rev-parse failed in {}: {}",
            repo.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8(output.stdout)
        .context("git output was not UTF-8")?
        .trim()
        .to_string())
}

#[cfg(test)]
mod tests {
    use super::{SpecIdentityOptions, artifact_digest, verify_spec_identity};
    use serde_json::json;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use tempfile::tempdir;

    fn write_skill(root: &Path, contract_text: &str) {
        write_skill_with_metadata(root, "0.15.0", 2, contract_text);
    }

    fn write_skill_with_metadata(
        root: &Path,
        version: &str,
        runtime_contract: u32,
        contract_text: &str,
    ) {
        fs::create_dir_all(root).unwrap();
        fs::write(
            root.join("SKILL.md"),
            format!(
                "---\nname: open-prose\nversion: {version}\nruntime_contract: {runtime_contract}\n---\n"
            ),
        )
        .unwrap();
        fs::write(root.join("contract-markdown.md"), contract_text).unwrap();
        fs::write(root.join("prose.md"), "Prose VM\n").unwrap();
        fs::write(root.join("forme.md"), "Forme\n").unwrap();
        fs::write(root.join("prosescript.md"), "ProseScript\n").unwrap();
        fs::write(root.join("reactor.md"), "Reactor\n").unwrap();
        fs::write(
            root.join("responsibility-runtime.md"),
            "Responsibility Runtime\n",
        )
        .unwrap();
    }

    fn write_manifest(root: &Path, package_version: &str) -> PathBuf {
        let manifest = json!({
            "schema": "openprose.spec-identity",
            "schema_version": 1,
            "spec_id": "openprose",
            "source": {
                "repo": "openprose/prose"
            },
            "skill": {
                "version": "0.15.0",
                "runtime_contract": 2
            },
            "packages": {
                "@openprose/reactor": package_version
            },
            "artifacts": {
                "SKILL.md": artifact_digest(&root.join("SKILL.md")).unwrap(),
                "contract-markdown.md": artifact_digest(&root.join("contract-markdown.md")).unwrap(),
                "prose.md": artifact_digest(&root.join("prose.md")).unwrap(),
                "forme.md": artifact_digest(&root.join("forme.md")).unwrap(),
                "prosescript.md": artifact_digest(&root.join("prosescript.md")).unwrap(),
                "reactor.md": artifact_digest(&root.join("reactor.md")).unwrap(),
                "responsibility-runtime.md": artifact_digest(&root.join("responsibility-runtime.md")).unwrap()
            }
        });
        let path = root.join("spec-version.json");
        fs::write(&path, serde_json::to_string_pretty(&manifest).unwrap()).unwrap();
        path
    }

    fn write_manifest_without_packages(root: &Path) -> PathBuf {
        let manifest = json!({
            "schema": "openprose.spec-identity",
            "schema_version": 1,
            "spec_id": "openprose",
            "source": {
                "repo": "openprose/prose"
            },
            "skill": {
                "version": "0.15.0",
                "runtime_contract": 2
            },
            "packages": {},
            "artifacts": {
                "SKILL.md": artifact_digest(&root.join("SKILL.md")).unwrap(),
                "contract-markdown.md": artifact_digest(&root.join("contract-markdown.md")).unwrap(),
                "prose.md": artifact_digest(&root.join("prose.md")).unwrap(),
                "forme.md": artifact_digest(&root.join("forme.md")).unwrap(),
                "prosescript.md": artifact_digest(&root.join("prosescript.md")).unwrap(),
                "reactor.md": artifact_digest(&root.join("reactor.md")).unwrap(),
                "responsibility-runtime.md": artifact_digest(&root.join("responsibility-runtime.md")).unwrap()
            }
        });
        let path = root.join("spec-version.json");
        fs::write(&path, serde_json::to_string_pretty(&manifest).unwrap()).unwrap();
        path
    }

    fn git(repo: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    }

    fn commit(repo: &Path, message: &str) -> String {
        git(repo, &["add", "."]);
        git(repo, &["commit", "-m", message]);
        git(repo, &["rev-parse", "HEAD"])
    }

    #[test]
    fn verifies_two_pinned_git_commits_without_manifest_self_reference() {
        let dir = tempdir().unwrap();
        let repo = dir.path().join("prose");
        let skill_root = repo.join("skills/open-prose");
        fs::create_dir_all(&skill_root).unwrap();
        git(dir.path(), &["init", "prose"]);
        git(&repo, &["config", "user.email", "agent@example.invalid"]);
        git(&repo, &["config", "user.name", "Agent"]);

        write_skill(&skill_root, "contract one\n");
        let manifest_a = write_manifest_without_packages(&skill_root);
        let commit_a = commit(&repo, "contract one");

        let report = verify_spec_identity(
            &manifest_a,
            SpecIdentityOptions {
                root: Some(skill_root.clone()),
                git_repo: Some(repo.clone()),
                expected_repo: Some("openprose/prose".to_string()),
                expected_commit: Some(commit_a.clone()),
                package_jsons: vec![],
            },
        )
        .unwrap();
        assert!(report.valid, "{report:#?}");

        write_skill(&skill_root, "contract two\n");
        let manifest_b = write_manifest_without_packages(&skill_root);
        let commit_b = commit(&repo, "contract two");
        let report = verify_spec_identity(
            &manifest_b,
            SpecIdentityOptions {
                root: Some(skill_root.clone()),
                git_repo: Some(repo.clone()),
                expected_repo: Some("openprose/prose".to_string()),
                expected_commit: Some(commit_b),
                package_jsons: vec![],
            },
        )
        .unwrap();
        assert!(report.valid, "{report:#?}");

        let report = verify_spec_identity(
            &manifest_b,
            SpecIdentityOptions {
                root: Some(skill_root),
                git_repo: Some(repo),
                expected_repo: Some("openprose/prose".to_string()),
                expected_commit: Some(commit_a),
                package_jsons: vec![],
            },
        )
        .unwrap();
        assert!(!report.valid, "{report:#?}");
        assert!(
            report
                .checks
                .iter()
                .any(|check| check.name == "git.head" && !check.passed)
        );
    }

    #[test]
    fn rejects_dirty_artifact_bytes_not_present_in_pinned_commit() {
        let dir = tempdir().unwrap();
        let repo = dir.path().join("prose");
        let skill_root = repo.join("skills/open-prose");
        fs::create_dir_all(&skill_root).unwrap();
        git(dir.path(), &["init", "prose"]);
        git(&repo, &["config", "user.email", "agent@example.invalid"]);
        git(&repo, &["config", "user.name", "Agent"]);

        write_skill(&skill_root, "committed contract\n");
        let _committed_manifest = write_manifest_without_packages(&skill_root);
        let commit = commit(&repo, "committed contract");

        fs::write(skill_root.join("contract-markdown.md"), "dirty contract\n").unwrap();
        let dirty_manifest = write_manifest_without_packages(&skill_root);
        let report = verify_spec_identity(
            &dirty_manifest,
            SpecIdentityOptions {
                root: Some(skill_root),
                git_repo: Some(repo),
                expected_repo: Some("openprose/prose".to_string()),
                expected_commit: Some(commit),
                package_jsons: vec![],
            },
        )
        .unwrap();
        assert!(!report.valid, "{report:#?}");
        assert!(
            report.checks.iter().any(|check| {
                check.name == "git.artifact:contract-markdown.md" && !check.passed
            })
        );
    }

    #[test]
    fn verifies_package_json_versions_against_bundled_skill_manifest() {
        for version in ["0.3.0", "0.3.1"] {
            let dir = tempdir().unwrap();
            let package_root = dir.path().join("node_modules/@openprose/reactor");
            let skill_root = package_root.join("skill/open-prose");
            write_skill(&skill_root, "contract package\n");
            let manifest = write_manifest(&skill_root, version);
            fs::write(
                package_root.join("package.json"),
                serde_json::to_string_pretty(&json!({
                    "name": "@openprose/reactor",
                    "version": version
                }))
                .unwrap(),
            )
            .unwrap();

            let report = verify_spec_identity(
                &manifest,
                SpecIdentityOptions {
                    root: Some(skill_root),
                    expected_repo: Some("openprose/prose".to_string()),
                    package_jsons: vec![package_root.join("package.json")],
                    ..SpecIdentityOptions::default()
                },
            )
            .unwrap();
            assert!(report.valid, "{report:#?}");
        }
    }

    #[test]
    fn rejects_missing_expected_repo_identity_in_direct_mode() {
        let dir = tempdir().unwrap();
        let package_root = dir.path().join("node_modules/@openprose/reactor");
        let skill_root = package_root.join("skill/open-prose");
        write_skill(&skill_root, "contract package\n");
        let manifest = write_manifest(&skill_root, "0.3.1");
        fs::write(
            package_root.join("package.json"),
            serde_json::to_string_pretty(&json!({
                "name": "@openprose/reactor",
                "version": "0.3.1"
            }))
            .unwrap(),
        )
        .unwrap();

        let report = verify_spec_identity(
            &manifest,
            SpecIdentityOptions {
                root: Some(skill_root),
                package_jsons: vec![package_root.join("package.json")],
                ..SpecIdentityOptions::default()
            },
        )
        .unwrap();
        assert!(!report.valid, "{report:#?}");
        assert!(
            report
                .checks
                .iter()
                .any(|check| { check.name == "source.repo.expected" && !check.passed })
        );
    }

    #[test]
    fn rejects_tampered_artifacts_and_package_version_mismatch() {
        let dir = tempdir().unwrap();
        let package_root = dir.path().join("node_modules/@openprose/reactor");
        let skill_root = package_root.join("skill/open-prose");
        write_skill(&skill_root, "contract package\n");
        let manifest = write_manifest(&skill_root, "0.3.1");
        fs::write(skill_root.join("contract-markdown.md"), "tampered\n").unwrap();
        fs::write(
            package_root.join("package.json"),
            serde_json::to_string_pretty(&json!({
                "name": "@openprose/reactor",
                "version": "9.9.9"
            }))
            .unwrap(),
        )
        .unwrap();

        let report = verify_spec_identity(
            &manifest,
            SpecIdentityOptions {
                root: Some(skill_root),
                expected_repo: Some("openprose/prose".to_string()),
                package_jsons: vec![package_root.join("package.json")],
                ..SpecIdentityOptions::default()
            },
        )
        .unwrap();
        assert!(!report.valid, "{report:#?}");
        assert!(
            report
                .checks
                .iter()
                .any(|check| check.name == "artifact:contract-markdown.md" && !check.passed)
        );
        assert!(
            report
                .checks
                .iter()
                .any(|check| check.name == "package:@openprose/reactor@9.9.9" && !check.passed)
        );
    }

    #[test]
    fn rejects_artifact_root_outside_pinned_git_checkout() {
        let dir = tempdir().unwrap();
        let repo = dir.path().join("prose");
        let repo_skill_root = repo.join("skills/open-prose");
        let external_skill_root = dir.path().join("external/skills/open-prose");
        git(dir.path(), &["init", "prose"]);
        git(&repo, &["config", "user.email", "agent@example.invalid"]);
        git(&repo, &["config", "user.name", "Agent"]);

        write_skill(&repo_skill_root, "contract from repo\n");
        let commit = commit(&repo, "contract from repo");
        write_skill(&external_skill_root, "contract outside repo\n");
        let external_manifest = write_manifest_without_packages(&external_skill_root);

        let report = verify_spec_identity(
            &external_manifest,
            SpecIdentityOptions {
                root: Some(external_skill_root),
                git_repo: Some(repo),
                expected_repo: Some("openprose/prose".to_string()),
                expected_commit: Some(commit),
                package_jsons: vec![],
            },
        )
        .unwrap();
        assert!(!report.valid, "{report:#?}");
        assert!(
            report
                .checks
                .iter()
                .any(|check| check.name == "git.root" && !check.passed)
        );
    }

    #[test]
    fn rejects_wrong_expected_repo_identity() {
        let dir = tempdir().unwrap();
        let repo = dir.path().join("prose");
        let skill_root = repo.join("skills/open-prose");
        git(dir.path(), &["init", "prose"]);
        git(&repo, &["config", "user.email", "agent@example.invalid"]);
        git(&repo, &["config", "user.name", "Agent"]);

        write_skill(&skill_root, "contract\n");
        let manifest = write_manifest_without_packages(&skill_root);
        let commit = commit(&repo, "contract");

        let report = verify_spec_identity(
            &manifest,
            SpecIdentityOptions {
                root: Some(skill_root),
                git_repo: Some(repo),
                expected_repo: Some("wrong/prose".to_string()),
                expected_commit: Some(commit),
                package_jsons: vec![],
            },
        )
        .unwrap();
        assert!(!report.valid, "{report:#?}");
        assert!(
            report
                .checks
                .iter()
                .any(|check| check.name == "source.repo.expected" && !check.passed)
        );
    }

    #[test]
    fn rejects_expected_commit_without_git_repo() {
        let dir = tempdir().unwrap();
        let skill_root = dir.path().join("skills/open-prose");
        write_skill(&skill_root, "contract\n");
        let manifest = write_manifest_without_packages(&skill_root);

        let report = verify_spec_identity(
            &manifest,
            SpecIdentityOptions {
                root: Some(skill_root),
                expected_commit: Some("0123456789abcdef".to_string()),
                ..SpecIdentityOptions::default()
            },
        )
        .unwrap();
        assert!(!report.valid, "{report:#?}");
        assert!(
            report
                .checks
                .iter()
                .any(|check| check.name == "git.repo" && !check.passed)
        );
    }

    #[test]
    fn rejects_skill_document_metadata_mismatch() {
        let dir = tempdir().unwrap();
        let skill_root = dir.path().join("skills/open-prose");
        write_skill_with_metadata(&skill_root, "0.15.0", 2, "contract\n");
        let manifest = json!({
            "schema": "openprose.spec-identity",
            "schema_version": 1,
            "spec_id": "openprose",
            "source": {
                "repo": "openprose/prose"
            },
            "skill": {
                "version": "0.16.0",
                "runtime_contract": 1
            },
            "packages": {},
            "artifacts": {
                "SKILL.md": artifact_digest(&skill_root.join("SKILL.md")).unwrap(),
                "contract-markdown.md": artifact_digest(&skill_root.join("contract-markdown.md")).unwrap(),
                "prose.md": artifact_digest(&skill_root.join("prose.md")).unwrap(),
                "forme.md": artifact_digest(&skill_root.join("forme.md")).unwrap()
            }
        });
        let manifest_path = skill_root.join("spec-version.json");
        fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let report = verify_spec_identity(
            &manifest_path,
            SpecIdentityOptions {
                root: Some(skill_root),
                expected_repo: Some("openprose/prose".to_string()),
                ..SpecIdentityOptions::default()
            },
        )
        .unwrap();
        assert!(!report.valid, "{report:#?}");
        assert!(
            report
                .checks
                .iter()
                .any(|check| { check.name == "skill.version.document" && !check.passed })
        );
        assert!(
            report
                .checks
                .iter()
                .any(|check| { check.name == "skill.runtime_contract.document" && !check.passed })
        );
    }

    #[test]
    fn rejects_declared_package_without_matching_package_json() {
        let dir = tempdir().unwrap();
        let skill_root = dir.path().join("skill/open-prose");
        write_skill(&skill_root, "contract package\n");
        let manifest = write_manifest(&skill_root, "0.3.1");

        let report = verify_spec_identity(
            &manifest,
            SpecIdentityOptions {
                root: Some(skill_root),
                expected_repo: Some("openprose/prose".to_string()),
                ..SpecIdentityOptions::default()
            },
        )
        .unwrap();
        assert!(!report.valid, "{report:#?}");
        assert!(
            report
                .checks
                .iter()
                .any(|check| check.name == "package:@openprose/reactor" && !check.passed)
        );
    }

    #[test]
    fn verifies_expanded_language_surface_artifacts_for_reactor_transition() {
        let dir = tempdir().unwrap();
        let package_root = dir.path().join("node_modules/@openprose/reactor");
        let skill_root = package_root.join("skill/open-prose");
        write_skill(&skill_root, "contract markdown\n");
        fs::write(skill_root.join("prosescript.md"), "ProseScript\n").unwrap();
        fs::write(skill_root.join("reactor.md"), "Reactor\n").unwrap();
        fs::write(
            skill_root.join("responsibility-runtime.md"),
            "Responsibility Runtime\n",
        )
        .unwrap();
        fs::write(
            package_root.join("package.json"),
            serde_json::to_string_pretty(&json!({
                "name": "@openprose/reactor",
                "version": "0.3.1"
            }))
            .unwrap(),
        )
        .unwrap();

        let manifest = json!({
            "schema": "openprose.spec-identity",
            "schema_version": 1,
            "spec_id": "openprose",
            "source": {
                "repo": "openprose/prose"
            },
            "skill": {
                "version": "0.15.0",
                "runtime_contract": 2
            },
            "packages": {
                "@openprose/reactor": "0.3.1"
            },
            "artifacts": {
                "SKILL.md": artifact_digest(&skill_root.join("SKILL.md")).unwrap(),
                "contract-markdown.md": artifact_digest(&skill_root.join("contract-markdown.md")).unwrap(),
                "forme.md": artifact_digest(&skill_root.join("forme.md")).unwrap(),
                "prose.md": artifact_digest(&skill_root.join("prose.md")).unwrap(),
                "prosescript.md": artifact_digest(&skill_root.join("prosescript.md")).unwrap(),
                "reactor.md": artifact_digest(&skill_root.join("reactor.md")).unwrap(),
                "responsibility-runtime.md": artifact_digest(&skill_root.join("responsibility-runtime.md")).unwrap()
            }
        });
        let manifest_path = skill_root.join("spec-version.json");
        fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let report = verify_spec_identity(
            &manifest_path,
            SpecIdentityOptions {
                root: Some(skill_root),
                expected_repo: Some("openprose/prose".to_string()),
                package_jsons: vec![package_root.join("package.json")],
                ..SpecIdentityOptions::default()
            },
        )
        .unwrap();
        assert!(report.valid, "{report:#?}");
        for artifact in [
            "artifact:forme.md",
            "artifact:prosescript.md",
            "artifact:reactor.md",
            "artifact:responsibility-runtime.md",
        ] {
            assert!(
                report
                    .checks
                    .iter()
                    .any(|check| check.name == artifact && check.passed),
                "missing passed check for {artifact}: {report:#?}"
            );
        }
    }
}
