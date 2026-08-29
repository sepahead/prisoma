#!/usr/bin/env python3
"""Provider-free controls for generic ecosystem source provenance."""

from __future__ import annotations

import copy
import hashlib
import os
import subprocess
import tempfile
from pathlib import Path

from source_provenance import (
    EVIDENCE_PUBLICATION_POLICY,
    capture_evidence_publication,
    capture_repository_files,
    capture_repository_identity,
    verify_committed_source_roster,
)


EVIDENCE_DIRECTORY = Path("evidence/real-nest-3.9-v2")
EVIDENCE_PATHS = tuple(
    EVIDENCE_DIRECTORY / name
    for name in (
        "INDEX.json",
        "capture-1-drone.json",
        "capture-2-drones.json",
        "capture-3-drones.json",
    )
)


def git(root: Path, *arguments: str) -> str:
    completed = subprocess.run(
        ["git", *arguments],
        cwd=root,
        check=True,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env={
            **{
                key: value
                for key, value in os.environ.items()
                if not key.startswith("GIT_")
            },
            "LC_ALL": "C",
        },
    )
    return completed.stdout.decode("utf-8").strip()


def expect_rejected(label: str, action: object) -> None:
    try:
        action()  # type: ignore[operator]
    except (OSError, ValueError):
        return
    raise AssertionError(f"hostile provenance control was accepted: {label}")


def source_row(root: Path, relative: str) -> dict[str, object]:
    payload = root.joinpath(relative).read_bytes()
    tree = git(root, "ls-tree", "HEAD", "--", relative)
    metadata, observed_path = tree.split("\t", 1)
    mode, kind, blob = metadata.split(" ", 2)
    assert kind == "blob" and observed_path == relative
    return {
        "relative_path": relative,
        "size_bytes": len(payload),
        "sha256": hashlib.sha256(payload).hexdigest(),
        "git_mode": mode,
        "git_blob": blob,
    }


def publication_fixture(
    parent: Path,
    name: str,
    *,
    object_format: str = "sha1",
    variant: str = "exact",
) -> tuple[Path, str, str]:
    root = parent / name
    root.mkdir()
    init = ["init", "--initial-branch=main"]
    if object_format == "sha256":
        init.insert(1, "--object-format=sha256")
    git(root, *init)
    git(root, "config", "user.name", "Prisoma Test")
    git(root, "config", "user.email", "prisoma-test@example.invalid")
    git(root, "config", "core.filemode", "true")
    git(root, "remote", "add", "origin", f"https://example.invalid/{name}.git")
    (root / "src").mkdir()
    (root / "src/tool.py").write_text("VALUE = 1\n", encoding="utf-8")
    evidence = root / EVIDENCE_DIRECTORY
    if variant in {"modified", "deleted", "renamed"}:
        evidence.mkdir(parents=True)
    if variant == "modified":
        (root / EVIDENCE_PATHS[0]).write_text('{"old":true}\n', encoding="utf-8")
    elif variant == "deleted":
        (evidence / "obsolete.json").write_text("{}\n", encoding="utf-8")
    elif variant == "renamed":
        (evidence / "old-index.json").write_text("{}\n", encoding="utf-8")
    git(root, "add", "-A")
    git(root, "commit", "-m", "source")
    source_revision = git(root, "rev-parse", "HEAD")
    git(root, "update-ref", "refs/remotes/origin/main", source_revision)

    evidence.mkdir(parents=True, exist_ok=True)
    if variant == "deleted":
        (evidence / "obsolete.json").unlink()
    elif variant == "renamed":
        (evidence / "old-index.json").rename(root / EVIDENCE_PATHS[0])
    for ordinal, relative in enumerate(EVIDENCE_PATHS, start=1):
        path = root / relative
        if variant == "symlink" and ordinal == 1:
            path.symlink_to("../../src/tool.py")
        else:
            path.write_text(f'{{"ordinal":{ordinal}}}\n', encoding="utf-8")
    if variant == "executable":
        (root / EVIDENCE_PATHS[0]).chmod(0o755)
    if variant == "extra-delta":
        (root / "unrelated.txt").write_text("unrelated\n", encoding="utf-8")
    git(root, "add", "-A")
    git(root, "commit", "-m", "evidence publication")
    publication_revision = git(root, "rev-parse", "HEAD")
    if variant == "merge":
        source_tree = git(root, "rev-parse", f"{source_revision}^{{tree}}")
        side = git(
            root,
            "commit-tree",
            source_tree,
            "-p",
            source_revision,
            "-m",
            "side",
        )
        publication_tree = git(
            root,
            "rev-parse",
            f"{publication_revision}^{{tree}}",
        )
        publication_revision = git(
            root,
            "commit-tree",
            publication_tree,
            "-p",
            source_revision,
            "-p",
            side,
            "-m",
            "merge publication",
        )
        git(root, "reset", "--hard", publication_revision)
    git(root, "update-ref", "refs/remotes/origin/main", publication_revision)
    return root, source_revision, publication_revision


