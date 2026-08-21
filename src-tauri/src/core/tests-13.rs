fn fake_github_cli(root: &Path, name: &str, stderr: &str) -> PathBuf {
    let executable = root.join(name);
    fs::write(
        &executable,
        format!("#!/bin/sh\nprintf '%s\\n' '{stderr}' >&2\nexit 1\n"),
    )
    .expect("fake GitHub CLI should be writable");
    let mut permissions = fs::metadata(&executable)
        .expect("fake GitHub CLI metadata should load")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&executable, permissions)
        .expect("fake GitHub CLI should be executable");
    executable
}

#[test]
fn github_cli_json_distinguishes_failure_causes_without_overclaiming_authentication() {
    let root = fixture_root();
    let cases = [
        (
            "network",
            "error connecting to api.github.com: check your internet connection",
            "GitHub provider unavailable: GitHub CLI could not reach GitHub; authentication was not verified. Check network access and retry.",
        ),
        (
            "authentication",
            "HTTP 401: Bad credentials",
            "GitHub provider unavailable: GitHub CLI authentication is invalid or expired. Run `gh auth status` and reauthenticate if needed.",
        ),
        (
            "unknown",
            "unexpected GitHub CLI failure",
            "GitHub provider unavailable: GitHub CLI request failed; authentication and network status could not be determined.",
        ),
    ];

    for (name, stderr, expected) in cases {
        let executable = fake_github_cli(&root, name, stderr);
        let adapter = GitHubCliAdapter {
            executable: executable.to_string_lossy().into_owned(),
            target_repository_names: None,
        };
        let error = adapter
            .json(&["api", "user"])
            .expect_err("a failing GitHub CLI should return a classified error");
        assert_eq!(error, expected);
    }

    fs::remove_dir_all(root).expect("GitHub CLI fixture should be removable");
}

#[test]
fn github_cli_json_reports_a_missing_executable_without_requesting_reauthentication() {
    let root = fixture_root();
    let adapter = GitHubCliAdapter {
        executable: root.join("missing-gh").to_string_lossy().into_owned(),
        target_repository_names: None,
    };

    let error = adapter
        .json(&["api", "user"])
        .expect_err("a missing GitHub CLI should return an availability error");
    assert_eq!(
        error,
        "GitHub provider unavailable: GitHub CLI is not installed or not on PATH."
    );

    fs::remove_dir_all(root).expect("GitHub CLI fixture should be removable");
}
