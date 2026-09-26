#!/usr/bin/env python3
"""Install a pinned, checksum-verified FFmpeg/FFprobe pair for desktop E2E."""
import gzip
import hashlib
import os
from pathlib import Path
import platform
import stat
import subprocess
import sys
import tempfile
from urllib.parse import urlparse
from urllib.request import Request, urlopen

RELEASE = "b6.1.1"
BASE_URL = f"https://github.com/eugeneware/ffmpeg-static/releases/download/{RELEASE}"
MAX_DOWNLOAD_BYTES = 40 * 1024 * 1024
MAX_BINARY_BYTES = 128 * 1024 * 1024
ASSETS = {
    ("linux", "x64"): {
        "ffmpeg": ("ffmpeg-linux-x64.gz", "bfe8a8fc511530457b528c48d77b5737527b504a3797a9bc4866aeca69c2dffa"),
        "ffprobe": ("ffprobe-linux-x64.gz", "25d9b6ccb05e3d9de9e04e31e2506d8dd7f9f0418981965ac6df12e8d3afd067"),
    },
    ("darwin", "x64"): {
        "ffmpeg": ("ffmpeg-darwin-x64.gz", "929b375c1182d956c51f7ac25e0b2b0411fb01f6f407aa15c9758efeb4242106"),
        "ffprobe": ("ffprobe-darwin-x64.gz", "d4da574d6e2e197bd259b47d69cf262df9e312af24ad960444f6d806d3d4c186"),
    },
    ("darwin", "arm64"): {
        "ffmpeg": ("ffmpeg-darwin-arm64.gz", "8923876afa8db5585022d7860ec7e589af192f441c56793971276d450ed3bbfa"),
        "ffprobe": ("ffprobe-darwin-arm64.gz", "d986a8ec7b030899fe66a8a288ed809a3543338705a3ce178cfb85869c5d80be"),
    },
    ("win32", "x64"): {
        "ffmpeg": ("ffmpeg-win32-x64.gz", "8883a3dffbd0a16cf4ef95206ea05283f78908dbfb118f73c83f4951dcc06d77"),
        "ffprobe": ("ffprobe-win32-x64.gz", "f309e6223ad89d2fe54bccd420a7709b66fd27540674e92309578ed491a43c8d"),
    },
}


def architecture(value):
    normalized = value.lower()
    if normalized in {"amd64", "x86_64"}:
        return "x64"
    if normalized in {"arm64", "aarch64"}:
        return "arm64"
    return normalized


def download(url, destination, expected):
    request = Request(url, headers={"User-Agent": "MPD-Viewer-E2E-recorder-installer"})
    digest, size = hashlib.sha256(), 0
    with urlopen(request, timeout=60) as response, destination.open("xb") as output:
        host = urlparse(response.geturl()).hostname
        if host not in {"github.com", "release-assets.githubusercontent.com", "objects.githubusercontent.com"}:
            raise ValueError("Recorder asset redirected to an unexpected host")
        while chunk := response.read(1024 * 1024):
            size += len(chunk)
            if size > MAX_DOWNLOAD_BYTES:
                raise ValueError("Recorder asset exceeded its download ceiling")
            digest.update(chunk)
            output.write(chunk)
    if digest.hexdigest() != expected:
        raise ValueError("Recorder asset checksum mismatch")


def decompress(source, destination):
    size = 0
    with gzip.open(source, "rb") as compressed, destination.open("xb") as output:
        while chunk := compressed.read(1024 * 1024):
            size += len(chunk)
            if size > MAX_BINARY_BYTES:
                raise ValueError("Recorder binary exceeded its extraction ceiling")
            output.write(chunk)
    if size == 0:
        raise ValueError("Recorder binary was empty")
    destination.chmod(destination.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)


def verify(destination, windows, backend):
    suffix = ".exe" if windows else ""
    ffmpeg, ffprobe = destination / f"ffmpeg{suffix}", destination / f"ffprobe{suffix}"
    version = subprocess.run([str(ffmpeg), "-version"], capture_output=True, text=True, timeout=15, check=True)
    subprocess.run([str(ffprobe), "-version"], capture_output=True, text=True, timeout=15, check=True)
    devices = subprocess.run([str(ffmpeg), "-hide_banner", "-devices"], capture_output=True, text=True, timeout=15, check=True)
    encoders = subprocess.run([str(ffmpeg), "-hide_banner", "-encoders"], capture_output=True, text=True, timeout=15, check=True)
    if backend not in devices.stdout or "libx264" not in encoders.stdout:
        raise ValueError(f"Pinned FFmpeg lacks required {backend} or libx264 support")
    print(version.stdout.splitlines()[0][:200])


def install(system=sys.platform, machine=platform.machine(), runner_temp=None, github_path=None):
    arch = architecture(machine)
    try:
        assets = ASSETS[(system, arch)]
    except KeyError:
        raise ValueError(f"No pinned recorder for {system}/{arch}") from None
    runner = Path(runner_temp or os.environ["RUNNER_TEMP"])
    destination = runner / f"mpd-e2e-ffmpeg-{RELEASE}"
    if destination.exists():
        raise ValueError("Pinned recorder destination already exists")
    destination.mkdir(parents=True)
    windows = system == "win32"
    suffix = ".exe" if windows else ""
    with tempfile.TemporaryDirectory(prefix="mpd-e2e-recorder-", dir=runner) as temporary:
        for tool, (filename, checksum) in assets.items():
            compressed = Path(temporary) / filename
            download(f"{BASE_URL}/{filename}", compressed, checksum)
            decompress(compressed, destination / f"{tool}{suffix}")
    backends = {"linux": "x11grab", "darwin": "avfoundation", "win32": "gdigrab"}
    verify(destination, windows, backends[system])
    path_file = Path(github_path or os.environ["GITHUB_PATH"])
    with path_file.open("a", encoding="utf-8") as output:
        output.write(str(destination) + "\n")
    print(f"Installed checksum-verified FFmpeg release {RELEASE} for {system}/{arch}")
    return destination


if __name__ == "__main__":
    try:
        install()
    except (EOFError, KeyError, OSError, ValueError, subprocess.SubprocessError) as error:
        print(f"Recorder installation failed: {error}", file=sys.stderr)
        sys.exit(1)
