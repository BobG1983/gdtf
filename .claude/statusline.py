#!/usr/bin/env python3
"""Claude Code status line for grimdark-turfwar.
Prints: project · ⛎ branch · model · NN% ctx.
"""
import json
import os
import subprocess
import sys

try:
    d = json.load(sys.stdin)
except Exception:
    d = {}

model = (d.get("model") or {}).get("display_name") or "?"
ws = d.get("workspace") or {}
cwd = ws.get("current_dir") or d.get("cwd") or os.getcwd()
proj = os.path.basename((ws.get("project_dir") or cwd).rstrip("/")) or "project"

branch = ""
try:
    branch = subprocess.check_output(
        ["git", "-C", cwd, "branch", "--show-current"],
        text=True, stderr=subprocess.DEVNULL,
    ).strip()
except Exception:
    pass

# Context usage comes straight off the payload (Claude Code provides it).
pct = None
cw = d.get("context_window") or {}
if isinstance(cw, dict):
    pct = cw.get("used_percentage")
    if pct is None and cw.get("context_window_size"):
        try:
            pct = round(100.0 * (cw.get("total_input_tokens") or 0) / cw["context_window_size"])
        except Exception:
            pct = None

ORANGE = "\033[38;5;208m"
GREY = "\033[38;5;245m"
DIM = "\033[2m"
GREEN = "\033[38;5;71m"
YELLOW = "\033[38;5;179m"
RED = "\033[38;5;167m"
R = "\033[0m"

seg = [f"{ORANGE}{proj}{R}"]
if branch:
    seg.append(f"{GREY}⛎ {branch}{R}")
seg.append(f"{DIM}{model}{R}")
if isinstance(pct, (int, float)):
    p = int(round(pct))
    col = GREEN if p < 70 else (YELLOW if p < 90 else RED)
    seg.append(f"{col}◴ {p}% ctx{R}")
sys.stdout.write("  ".join(seg))
