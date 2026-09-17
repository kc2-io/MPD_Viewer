#!/usr/bin/env python3
"""Configure a real HTTPS origin before a build. No hosting, uploads, or certificate changes."""
from __future__ import annotations
import argparse
import json
from pathlib import Path
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parents[1]
LOCAL = "http://localhost:*/*"

def validate_url(value: str) -> str:
    parsed = urlsplit(value)
    if parsed.scheme != "https" or not parsed.hostname or parsed.username or parsed.password:
        raise ValueError("Use an HTTPS URL without embedded credentials.")
    if parsed.query or parsed.fragment:
        raise ValueError("Use a static page URL with no query string or fragment.")
    if not parsed.path.endswith(("/", "/index.html")):
        raise ValueError("Point to /index.html or a directory ending in /.")
    # Accessing .port validates malformed/invalid port values.
    _ = parsed.port
    return f"{parsed.scheme}://{parsed.netloc}/*"

def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("url", help="HTTPS URL of the hosted player-wrapper/index.html, or --local", nargs="?")
    parser.add_argument("--local", action="store_true", help="Restore development-only localhost wrapper")
    args = parser.parse_args()
    if args.local and args.url:
        parser.error("Choose --local or an HTTPS URL, not both.")
    if not args.local and not args.url:
        parser.error("Provide an HTTPS player URL or --local.")
    try:
        remote = [LOCAL] if args.local else [LOCAL, validate_url(args.url)]
    except ValueError as error:
        parser.error(str(error))
    path = ROOT / "src-tauri/capabilities/players.json"
    capability = json.loads(path.read_text(encoding="utf-8"))
    capability["remote"]["urls"] = remote
    path.write_text(json.dumps(capability, indent=2) + "\n", encoding="utf-8")
    (ROOT / "src-tauri/player-origin.json").write_text(
        json.dumps({"url": None if args.local else args.url}, indent=2) + "\n", encoding="utf-8")
    print("Player origin configured. Rebuild the native application for this change to take effect.")

if __name__ == "__main__":
    main()
