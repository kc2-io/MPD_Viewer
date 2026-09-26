#!/usr/bin/env python3
"""Metadata-only Desktop E2E PR reporter; --write is the sole mutation mode."""
import argparse
from datetime import datetime, timezone
import json
import os
import re
import sys
from urllib.error import HTTPError
from urllib.parse import quote, urlencode
from urllib.request import Request, urlopen

MARKER = "<!-- mpd-desktop-e2e-report:v1 -->"
PATH = ".github/workflows/desktop-e2e.yml"
PLATFORMS = ("Windows-x64", "macOS-arm64", "Linux-x64", "macOS-x64")
CONCLUSIONS = {"success", "failure", "cancelled", "timed_out", "action_required",
               "startup_failure", "stale", "skipped", "neutral"}
INFRA_PATHS = (PATH, "tests/e2e/run.mjs", "tests/e2e/support.mjs", "tests/e2e/specs.mjs",
               "tests/e2e/video.mjs", "tests/e2e/package.json", "tests/e2e/package-lock.json",
               "scripts/e2e-linux-session.sh", "scripts/e2e-no-driver-probe.py",
               "scripts/e2e-runner-test.py", "scripts/e2e-stage-evidence.py",
               "scripts/e2e-runtime-manifest.py", "scripts/e2e-validate-evidence.py",
               "scripts/e2e-job-summary.py", "scripts/install-linux-deps.sh",
               "scripts/install-e2e-ffmpeg.py",
               "scripts/verify-native-host.py", ".github/action-pins.json")
SHA = re.compile(r"[0-9a-f]{40}\Z")
GITHUB_ACTIONS_BOT_ID = 41898282


class StaleRun(Exception):
    """The PR moved or closed after this workflow run started."""


def obj(value):
    return value if isinstance(value, dict) else {}


class Api:
    def __init__(self, base, repo, token):
        self.base = base.rstrip("/")
        self.repo = repo
        self.token = token

    def request(self, method, path, body=None, allow_missing=False):
        url = self.base + path
        data = None if body is None else json.dumps(body).encode("utf-8")
        request = Request(url, data=data, method=method,
                          headers={"Accept": "application/vnd.github+json", "Authorization": f"Bearer {self.token}",
                                   "X-GitHub-Api-Version": "2022-11-28", "Content-Type": "application/json"})
        try:
            with urlopen(request, timeout=20) as response:
                return json.loads(response.read())
        except HTTPError as error:
            if allow_missing and error.code == 404:
                return None
            raise RuntimeError(f"GitHub API {method} {path.split('?')[0]} returned {error.code}") from None

    def get(self, path, allow_missing=False):
        return self.request("GET", path, allow_missing=allow_missing)

    def pages(self, path, key=None):
        output = []
        for page in range(1, 21):
            suffix = ("&" if "?" in path else "?") + urlencode({"per_page": 100, "page": page})
            response = self.get(path + suffix)
            batch = response if key is None else response.get(key, [])
            if not isinstance(batch, list):
                raise ValueError("Unexpected paginated API response")
            output.extend(batch)
            if len(batch) < 100:
                return output
        raise ValueError("API pagination exceeded safety limit")


def numeric(value):
    return isinstance(value, int) and value > 0 and not isinstance(value, bool)


def timestamp(value, label):
    if not isinstance(value, str) or not re.fullmatch(r"\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ", value):
        raise ValueError(f"Invalid {label} timestamp")
    try:
        return datetime.strptime(value, "%Y-%m-%dT%H:%M:%SZ").replace(tzinfo=timezone.utc)
    except ValueError:
        raise ValueError(f"Invalid {label} timestamp") from None


def active_at(candidate, instant):
    created = timestamp(candidate.get("created_at"), "pull request creation")
    closed_value = candidate.get("closed_at")
    closed = None if closed_value is None else timestamp(closed_value, "pull request closure")
    return created <= instant and (closed is None or instant < closed)


