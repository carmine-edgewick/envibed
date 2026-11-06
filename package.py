#!/usr/bin/env python3
# Builds the release zip, target/envibed-<version>.zip: the DLL, our license and the license notices of every crate
# built into it, laid out to extract straight into the game folder.
import json, os, subprocess, zipfile

TARGET = "x86_64-pc-windows-msvc"
FOLDER = "mods/carmine-edgewick.envibed/"
NAMES = ("LICENSE", "LICENCE", "COPYING", "UNLICENSE")

def cargo(*args):
    return subprocess.run(["cargo", *args], check=True, capture_output=True, text=True).stdout

# A workspace member keeps its license at the workspace root; cargo marks the top of every unpacked source with .cargo-ok.
def licenses(d):
    while True:
        found = sorted(f for f in os.listdir(d) if f.upper().startswith(NAMES) and os.path.isfile(os.path.join(d, f)))
        if found or os.path.exists(os.path.join(d, ".cargo-ok")) or d == os.path.dirname(d):
            return [os.path.join(d, f) for f in found]
        d = os.path.dirname(d)

def notices(meta):
    built = {tuple(l.split()[:2]) for l in cargo("tree", "-e", "normal", "--target", TARGET, "--prefix", "none", "-f", "{p}").splitlines()}
    out = ["Third-party software built into envibed.dll\n\n"]
    for p in sorted(meta["packages"], key=lambda p: p["name"]):
        if (p["name"], "v" + p["version"]) not in built or p["name"] == "envibed":
            continue
        out.append(f"--- {p['name']} {p['version']} ({p['license']}) {p.get('repository') or ''}\n\n")
        files = licenses(os.path.dirname(p["manifest_path"]))
        out += [open(f, errors="replace").read().strip() + "\n\n" for f in files] or [f"(no license file; {p['license']})\n\n"]
    return "".join(out)

os.chdir(os.path.dirname(os.path.abspath(__file__)))
# Panic messages carry each crate's source path, under the builder's home; cargo joins this array onto the config's.
home = os.path.expanduser("~")
subprocess.run(["cargo", "xwin", "build", "--release", "--target", TARGET, "--config", f'target.{TARGET}.rustflags = ["--remap-path-prefix={home}=~"]'], check=True)
dll = f"target/{TARGET}/release/envibed.dll"
if home.encode() in open(dll, "rb").read():
    raise SystemExit(f"{dll} still holds {home}")
meta = json.loads(cargo("metadata", "--format-version", "1", "--filter-platform", TARGET))
version = next(p["version"] for p in meta["packages"] if p["name"] == "envibed")
path = f"target/envibed-{version}.zip"
with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as z:
    z.write(dll, FOLDER + "envibed.dll")
    z.write("LICENSE", FOLDER + "LICENSE.txt")
    z.writestr(FOLDER + "THIRD-PARTY.txt", notices(meta))
print(path)
