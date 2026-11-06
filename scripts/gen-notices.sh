#!/usr/bin/env bash
# Daftar lisensi seluruh dependensi (transitif) Rust dan npm, dibaca dari metadata
# yang sudah ada (tanpa jaringan): `cargo metadata` + package-lock.json.
#
#   scripts/gen-notices.sh > build/dependency-licenses.md
#
# Hasilnya ringkasan nama/versi/lisensi per paket (bukan teks lisensi lengkap).
# `cargo-about` / `cargo-license` tidak diperlukan.
set -euo pipefail
cd "$(dirname "$0")/.."

cargo metadata --format-version 1 --locked --manifest-path src-tauri/Cargo.toml 2>/dev/null |
python3 -I -c '
import json, sys
d = json.load(sys.stdin)
ws = set(d["workspace_members"])
rows = sorted((p["name"], p["version"], p.get("license") or p.get("license_file") or "(tidak dinyatakan)")
              for p in d["packages"] if p["id"] not in ws)
print("# Dependensi Rust (%d paket)\n" % len(rows))
print("| Paket | Versi | Lisensi |\n|---|---|---|")
for n, v, l in rows:
    print("| %s | %s | %s |" % (n, v, l.replace("|", "/")))
'

python3 -I - <<'PY'
import json
lock = json.load(open("package-lock.json"))
rows = {"runtime": [], "dev": []}
for path, p in lock["packages"].items():
    if not path:
        continue
    name = path.split("node_modules/")[-1]
    lic = p.get("license") or "(tidak dinyatakan)"
    rows["dev" if p.get("dev") else "runtime"].append((name, p.get("version", "?"), lic))
for key, title in (("runtime", "Dependensi npm yang ikut ke aplikasi"), ("dev", "Dependensi npm untuk pengembangan/build saja")):
    items = sorted(rows[key])
    print("\n# %s (%d paket)\n" % (title, len(items)))
    print("| Paket | Versi | Lisensi |\n|---|---|---|")
    for n, v, l in items:
        print("| %s | %s | %s |" % (n, v, str(l).replace("|", "/")))
PY