def checked_run(event, repo, workflow):
    event, workflow = obj(event), obj(workflow)
    run = obj(event.get("workflow_run"))
    repository = obj(event.get("repository"))
    if repository.get("full_name") != repo or run.get("event") != "pull_request":
        raise ValueError("Not an expected pull request run")
    if repository.get("id") != obj(run.get("repository")).get("id"):
        raise ValueError("Run repository mismatch")
    if run.get("name") != "Desktop E2E" or run.get("path") != PATH or run.get("workflow_id") != workflow.get("id"):
        raise ValueError("Unexpected workflow identity")
    if not numeric(run.get("id")) or not numeric(run.get("run_attempt")) or not SHA.fullmatch(run.get("head_sha", "")):
        raise ValueError("Invalid run identity")
    branch = run.get("head_branch")
    if (not numeric(obj(run.get("head_repository")).get("id")) or not isinstance(branch, str)
            or not branch or len(branch) > 200):
        raise ValueError("Missing run head repository")
    timestamp(run.get("created_at"), "run creation")
    return run


def resolve_pr(api, run, repo_id):
    associated = run.get("pull_requests", [])
    if not isinstance(associated, list):
        raise ValueError("Invalid pull request association")
    if len(associated) > 1:
        raise ValueError("Ambiguous pull request association")
    if associated:
        number = associated[0].get("number")
        if not numeric(number):
            raise ValueError("Invalid associated pull request")
        candidates = [api.get(f"/repos/{api.repo}/pulls/{number}")]
    else:
        head_repo = obj(run.get("head_repository"))
        owner = obj(head_repo.get("owner")).get("login", "")
        branch = run.get("head_branch", "")
        if not re.fullmatch(r"[A-Za-z0-9-]+", owner):
            raise ValueError("Invalid fallback head selector")
        selector = urlencode({"state": "all", "head": owner + ":" + branch})
        historical = api.pages(f"/repos/{api.repo}/pulls?{selector}")
        run_created = timestamp(run.get("created_at"), "run creation")
        candidates = []
        for candidate in historical:
            candidate = obj(candidate)
            head, base = obj(candidate.get("head")), obj(candidate.get("base"))
            if (active_at(candidate, run_created)
                    and obj(head.get("repo")).get("id") == head_repo.get("id")
                    and obj(base.get("repo")).get("id") == repo_id):
                candidates.append(candidate)
        if len(candidates) != 1:
            raise ValueError("Fork fallback did not identify exactly one PR active when the run began")
    pr = obj(candidates[0])
    if not active_at(pr, timestamp(run.get("created_at"), "run creation")):
        raise ValueError("Pull request association was not active when the run began")
    head, base = obj(pr.get("head")), obj(pr.get("base"))
    if pr.get("state") != "open" or head.get("sha") != run["head_sha"]:
        raise StaleRun("PR head moved or closed")
    if (obj(head.get("repo")).get("id") != obj(run.get("head_repository")).get("id")
            or obj(base.get("repo")).get("id") != repo_id or not SHA.fullmatch(base.get("sha", ""))):
        raise ValueError("PR repository/base identity mismatch")
    return pr


def newest(api, run, workflow_id):
    selector = urlencode({"event": "pull_request", "head_sha": run["head_sha"]})
    runs = api.pages(f"/repos/{api.repo}/actions/workflows/{workflow_id}/runs?{selector}", "workflow_runs")
    matches = [candidate for candidate in runs if isinstance(candidate, dict)
               and candidate.get("head_sha") == run["head_sha"]
               and obj(candidate.get("head_repository")).get("id") == obj(run.get("head_repository")).get("id")
               and candidate.get("head_branch") == run["head_branch"] and candidate.get("path") == PATH
               and candidate.get("event") == "pull_request" and numeric(candidate.get("run_number"))]
    if not matches:
        raise ValueError("Current run missing from workflow run listing")
    winner = max(matches, key=lambda item: (item["run_number"], item.get("run_attempt", 1)))
    return winner.get("id") == run["id"] and winner.get("run_attempt", 1) == run["run_attempt"]