def exercise_evidence_publication(parent: Path) -> None:
    root, source_revision, publication_revision = publication_fixture(
        parent,
        "publication-sha1",
    )
    observed = capture_evidence_publication(
        root,
        source_revision,
        publication_revision,
        EVIDENCE_PATHS,
        1024,
    )
    if (
        observed["source"]["commit"] != source_revision
        or observed["publication"]["commit"] != publication_revision
        or observed["publication"]["parent_commit"] != source_revision
        or observed["publication"]["policy"] != EVIDENCE_PUBLICATION_POLICY
        or observed["publication"]["file_count"] != 4
        or [row["path"] for row in observed["publication"]["files"]]
        != [path.as_posix() for path in EVIDENCE_PATHS]
    ):
        raise AssertionError("positive evidence publication differs")
    historical = capture_repository_files(
        root,
        source_revision,
        (Path("src/tool.py"),),
        1024,
        checkout_revision=publication_revision,
    )
    historical_rows = [source_row(root, "src/tool.py")]
    verify_committed_source_roster(
        root,
        source_revision,
        historical_rows,
        checkout_revision=publication_revision,
    )
    if historical[0]["path"] != "src/tool.py":
        raise AssertionError("historical source capture differs")

    expect_rejected(
        "identical source and publication revisions",
        lambda: capture_evidence_publication(
            root,
            publication_revision,
            publication_revision,
            EVIDENCE_PATHS,
            1024,
        ),
    )
    unrelated = git(
        root,
        "commit-tree",
        f"{source_revision}^{{tree}}",
        "-m",
        "unrelated source",
    )
    expect_rejected(
        "wrong publication parent",
        lambda: capture_evidence_publication(
            root,
            unrelated,
            publication_revision,
            EVIDENCE_PATHS,
            1024,
        ),
    )
    expect_rejected(
        "unordered evidence roster",
        lambda: capture_evidence_publication(
            root,
            source_revision,
            publication_revision,
            tuple(reversed(EVIDENCE_PATHS)),
            1024,
        ),
    )

    for variant, label in (
        ("merge", "merge publication"),
        ("extra-delta", "extra publication delta"),
        ("modified", "modified evidence path"),
        ("deleted", "deleted evidence path"),
        ("renamed", "renamed evidence path"),
        ("executable", "executable evidence mode"),
        ("symlink", "symlink evidence entry"),
    ):
        hostile_root, hostile_source, hostile_publication = publication_fixture(
            parent,
            f"publication-{variant}",
            variant=variant,
        )
        expect_rejected(
            label,
            lambda root=hostile_root,
            source=hostile_source,
            publication=hostile_publication: (
                capture_evidence_publication(
                    root,
                    source,
                    publication,
                    EVIDENCE_PATHS,
                    1024,
                )
            ),
        )

    ignored_root, ignored_source, ignored_publication = publication_fixture(
        parent,
        "publication-ignored-extra",
    )
    ignored_extra = ignored_root / EVIDENCE_DIRECTORY / "ignored.json"
    ignored_extra.write_text("{}\n", encoding="utf-8")
    (ignored_root / ".git/info/exclude").write_text(
        f"/{ignored_extra.relative_to(ignored_root).as_posix()}\n",
        encoding="utf-8",
    )
    expect_rejected(
        "ignored extra evidence file",
        lambda: capture_evidence_publication(
            ignored_root,
            ignored_source,
            ignored_publication,
            EVIDENCE_PATHS,
            1024,
        ),
    )

    dirty_root, dirty_source, dirty_publication = publication_fixture(
        parent,
        "publication-dirty",
    )
    (dirty_root / EVIDENCE_PATHS[0]).write_text('{"dirty":true}\n', encoding="utf-8")
    expect_rejected(
        "dirty evidence bytes",
        lambda: capture_evidence_publication(
            dirty_root,
            dirty_source,
            dirty_publication,
            EVIDENCE_PATHS,
            1024,
        ),
    )

    missing_root, missing_source, missing_publication = publication_fixture(
        parent,
        "publication-missing",
    )
    (missing_root / EVIDENCE_PATHS[0]).unlink()
    expect_rejected(
        "missing evidence file",
        lambda: capture_evidence_publication(
            missing_root,
            missing_source,
            missing_publication,
            EVIDENCE_PATHS,
            1024,
        ),
    )

    drift_root, drift_source, drift_publication = publication_fixture(
        parent,
        "publication-origin-drift",
    )
    git(drift_root, "update-ref", "refs/remotes/origin/main", drift_source)
    expect_rejected(
        "publication origin-main drift",
        lambda: capture_evidence_publication(
            drift_root,
            drift_source,
            drift_publication,
            EVIDENCE_PATHS,
            1024,
        ),
    )

    worktree_root, worktree_source, worktree_publication = publication_fixture(
        parent,
        "publication-core-worktree",
    )
    redirected_worktree = parent / "redirected-core-worktree"
    redirected_worktree.mkdir()
    git(worktree_root, "config", "core.worktree", os.fspath(redirected_worktree))
    expect_rejected(
        "redirected core.worktree",
        lambda: capture_evidence_publication(
            worktree_root,
            worktree_source,
            worktree_publication,
            EVIDENCE_PATHS,
            1024,
        ),
    )

    assume_root, assume_source, assume_publication = publication_fixture(
        parent,
        "publication-assume-unchanged",
    )
    git(
        assume_root,
        "update-index",
        "--assume-unchanged",
        EVIDENCE_PATHS[0].as_posix(),
    )
    expect_rejected(
        "assume-unchanged index flag",
        lambda: capture_evidence_publication(
            assume_root,
            assume_source,
            assume_publication,
            EVIDENCE_PATHS,
            1024,
        ),
    )

    skip_root, skip_source, skip_publication = publication_fixture(
        parent,
        "publication-skip-worktree",
    )
    git(
        skip_root,
        "update-index",
        "--skip-worktree",
        EVIDENCE_PATHS[0].as_posix(),
    )
    expect_rejected(
        "skip-worktree index flag",
        lambda: capture_evidence_publication(
            skip_root,
            skip_source,
            skip_publication,
            EVIDENCE_PATHS,
            1024,
        ),
    )

    graft_root, graft_source, graft_publication = publication_fixture(
        parent,
        "publication-graft",
    )
    source_tree = git(graft_root, "rev-parse", f"{graft_source}^{{tree}}")
    unrelated_parent = git(
        graft_root,
        "commit-tree",
        source_tree,
        "-m",
        "unrelated literal parent",
    )
    publication_tree = git(
        graft_root,
        "rev-parse",
        f"{graft_publication}^{{tree}}",
    )
    graft_publication = git(
        graft_root,
        "commit-tree",
        publication_tree,
        "-p",
        unrelated_parent,
        "-m",
        "non-child publication",
    )
    git(graft_root, "reset", "--hard", graft_publication)
    git(
        graft_root,
        "update-ref",
        "refs/remotes/origin/main",
        graft_publication,
    )
    (graft_root / ".git/info/grafts").write_text(
        f"{graft_publication} {graft_source}\n",
        encoding="ascii",
    )
    expect_rejected(
        "grafted direct parent",
        lambda: capture_evidence_publication(
            graft_root,
            graft_source,
            graft_publication,
            EVIDENCE_PATHS,
            1024,
        ),
    )

    shallow_root, shallow_source, shallow_publication = publication_fixture(
        parent,
        "publication-shallow",
    )
    (shallow_root / ".git/shallow").write_text(
        f"{shallow_source}\n",
        encoding="ascii",
    )
    expect_rejected(
        "shallow publication history",
        lambda: capture_evidence_publication(
            shallow_root,
            shallow_source,
            shallow_publication,
            EVIDENCE_PATHS,
            1024,
        ),
    )

    replace_root, replace_source, replace_publication = publication_fixture(
        parent,
        "publication-replacement",
    )
    replacement = git(
        replace_root,
        "commit-tree",
        f"{replace_publication}^{{tree}}",
        "-p",
        replace_source,
        "-m",
        "replacement publication",
    )
    git(replace_root, "replace", replace_publication, replacement)
    expect_rejected(
        "replacement reference",
        lambda: capture_evidence_publication(
            replace_root,
            replace_source,
            replace_publication,
            EVIDENCE_PATHS,
            1024,
        ),
    )

    writable_file_root, writable_file_source, writable_file_publication = (
        publication_fixture(parent, "publication-group-writable-file")
    )
    (writable_file_root / EVIDENCE_PATHS[0]).chmod(0o664)
    expect_rejected(
        "group-writable publication file",
        lambda: capture_evidence_publication(
            writable_file_root,
            writable_file_source,
            writable_file_publication,
            EVIDENCE_PATHS,
            1024,
        ),
    )

    executable_drift_root, executable_drift_source, executable_drift_publication = (
        publication_fixture(parent, "publication-executable-worktree-drift")
    )
    git(executable_drift_root, "config", "core.filemode", "false")
    (executable_drift_root / EVIDENCE_PATHS[0]).chmod(0o755)
    expect_rejected(
        "worktree executable mode hidden by core.filemode=false",
        lambda: capture_evidence_publication(
            executable_drift_root,
            executable_drift_source,
            executable_drift_publication,
            EVIDENCE_PATHS,
            1024,
        ),
    )

    hardlink_root, hardlink_source, hardlink_publication = publication_fixture(
        parent,
        "publication-hardlink",
    )
    os.link(hardlink_root / EVIDENCE_PATHS[0], hardlink_root / ".git/evidence-link")
    expect_rejected(
        "hard-linked publication file",
        lambda: capture_evidence_publication(
            hardlink_root,
            hardlink_source,
            hardlink_publication,
            EVIDENCE_PATHS,
            1024,
        ),
    )

    writable_dir_root, writable_dir_source, writable_dir_publication = (
        publication_fixture(parent, "publication-group-writable-directory")
    )
    (writable_dir_root / EVIDENCE_DIRECTORY).chmod(0o775)
    expect_rejected(
        "group-writable publication directory",
        lambda: capture_evidence_publication(
            writable_dir_root,
            writable_dir_source,
            writable_dir_publication,
            EVIDENCE_PATHS,
            1024,
        ),
    )

    linked_dir_root, linked_dir_source, linked_dir_publication = publication_fixture(
        parent,
        "publication-linked-directory",
    )
    linked_directory = linked_dir_root / EVIDENCE_DIRECTORY
    moved_directory = parent / "publication-linked-directory-target"
    linked_directory.rename(moved_directory)
    linked_directory.symlink_to(moved_directory, target_is_directory=True)
    expect_rejected(
        "linked publication directory",
        lambda: capture_evidence_publication(
            linked_dir_root,
            linked_dir_source,
            linked_dir_publication,
            EVIDENCE_PATHS,
            1024,
        ),
    )

    marker_root, marker_source, marker_publication = publication_fixture(
        parent,
        "publication-linked-git-marker",
    )
    moved_git = parent / "publication-linked-git-marker-admin"
    (marker_root / ".git").rename(moved_git)
    (marker_root / ".git").symlink_to(moved_git, target_is_directory=True)
    expect_rejected(
        "linked Git worktree marker",
        lambda: capture_evidence_publication(
            marker_root,
            marker_source,
            marker_publication,
            EVIDENCE_PATHS,
            1024,
        ),
    )

    sha256_root, sha256_source, sha256_publication = publication_fixture(
        parent,
        "publication-sha256",
        object_format="sha256",
    )
    sha256_observed = capture_evidence_publication(
        sha256_root,
        sha256_source,
        sha256_publication,
        EVIDENCE_PATHS,
        1024,
    )
    if (
        sha256_observed["source"]["object_format"] != "sha256"
        or len(sha256_observed["source"]["commit"]) != 64
        or any(
            len(row["git_blob"]) != 64
            for row in sha256_observed["publication"]["files"]
        )
    ):
        raise AssertionError("SHA-256 evidence publication differs")


