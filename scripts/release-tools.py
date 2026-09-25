#!/usr/bin/env python3
"""Small release helpers. No external Python dependencies; Python 3.11+ required."""
from __future__ import annotations
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[1]
SEMVER = re.compile(r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(?:-(alpha|beta|rc)\.(0|[1-9][0-9]*))?")


def run(*args: str, capture: bool = False) -> str:
    p = subprocess.run(args, cwd=ROOT, check=True, text=True,
                       stdout=subprocess.PIPE if capture else None)
    return p.stdout.strip() if capture else ""


def parse_tag(tag: str) -> str:
    if not tag.startswith("v") or not SEMVER.fullmatch(tag[1:]):
        raise ValueError("Tag must be vMAJOR.MINOR.PATCH or vMAJOR.MINOR.PATCH-{alpha,beta,rc}.N.")
    return tag[1:]


def version() -> str:
    cargo = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    native = json.loads((ROOT / "src-tauri/tauri.conf.json").read_text(encoding="utf-8"))
    v = cargo["workspace"]["package"]["version"]
    if not SEMVER.fullmatch(v) or native["version"] != v:
        raise ValueError("Workspace and Tauri versions must be the same supported SemVer.")
    return v


def require_env(names: list[str]) -> None:
    missing = [name for name in names if not os.environ.get(name, "").strip()]
    if missing:
        raise ValueError("Missing configuration: " + ", ".join(missing))


def require_lock() -> None:
    lock = ROOT / "Cargo.lock"
    if not lock.exists():
        raise ValueError("Cargo.lock is missing. Run Bootstrap dependency lock, review and commit it first.")
    if not tomllib.loads(lock.read_text(encoding="utf-8")).get("package"):
        raise ValueError("Cargo.lock is empty or not a resolved Cargo lockfile.")
    run("git", "ls-files", "--error-unmatch", "Cargo.lock", capture=True)


def pin_rust() -> None:
    output = run("rustc", "--version", capture=True)
    match = re.match(r"rustc (1\.[0-9]+\.[0-9]+) ", output)
    if not match:
        raise ValueError("An exact stable Rust release is required, not a nightly/dev toolchain.")
    path = ROOT / "rust-toolchain.toml"
    text, count = re.subn(r'^channel\s*=\s*"[^"\n]+"', 'channel = "' + match[1] + '"', path.read_text(encoding="utf-8"), flags=re.M)
    if count != 1:
        raise ValueError("Expected one toolchain channel.")
    path.write_text(text, encoding="utf-8")
    print("Pinned Rust", match[1])


def require_release_pins() -> None:
    channel = tomllib.loads((ROOT / "rust-toolchain.toml").read_text(encoding="utf-8"))["toolchain"]["channel"]
    if not re.fullmatch(r"1\.[0-9]+\.[0-9]+", channel):
        raise ValueError("Pin rust-toolchain.toml to an exact stable version before releasing.")
    run(sys.executable, "scripts/pin-actions.py", "--check")


def release_policy() -> dict[str, str]:
    policy = json.loads((ROOT / ".github/release-policy.json").read_text(encoding="utf-8"))
    if (not isinstance(policy, dict) or set(policy) != {"repository", "visibility"}
            or not isinstance(policy["repository"], str)
            or not re.fullmatch(r"[A-Za-z0-9_.-]+/MPD_Viewer", policy["repository"])
            or policy["visibility"] not in ("public", "private")):
        raise ValueError("Invalid committed release repository policy.")
    return policy


def require_repository_policy(repository: str, private: bool) -> None:
    policy = release_policy()
    if repository != policy["repository"]:
        raise ValueError("Repository identity differs from the committed release policy.")
    if not isinstance(private, bool) or private != (policy["visibility"] == "private"):
        raise ValueError("Repository visibility differs from the committed release policy.")


def release_scope(v: str) -> str:
    policy = json.loads((ROOT / ".github/release-scope.json").read_text(encoding="utf-8"))
    alpha_scopes = ("windows-alpha", "multiplatform-alpha")
    if not isinstance(policy, dict) or set(policy) != {"scope"} or policy["scope"] not in ("full", *alpha_scopes):
        raise ValueError("Invalid committed release scope.")
    scope = policy["scope"]
    if scope in alpha_scopes and (not SEMVER.fullmatch(v) or "-alpha." not in v):
        raise ValueError("Reduced-signing scopes are permitted only for an alpha prerelease.")
    return scope


def release_assets(v: str, scope: str) -> list[str]:
    names = asset_names(v)
    if scope == "full":
        return names
    if scope == "windows-alpha" and SEMVER.fullmatch(v) and "-alpha." in v:
        return [name for name in names if name.endswith(("-Windows-x64.zip", "-source.zip", "-player-sources.zip"))]
    if scope == "multiplatform-alpha" and SEMVER.fullmatch(v) and "-alpha." in v:
        return multiplatform_alpha_asset_names(v)
    raise ValueError("Unsupported release asset scope.")


def gate() -> None:
    if os.environ.get("RELEASES_ENABLED") != "true":
        raise ValueError("Releases are disabled. Complete repository/signing setup before enabling them.")
    private = os.environ.get("REPOSITORY_PRIVATE", "").lower()
    if private not in ("true", "false"):
        raise ValueError("Repository visibility is missing or invalid.")
    require_repository_policy(os.environ.get("GITHUB_REPOSITORY", ""), private == "true")
    if os.environ.get("GITHUB_EVENT_NAME") != "push" or os.environ.get("GITHUB_REF_TYPE") != "tag":
        raise ValueError("Signing is only permitted for a pushed release tag.")
    tag = os.environ["RELEASE_TAG"]
    v = parse_tag(tag)
    if v != version():
        raise ValueError("Tag, Cargo workspace version and Tauri version differ.")
    if "-" not in v and os.environ.get("STABLE_RELEASES_ENABLED") != "true":
        raise ValueError("Stable releases need a separate explicit enable after native acceptance.")
    scope = release_scope(v)
    require_lock()
    require_release_pins()
    ref = "refs/tags/" + tag
    if run("git", "cat-file", "-t", ref, capture=True) != "tag":
        raise ValueError("Use an annotated release tag, not a lightweight tag.")
    head = run("git", "rev-parse", "HEAD", capture=True)
    if run("git", "rev-parse", ref + "^{commit}", capture=True) != head:
        raise ValueError("Tag does not identify the checked-out source.")
    run("git", "merge-base", "--is-ancestor", "HEAD", "refs/remotes/origin/main")
    if path := os.environ.get("GITHUB_OUTPUT"):
        with open(path, "a", encoding="utf-8") as f:
            f.write(f"version={v}\nscope={scope}\n")
    print("Release metadata gate passed:", tag)


def digest(path: Path) -> str:
    with path.open("rb") as f:
        return hashlib.file_digest(f, "sha256").hexdigest()


def final_dir() -> Path:
    p = ROOT / ".release-assets/final"
    p.mkdir(parents=True, exist_ok=True)
    return p


def asset_names(v: str) -> list[str]:
    return [f"MPD_Viewer-v{v}-Windows-x64.zip", f"MPD_Viewer-v{v}-macOS-arm64.dmg",
            f"MPD_Viewer-v{v}-macOS-x64.dmg", f"MPD_Viewer-v{v}-Linux-x64.deb",
            f"MPD_Viewer-v{v}-Linux-x64.AppImage", f"MPD_Viewer-v{v}-source.zip",
            f"MPD_Viewer-v{v}-player-sources.zip"]


def multiplatform_alpha_asset_names(v: str) -> list[str]:
    return [f"MPD_Viewer-v{v}-Windows-x64.zip",
            f"MPD_Viewer-v{v}-macOS-arm64-UNSIGNED.zip",
            f"MPD_Viewer-v{v}-macOS-x64-UNSIGNED.zip",
            f"MPD_Viewer-v{v}-Linux-x64-UNSIGNED.deb",
            f"MPD_Viewer-v{v}-Linux-x64-UNSIGNED.AppImage",
            f"MPD_Viewer-v{v}-source.zip",
            f"MPD_Viewer-v{v}-player-sources.zip"]


def env_version() -> str:
    v = os.environ["RELEASE_VERSION"]
    if v != version():
        raise ValueError("Release version disagrees with checked-out source.")
    return v


def stage_ci(platform: str) -> None:
    allowed = {"Windows-x64", "macOS-arm64", "macOS-x64", "Linux-x64"}
    if platform not in allowed:
        raise ValueError("Unexpected platform.")
    ext = ".exe" if platform.startswith("Windows") else ""
    dest = ROOT / ".release-assets/ci"
    dest.mkdir(parents=True, exist_ok=True)
    exe = ROOT / ("target/release/mpd-tabber" + ext)
    if not exe.is_file():
        raise ValueError("Expected native binary was not built.")
    # ZIP stores mode bits; these binaries are diagnostics, NOT macOS/Linux installers.
    with zipfile.ZipFile(dest / ("MPD_Viewer-" + platform + "-UNSIGNED.zip"), "w", zipfile.ZIP_DEFLATED) as z:
        z.write(exe, "MPD_Viewer" + ext)
        z.writestr("READ-ME-FIRST.txt", "UNSIGNED developer diagnostic binary, not a verified release.\n"
                   "macOS needs an application bundle; Linux needs native system libraries.\n"
                   "Twitch/webview authentication and hosted protocol integration remain unverified.\n")


def package_windows() -> None:
    v = env_version()
    exe = ROOT / ".release-work/windows/mpd-tabber.exe"
    if not exe.is_file():
        raise ValueError("Signed Windows binary missing.")
    with zipfile.ZipFile(final_dir() / asset_names(v)[0], "w", zipfile.ZIP_DEFLATED) as z:
        z.write(exe, "MPD_Viewer.exe")
        z.write(ROOT / "LICENSE", "LICENSE")
        z.writestr("README.txt", "MPD Viewer " + v + "\nWindows x64. Requires Microsoft Edge WebView2 Runtime.\n"
                   "This is a ZIP distribution, not an installer. Preferences remain in the existing app data directory.\n"
                   "POC: see release notes for current hosted-player and runtime limitations.\n")


def package_linux() -> None:
    v = env_version()
    scope = release_scope(v)
    if scope == "full":
        names = asset_names(v)
    elif scope == "multiplatform-alpha":
        names = multiplatform_alpha_asset_names(v)
    else:
        raise ValueError("Linux release packages are not permitted by the committed scope.")
    bundle = ROOT / "target/release/bundle"
    for suffix, name in [(".deb", names[3]), (".AppImage", names[4])]:
        candidates = list(bundle.rglob("*" + suffix))
        if len(candidates) != 1 or not candidates[0].is_file():
            raise ValueError("Expected exactly one Linux package with suffix " + suffix)
        shutil.copy2(candidates[0], final_dir() / name)


def package_macos_unsigned(platform: str) -> None:
    v = env_version()
    if sys.platform != "darwin":
        raise ValueError("Unsigned macOS app packaging requires a macOS runner.")
    if release_scope(v) != "multiplatform-alpha" or platform not in ("macOS-arm64", "macOS-x64"):
        raise ValueError("Unsigned macOS app ZIPs are permitted only by the multiplatform alpha scope.")
    apps = list((ROOT / "target/release/bundle/macos").glob("*.app"))
    if len(apps) != 1 or not apps[0].is_dir():
        raise ValueError("Expected exactly one macOS app bundle.")
    name = multiplatform_alpha_asset_names(v)[1 if platform == "macOS-arm64" else 2]
    run("ditto", "-c", "-k", "--keepParent", "--sequesterRsrc", str(apps[0]), str(final_dir() / name))


def gh_json(*args: str) -> object:
    return json.loads(run("gh", *args, capture=True))


def live_commit(repo: str, tag: str) -> str:
    obj = gh_json("api", f"repos/{repo}/git/ref/tags/{tag}")["object"]
    for _ in range(6):
        if obj["type"] == "commit":
            return obj["sha"]
        if obj["type"] != "tag":
            raise ValueError("Unexpected release tag target.")
        obj = gh_json("api", f"repos/{repo}/git/tags/{obj['sha']}")["object"]
    raise ValueError("Too many annotated tag indirections.")


def publish() -> None:
    v = env_version()
    tag = os.environ["RELEASE_TAG"]
    if parse_tag(tag) != v:
        raise ValueError("Tag/version mismatch.")
    repo = os.environ["GH_REPO"]
    if repo != release_policy()["repository"]:
        raise ValueError("Repository identity differs from the committed release policy.")
    head = run("git", "rev-parse", "HEAD", capture=True)
    remote = gh_json("api", f"repos/{repo}")
    require_repository_policy(remote["full_name"], remote["private"])
    if live_commit(repo, tag) != head:
        raise ValueError("Live tag target has changed.")
    out = final_dir()
    scope = release_scope(v)
    names = release_assets(v, scope)
    source_name = f"MPD_Viewer-v{v}-source.zip"
    player_source_name = f"MPD_Viewer-v{v}-player-sources.zip"
    binaries = [name for name in names if name not in (source_name, player_source_name)]
    actual = sorted(p.name for p in out.iterdir())
    if actual != sorted(binaries):
        raise ValueError("Release asset set incomplete or unexpected: " + repr(actual))
    if any((out / p).is_symlink() or (out / p).stat().st_size == 0 for p in binaries):
        raise ValueError("Empty or symlink release asset.")
    # Never overwrite an earlier (even draft) release on a rerun.
    releases = gh_json("api", "--paginate", "--slurp", f"repos/{repo}/releases?per_page=100")
    if any(r["tag_name"] == tag for page in releases for r in page):
        raise ValueError("A release already exists. Inspect it; this workflow never overwrites release assets.")
    run("git", "archive", "--format=zip", f"--prefix=MPD_Viewer-{tag}/",
        "--output=" + str(out / source_name), "HEAD")
    run("git", "archive", "--format=zip", "--output=" + str(out / player_source_name), "HEAD",
        "player-wrapper", "web/parent.mpdviewer.com", "src-tauri/player-origin.json",
        "docs/HOSTED-SOURCE-PLAN-ADDENDUM.md")
    files = [out / name for name in names]
    signing = {"windows": "Authenticode + timestamp verified before ZIP"}
    if scope == "full":
        signing.update({"macos": "Developer ID + notarization + stapling checked",
                        "linux": "No OS-native signature; SHA-256 checksums only"})
    elif scope == "multiplatform-alpha":
        signing.update({"macos": "UNSIGNED and not notarized; manual alpha testing only",
                        "linux": "No OS-native signature; SHA-256 checksums only"})
    manifest = {"schema": 2, "scope": scope, "repository": repo, "tag": tag, "commit": head,
                "repository_visibility": release_policy()["visibility"],
                "run_id": os.environ.get("GITHUB_RUN_ID"), "run_attempt": os.environ.get("GITHUB_RUN_ATTEMPT"),
                "cargo_lock_sha256": digest(ROOT / "Cargo.lock"),
                "assets": {p.name: {"sha256": digest(p), "bytes": p.stat().st_size} for p in files},
                "signing": signing}
    meta = out / "BUILD-METADATA.json"
    meta.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    files.append(meta)
    sums = out / "SHA256SUMS.txt"
    sums.write_text("".join(f"{digest(p)}  {p.name}\n" for p in sorted(files)), encoding="utf-8")
    files.append(sums)
    notes = ROOT / ".release-work/notes.md"
    notes.parent.mkdir(parents=True, exist_ok=True)
    if scope == "windows-alpha":
        platforms = ("Windows-only alpha: signed x64 ZIP, requires Microsoft Edge WebView2 Runtime. "
                     "macOS and Linux binaries are not included.")
    elif scope == "multiplatform-alpha":
        platforms = ("Manual-test alpha for all three desktop families. Windows: signed x64 ZIP. "
                     "macOS: UNSIGNED and not notarized Apple Silicon and Intel app ZIPs. "
                     "Linux: UNSIGNED x64 DEB/AppImage with SHA-256 checksums. "
                     "The macOS and Linux filenames and metadata intentionally identify their trust state.")
        install_notes = ("For macOS, extract the app ZIP; because it has no Developer ID signature or "
                         "notarization, expect Gatekeeper to warn or block it. Use it only on a test system "
                         "and record the exact result. Linux testers can use the DEB or AppImage after "
                         "verifying SHA256SUMS.txt.\n\n")
    else:
        platforms = ("Windows: signed x64 ZIP. macOS: signed/notarized Apple Silicon and Intel DMGs. "
                     "Linux: x64 DEB/AppImage with checksums, not OS-native signed.")
    if scope != "multiplatform-alpha":
        install_notes = ""
    notes.write_text(f"MPD Viewer `{tag}` from `{head}`.\n\n" + platforms + "\n\n"
                     "Close the old app, extract the Windows ZIP, and run MPD_Viewer.exe. "
                     "Existing preferences are preserved. This is a ZIP distribution, not an installer.\n\n"
                     + install_notes +
                     "Compared with alpha.6, this release adds clearly labeled macOS and Linux manual-test "
                     "packages; the application feature set is unchanged. Current functionality includes "
                     "isolated native GUI end-to-end coverage on Windows, macOS and Linux and full Twitch "
                     "channel pages as the default viewer. "
                     "The same executable retains embedded mode behind --embedded-viewer. Optional "
                     "per-channel timers (Always, 10 minutes, or custom minutes) rotate assignments to "
                     "the next eligible live favorite. Pause automation freezes timers; paused video "
                     "still consumes assignment time. Windows monitoring authorization is now stored "
                     "in Windows Credential Manager and restored on startup. Upgrading from older "
                     "memory-only builds needs one final authorization. Disconnect removes that saved "
                     "monitoring authorization, separately from Twitch website sign-in.\n\n"
                     "For web viewers, use Twitch's own volume and quality controls. Avatar menu > "
                     "Dark Theme selects Twitch's remembered appearance; the manager follows Windows. "
                     "Central audio/quality controls remain available for embedded mode, including "
                     "the closest available resolution preference. No unsupported "
                     "Twitch player API or website styling is injected into full channel pages.\n\n"
                     "Windows native checks observed authorization surviving multiple restarts, "
                     "remembered full-page/chat dark appearance, a one-minute live-channel rotation, "
                     "paused timer accounting, and the embedded launch flag. Automated policy, UI, "
                     "vault and recovery tests also pass. See the tagged integration verification notes "
                     "for the precise evidence and unobserved cases.\n\n"
                     "**Alpha limitations:** grid mode is not implemented. Twitch decides rewards, "
                     "streaks and watch-time eligibility; assignment timers do not measure credited "
                     "watch time. Twitch navigation or raids may leave the original MPD assignment; "
                     "Retry returns to the assigned channel. Web mode has no central programmatic "
                     "volume/quality control or automatic Windows-to-Twitch theme matching. macOS/Linux "
                     "secure authorization persistence is not implemented. Native network-outage and "
                     "OS sleep recovery, chat posting and fresh-profile login/MFA remain unobserved. "
                     "The legacy embedded SDK-reported volume mismatch remains a known limitation; "
                     "use Twitch's visible controls to check actual audio.\n\n"
                     "Source archives are review material, not signed executables or website deployments. "
                     "Verify SHA256SUMS.txt and BUILD-METADATA.json against the downloaded files.\n", encoding="utf-8")
    command = ["gh", "release", "create", tag, "--repo", repo, "--verify-tag", "--draft",
               "--title", "MPD Viewer " + tag, "--notes-file", str(notes)]
    if "-" in v:
        command.append("--prerelease")
    run(*(command + [str(p) for p in files]))
    # Download the draft assets and hash every file before making it visible to repo readers.
    with tempfile.TemporaryDirectory(prefix="mpdv-release-verify-") as d:
        run("gh", "release", "download", tag, "--repo", repo, "--dir", d)
        if sorted(p.name for p in Path(d).iterdir()) != sorted(p.name for p in files):
            raise ValueError("Uploaded draft asset set differs; leaving draft unpublished.")
        for p in files:
            if digest(Path(d) / p.name) != digest(p):
                raise ValueError("Draft asset hash mismatch; leaving draft unpublished.")
    remote = gh_json("api", f"repos/{repo}")
    require_repository_policy(remote["full_name"], remote["private"])
    if live_commit(repo, tag) != head:
        raise ValueError("Tag changed while uploading; leaving draft unpublished.")
    run("gh", "release", "edit", tag, "--repo", repo, "--draft=false", "--latest=false" if "-" in v else "--latest")
    print("Published verified asset set for", tag)


def main() -> None:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("command", choices=["check-version", "require-lock", "pin-rust", "gate", "require-env",
                                        "stage-ci", "package-windows", "package-macos-unsigned", "package-linux", "publish"])
    p.add_argument("names", nargs="*")
    p.add_argument("--platform")
    a = p.parse_args()
    if a.command == "check-version": print(version())
    elif a.command == "require-env": require_env(a.names)
    elif a.command in ("stage-ci", "package-macos-unsigned"): globals()[a.command.replace("-", "_")](a.platform)
    else: globals()[a.command.replace("-", "_")]()

if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, OSError, subprocess.CalledProcessError) as e:
        print("Release operation stopped:", str(e), file=sys.stderr)
        sys.exit(1)