def infrastructure_changed(api, base_sha, head_sha):
    changed = False
    for path in INFRA_PATHS:
        encoded = quote(path, safe="/")
        base = obj(api.get(f"/repos/{api.repo}/contents/{encoded}?ref={base_sha}", allow_missing=True))
        head = obj(api.get(f"/repos/{api.repo}/contents/{encoded}?ref={head_sha}", allow_missing=True))
        if base.get("sha") != head.get("sha"):
            changed = True
    return changed


def rows(api, run):
    run_id, attempt = run["id"], run["run_attempt"]
    jobs_by_platform = {}
    if run.get("status") not in {"requested", "queued", "waiting", "pending"}:
        # GitHub's "re-run failed jobs" carries successful jobs and artifacts
        # from older attempts. Reconstruct each platform from its newest actual
        # job attempt rather than pretending every artifact belongs to the run's
        # current attempt.
        for job_attempt in range(1, min(attempt, 20) + 1):
            jobs = api.pages(f"/repos/{api.repo}/actions/runs/{run_id}/attempts/{job_attempt}/jobs", "jobs")
            for platform in PLATFORMS:
                label = f"GUI ({platform})"
                matching = [job for job in jobs if isinstance(job, dict) and job.get("name") == label]
                if len(matching) == 1:
                    jobs_by_platform[platform] = (matching[0], job_attempt)
    artifacts = api.pages(f"/repos/{api.repo}/actions/runs/{run_id}/artifacts", "artifacts")
    result = []
    for platform in PLATFORMS:
        job, job_attempt = jobs_by_platform.get(platform, (None, attempt))
        state = (job or {}).get("conclusion") or ((job or {}).get("status") if job else "not scheduled")
        if state not in CONCLUSIONS | {"queued", "in_progress", "not scheduled"}:
            state = "unknown"
        artifact_name = f"Desktop-E2E-{platform}-{run_id}-attempt{job_attempt}"
        matching_artifacts = [item for item in artifacts if isinstance(item, dict)
                              and item.get("name") == artifact_name and not item.get("expired")
                              and numeric(item.get("id"))]
        artifact = matching_artifacts[0] if len(matching_artifacts) == 1 else None
        if artifact:
            url = f"https://github.com/{api.repo}/actions/runs/{run_id}/artifacts/{artifact['id']}"
            evidence = f"[Download]({url})"
            digest = artifact.get("digest", "")
            if re.fullmatch(r"sha256:[0-9a-f]{64}", digest):
                evidence += f" (`{digest}`)"
            expires = artifact.get("expires_at", "")
            if re.fullmatch(r"\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ", expires):
                evidence += f"; expires {expires}"
        else:
            evidence = "Missing or expired"
        result.append(f"| {platform} | {state} | {evidence} |")
    return result


def body(api, run, pr, changed):
    status = run.get("conclusion") or run.get("status") or "unknown"
    if status not in CONCLUSIONS | {"requested", "queued", "in_progress", "waiting", "pending"}:
        status = "unknown"
    run_id, attempt = run["id"], run["run_attempt"]
    link = f"https://github.com/{api.repo}/actions/runs/{run_id}/attempts/{attempt}"
    completed = run.get("updated_at", "") if run.get("status") == "completed" else "running"
    if not re.fullmatch(r"\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ", completed) and completed != "running":
        completed = "unknown"
    banner = "Test infrastructure differs from the default branch; this PR controls its E2E tests and capture code.\n\n" if changed else ""
    return (f"{MARKER}\n### Desktop E2E evidence\n\n{banner}"
            f"PR head / Actions run head: `{pr['head']['sha']}`. PR base at report time: "
            f"`{pr['base']['sha']}`. GitHub tested the generated "
            "pull-request merge ref; its exact checkout SHA is recorded inside each artifact manifest.\n\n"
            f"Run {run_id}, attempt {attempt}: **{status}**; updated {completed}. "
            f"[Open run and summary]({link}).\n\n"
            "| Platform | Job | Evidence |\n| --- | --- | --- |\n" + "\n".join(rows(api, run)) + "\n\n"
            "GitHub retains these artifacts for seven days. Downloading the ZIP links requires GitHub sign-in. "
            "Results come from controlled fixtures at the PR revision and do not establish live Twitch behavior.\n")


