#!/bin/sh
# One-command parity check for the cJSON C-to-Rust port.
#
#   ./parity.sh            (or: make parity)
#
# 1. Builds build/oracle (tools/cjson-oracle.c + original cJSON.c),
#    target/release/rust-driver (cjson-core) and build/oracle-ffi
#    (tools/cjson-oracle.c + target/release/libcjson_ffi.a).
# 2. Runs every fixture matched by the "fixtures" globs in .cursor/parity.json
#    through all three and compares stdout bytes and exit status:
#      core: build/oracle vs target/release/rust-driver  (what the parity gate checks)
#      ffi:  build/oracle vs build/oracle-ffi
#    The report format is tools/DRIVER_FORMAT.md. A fixture named in a
#    PARITY_EXCEPTIONS.md row counts as "excepted" instead of "diverged". The
#    row's test must pass, and step 3 runs it.
#    Writes build/parity/summary.md (per-fixture table) and keeps the outputs
#    of diverging fixtures in build/parity/out/.
# 3. cargo test --workspace
# 4. cargo clippy --workspace --all-targets -- -D warnings
# 5. make c-unity      original Unity suites against cJSON.c
# 6. make ffi-unity    public-API Unity suites against libcjson_ffi.a
# 7. make legacy-diff  test.c against cJSON.c and libcjson_ffi.a, diff stdout
#
# Exit status: 0 only if every step passed and no fixture diverged.
set -eu

cd "$(dirname "$0")"
PY=/usr/bin/python3
OUT=build/parity
mkdir -p "$OUT/logs"

failures=""
results=""

# step <name> <command...>: run a step, log it, record pass/fail, keep going.
step() {
    name=$1
    shift
    log="$OUT/logs/$name.log"
    echo "==> $name: $*"
    if "$@" > "$log" 2>&1; then
        results="$results
$name: ok"
    else
        results="$results
$name: FAILED (see $log)"
        failures="$failures $name"
        tail -n 20 "$log"
    fi
}

step build-oracle make oracle
step build-rust-driver make rust-driver
step build-oracle-ffi make oracle-ffi

echo "==> fixtures"
fixture_status=0
"$PY" - "$OUT" <<'PYEOF' || fixture_status=$?
import json, re, subprocess, sys
from pathlib import Path

out = Path(sys.argv[1])
root = Path(".").resolve()
cfg = json.loads(Path(".cursor/parity.json").read_text(encoding="utf-8"))
timeout = float(cfg.get("per_run_timeout_s", 10))

# Same expansion as the parity gate: Path.glob per pattern, files only, sorted.
fixtures = sorted({
    p.relative_to(root).as_posix()
    for pattern in cfg["fixtures"]
    for p in root.glob(pattern)
    if p.is_file()
})

# Fixtures named by PARITY_EXCEPTIONS.md rows (same table rules as the gate).
excepted = {}
exc = Path(cfg.get("exceptions_file", "PARITY_EXCEPTIONS.md"))
if exc.is_file():
    lines = exc.read_text(encoding="utf-8", errors="replace").splitlines()
    i = 0
    while i < len(lines):
        if not lines[i].strip().startswith("|"):
            i += 1
            continue
        cells = lambda s: [c.strip().replace("**", "").strip("`").strip() for c in re.split(r"(?<!\\)\|", s.strip().strip("|"))]
        header = [c.lower() for c in cells(lines[i])]
        i += 1
        if not {"id", "test", "fixture"} <= set(header):
            continue
        while i < len(lines) and lines[i].strip().startswith("|"):
            row = cells(lines[i])
            i += 1
            if len(row) != len(header) or all(re.fullmatch(r":?-{3,}:?", c) for c in row if c):
                continue
            rid = row[header.index("id")]
            if rid.lower() in {"", "-", "--", "\u2014", "\u2013", "n/a", "none"}:
                continue
            for f in re.split(r",|;|<br\s*/?>", row[header.index("fixture")]):
                f = f.strip().strip("`").removeprefix("./")
                if f and f.lower() not in {"-", "\u2014", "n/a", "none"}:
                    excepted.setdefault(f, []).append(rid)

def run(argv):
    try:
        r = subprocess.run(argv, capture_output=True, timeout=timeout)
    except subprocess.TimeoutExpired:
        return ("timeout", b"")
    except OSError as e:
        return (f"not started ({e.strerror})", b"")
    status = f"signal {-r.returncode}" if r.returncode < 0 else f"exit {r.returncode}"
    return (status, r.stdout)

def first_diff(a, b):
    n = min(len(a), len(b))
    for i in range(n):
        if a[i] != b[i]:
            return i
    return n

def context(data, off):
    lo = max(0, off - 16)
    return f"@{lo}: {data[lo:off + 16]!r}"