def main() -> int:
    with tempfile.TemporaryDirectory(prefix="prisoma-source-provenance-") as raw:
        root = Path(raw).resolve(strict=True)
        git(root, "init", "--initial-branch=main")
        git(root, "config", "user.name", "Prisoma Test")
        git(root, "config", "user.email", "prisoma-test@example.invalid")
        git(root, "remote", "add", "origin", "https://example.invalid/ecosystem.git")
        (root / "src").mkdir()
        (root / "src" / "alpha.py").write_text("ALPHA = 1\n", encoding="utf-8")
        (root / "src" / "beta.py").write_text("BETA = 2\n", encoding="utf-8")
        (root / "src" / "empty.py").write_bytes(b"")
        git(root, "add", "src/alpha.py", "src/beta.py", "src/empty.py")
        git(root, "commit", "-m", "fixture")
        revision = git(root, "rev-parse", "HEAD")
        git(root, "update-ref", "refs/remotes/origin/main", revision)
        identity = capture_repository_identity(root, revision)
        captured = capture_repository_files(
            root,
            revision,
            (Path("src/alpha.py"), Path("src/beta.py"), Path("src/empty.py")),
            1024,
        )
        rows = [
            source_row(root, path)
            for path in ("src/alpha.py", "src/beta.py", "src/empty.py")
        ]
        verify_committed_source_roster(root, revision, rows, allow_empty=True)
        mode_less_rows = [
            {
                "path": row["relative_path"],
                "byte_count": row["size_bytes"],
                "sha256": row["sha256"],
                "git_blob": row["git_blob"],
                "module_names": [f"fixture.{Path(str(row['relative_path'])).stem}"],
            }
            for row in rows
        ]
        verify_committed_source_roster(
            root,
            revision,
            mode_less_rows,
            path_field="path",
            size_field="byte_count",
            mode_field=None,
            allow_empty=True,
        )
        if (
            identity["commit"] != revision
            or identity["origin_main"] != revision
            or identity["clean"] is not True
            or [row["path"] for row in captured]
            != ["src/alpha.py", "src/beta.py", "src/empty.py"]
            or captured[2]["byte_count"] != 0
            or mode_less_rows[2]["sha256"] != hashlib.sha256(b"").hexdigest()
            or mode_less_rows[2]["git_blob"]
            != "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391"
        ):
            raise AssertionError("positive repository identity differs")

        missing_mode_less = copy.deepcopy(mode_less_rows)
        missing_mode_less[2]["path"] = "src/missing.py"
        expect_rejected(
            "mode-less roster missing committed source",
            lambda: verify_committed_source_roster(
                root,
                revision,
                missing_mode_less,
                path_field="path",
                size_field="byte_count",
                mode_field=None,
                allow_empty=True,
            ),
        )
        nonempty_mode_less = copy.deepcopy(mode_less_rows)
        nonempty_mode_less[2]["byte_count"] = 1
        expect_rejected(
            "mode-less zero-byte source reported as nonempty",
            lambda: verify_committed_source_roster(
                root,
                revision,
                nonempty_mode_less,
                path_field="path",
                size_field="byte_count",
                mode_field=None,
                allow_empty=True,
            ),
        )

        forged = copy.deepcopy(rows)
        forged[0]["sha256"] = "f" * 64
        expect_rejected(
            "forged source digest",
            lambda: verify_committed_source_roster(root, revision, forged),
        )
        swapped = copy.deepcopy(rows)
        swapped[0]["relative_path"], swapped[1]["relative_path"] = (
            swapped[1]["relative_path"],
            swapped[0]["relative_path"],
        )
        swapped.sort(key=lambda row: str(row["relative_path"]))
        expect_rejected(
            "swapped individually valid source rows",
            lambda: verify_committed_source_roster(root, revision, swapped),
        )
        traversal = copy.deepcopy(rows)
        traversal[0]["relative_path"] = "../alpha.py"
        expect_rejected(
            "source path traversal",
            lambda: verify_committed_source_roster(root, revision, traversal),
        )
        expect_rejected(
            "unordered source capture",
            lambda: capture_repository_files(
                root,
                revision,
                (Path("src/beta.py"), Path("src/alpha.py")),
                1024,
            ),
        )

        (root / "untracked.txt").write_text("untracked\n", encoding="utf-8")
        expect_rejected(
            "untracked source state",
            lambda: capture_repository_identity(root, revision),
        )
        (root / "untracked.txt").unlink()
        (root / "src" / "alpha.py").write_text("ALPHA = 9\n", encoding="utf-8")
        expect_rejected(
            "dirty tracked source state",
            lambda: capture_repository_identity(root, revision),
        )
        (root / "src" / "alpha.py").write_text("ALPHA = 1\n", encoding="utf-8")
        other_revision = git(
            root,
            "commit-tree",
            f"{revision}^{{tree}}",
            "-p",
            revision,
            "-m",
            "other",
        )
        git(root, "update-ref", "refs/remotes/origin/main", other_revision)
        expect_rejected(
            "origin main drift",
            lambda: capture_repository_identity(root, revision),
        )
        git(root, "update-ref", "-d", "refs/remotes/origin/main")
        expect_rejected(
            "missing origin main",
            lambda: capture_repository_identity(root, revision),
        )

        sha256_root = root / "sha256-repository"
        sha256_root.mkdir()
        git(sha256_root, "init", "--object-format=sha256", "--initial-branch=main")
        git(sha256_root, "config", "user.name", "Prisoma Test")
        git(sha256_root, "config", "user.email", "prisoma-test@example.invalid")
        git(
            sha256_root,
            "remote",
            "add",
            "origin",
            "https://example.invalid/ecosystem-sha256.git",
        )
        (sha256_root / "source.py").write_text("VALUE = 1\n", encoding="utf-8")
        git(sha256_root, "add", "source.py")
        git(sha256_root, "commit", "-m", "sha256 fixture")
        sha256_revision = git(sha256_root, "rev-parse", "HEAD")
        git(
            sha256_root,
            "update-ref",
            "refs/remotes/origin/main",
            sha256_revision,
        )
        sha256_identity = capture_repository_identity(
            sha256_root,
            sha256_revision,
        )
        sha256_rows = [source_row(sha256_root, "source.py")]
        verify_committed_source_roster(sha256_root, sha256_revision, sha256_rows)
        if (
            sha256_identity["object_format"] != "sha256"
            or len(sha256_identity["commit"]) != 64
            or len(sha256_rows[0]["git_blob"]) != 64
        ):
            raise AssertionError("SHA-256 repository identity differs")
        mismatched_format = copy.deepcopy(sha256_rows)
        mismatched_format[0]["git_blob"] = mismatched_format[0]["git_blob"][:40]
        expect_rejected(
            "SHA-256 repository with SHA-1 blob",
            lambda: verify_committed_source_roster(
                sha256_root,
                sha256_revision,
                mismatched_format,
            ),
        )

        exercise_evidence_publication(root)

    print("OK: generic source provenance accepted SHA-1/SHA-256 and hostile controls")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