def owned_comment(comment):
    comment = obj(comment)
    user, app = obj(comment.get("user")), obj(comment.get("performed_via_github_app"))
    return (str(comment.get("body") or "").splitlines()[:1] == [MARKER]
            and user.get("login") == "github-actions[bot]" and user.get("type") == "Bot"
            and user.get("id") == GITHUB_ACTIONS_BOT_ID
            and app.get("slug") in {None, "github-actions"})


def report(api, event, write=False):
    repo_info = obj(api.get(f"/repos/{api.repo}"))
    workflow = obj(api.get(f"/repos/{api.repo}/actions/workflows/desktop-e2e.yml"))
    if workflow.get("path") != PATH or not numeric(workflow.get("id")):
        raise ValueError("Default-branch workflow identity unavailable")
    run = checked_run(event, api.repo, workflow)
    try:
        pr = resolve_pr(api, run, repo_info["id"])
    except StaleRun:
        return "Skipped run for a PR that moved or closed."
    if not newest(api, run, workflow["id"]):
        return "Skipped stale run or attempt; current PR head unchanged."
    live = obj(api.get(f"/repos/{api.repo}/actions/runs/{run['id']}"))
    if (live.get("id") != run["id"] or live.get("run_attempt") != run["run_attempt"]
            or live.get("head_sha") != run["head_sha"] or live.get("workflow_id") != workflow["id"]):
        return "Skipped run whose live metadata changed."
    run = {**run, "status": live.get("status"), "conclusion": live.get("conclusion"),
           "updated_at": live.get("updated_at")}
    changed = infrastructure_changed(api, obj(pr.get("base"))["sha"], run["head_sha"])
    content = body(api, run, pr, changed)
    number = pr["number"]
    comments = api.pages(f"/repos/{api.repo}/issues/{number}/comments")
    matches = [comment for comment in comments if owned_comment(comment) and numeric(obj(comment).get("id"))]
    matches.sort(key=lambda item: item.get("id", 0))
    if not write:
        return f"Dry run: PR #{number}; would {'update' if matches else 'create'} Desktop E2E comment.\n\n{content}"
    current_pr = obj(api.get(f"/repos/{api.repo}/pulls/{number}"))
    current_head = obj(current_pr.get("head"))
    current_base = obj(current_pr.get("base"))
    if (current_pr.get("state") != "open" or current_head.get("sha") != run["head_sha"]
            or obj(current_head.get("repo")).get("id") != obj(run.get("head_repository")).get("id")
            or current_base.get("sha") != obj(pr.get("base")).get("sha")
            or obj(current_base.get("repo")).get("id") != repo_info.get("id")
            or not newest(api, run, workflow["id"])):
        return "Skipped PR whose head, base or newest run changed before comment write."
    if matches:
        api.request("PATCH", f"/repos/{api.repo}/issues/comments/{matches[0]['id']}", {"body": content})
        return f"Updated Desktop E2E comment on PR #{number}."
    api.request("POST", f"/repos/{api.repo}/issues/{number}/comments", {"body": content})
    return f"Created Desktop E2E comment on PR #{number}."


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    if args.dry_run and args.write:
        parser.error("choose only one mode")
    try:
        repo = os.environ["GITHUB_REPOSITORY"]
        token = os.environ["GITHUB_TOKEN"]
        event = json.loads(open(os.environ["GITHUB_EVENT_PATH"], encoding="utf-8").read())
        if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repo):
            raise ValueError("Invalid repository")
        result = report(Api(os.environ.get("GITHUB_API_URL", "https://api.github.com"), repo, token), event, args.write)
        print(result)
        status = 0
    except (KeyError, ValueError, RuntimeError, OSError, AttributeError, TypeError) as error:
        result = f"Desktop E2E reporter stopped: {error}"
        print(result, file=sys.stderr)
        status = 1
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as output:
            output.write(result + "\n")
    return status


if __name__ == "__main__":
    sys.exit(main())