sides = {"core": "./target/release/rust-driver", "ffi": "./build/oracle-ffi"}
counts = {k: {"identical": 0, "excepted": 0, "diverged": 0} for k in sides}
rows, details = [], {k: [] for k in sides}
keep = out / "out"
if keep.exists():
    for old in keep.iterdir():
        old.unlink()
keep.mkdir(parents=True, exist_ok=True)

for rel in fixtures:
    c_status, c_out = run(["./build/oracle", rel])
    row = {"fixture": rel, "c": c_status}
    for side, exe in sides.items():
        status, data = run([exe, rel])
        same = c_status == status and c_out == data and c_status != "timeout" and not c_status.startswith("not started")
        if same:
            verdict = "identical"
        elif rel in excepted:
            verdict = "excepted " + ",".join(excepted[rel])
        else:
            verdict = "DIVERGED"
        counts[side][verdict.split()[0].lower()] += 1
        row[side] = verdict
        row[side + "_status"] = status
        if not same:
            safe = rel.replace("/", "__")
            (keep / f"{safe}.c.out").write_bytes(c_out)
            (keep / f"{safe}.{side}.out").write_bytes(data)
            off = first_diff(c_out, data)
            details[side].append(
                f"- `{rel}`: C {c_status} ({len(c_out)} bytes), {side} {status} ({len(data)} bytes); "
                f"first difference at byte {off}\n    C     {context(c_out, off)}\n    {side:<5} {context(data, off)}"
            )
    rows.append(row)

md = ["# Parity summary", ""]
for side in sides:
    c = counts[side]
    md.append(f"- {side}: {len(fixtures)} fixtures, {c['identical']} identical, {c['excepted']} excepted, {c['diverged']} diverged")
md += ["", "core = `build/oracle` vs `target/release/rust-driver`; ffi = `build/oracle` vs `build/oracle-ffi`.",
       "Compared: stdout bytes and exit status (tools/DRIVER_FORMAT.md). Fixture globs: "
       + ", ".join(f"`{g}`" for g in cfg["fixtures"]) + ".", "",
       "## Per-fixture results", "",
       "| Fixture | core | ffi | C status | Rust status | FFI status |", "|---|---|---|---|---|---|"]
for r in rows:
    md.append(f"| `{r['fixture']}` | {r['core']} | {r['ffi']} | {r['c']} | {r['core_status']} | {r['ffi_status']} |")
for side in sides:
    md += ["", f"## Divergences ({side})", ""]
    md += details[side] or ["None."]
md.append("")
(out / "summary.md").write_text("\n".join(md), encoding="utf-8")

for side in sides:
    c = counts[side]
    print(f"{side}: {len(fixtures)} fixtures, {c['identical']} identical, {c['excepted']} excepted, {c['diverged']} diverged")
    for d in details[side][:5]:
        print("  " + d.replace("\n", "\n  "))
print(f"per-fixture table: {out / 'summary.md'}")
sys.exit(1 if any(counts[s]["diverged"] for s in sides) else 0)
PYEOF
if [ "$fixture_status" -eq 0 ]; then
    results="$results
fixtures: ok"
else
    results="$results
fixtures: FAILED (see $OUT/summary.md)"
    failures="$failures fixtures"
fi

step cargo-test cargo test --workspace
step cargo-clippy cargo clippy --workspace --all-targets -- -D warnings
step c-unity make c-unity
step ffi-unity make ffi-unity
step legacy-diff make legacy-diff

echo
echo "==== parity summary ===="
grep -E "^- (core|ffi): " "$OUT/summary.md" | sed 's/^- //' || true
grep -hE "^test result:" "$OUT/logs/cargo-test.log" 2>/dev/null | awk '{p+=$4; f+=$6; i+=$8} END {if (NR) printf "cargo test: %d passed, %d failed, %d ignored (%d test binaries)\n", p, f, i, NR}' || true
grep -hE "^[0-9]+ Tests [0-9]+ Failures" "$OUT/logs/c-unity.log" 2>/dev/null | awk '{t+=$1; f+=$3; i+=$5} END {if (NR) printf "c-unity: %d suites, %d tests, %d failures, %d ignored\n", NR, t, f, i}' || true
grep -hE "^[0-9]+ Tests [0-9]+ Failures" "$OUT/logs/ffi-unity.log" 2>/dev/null | awk '{t+=$1; f+=$3; i+=$5} END {if (NR) printf "ffi-unity: %d suites, %d tests, %d failures, %d ignored\n", NR, t, f, i}' || true
echo "$results" | sed '/^$/d'
if [ -n "$failures" ]; then
    echo "PARITY: FAIL ($(echo "$failures" | sed 's/^ //'))"
    exit 1
fi
echo "PARITY: PASS"
