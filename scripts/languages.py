#!/usr/bin/env python3
import json
import subprocess

data = json.loads(subprocess.run(["tokei", "-o", "json"], capture_output=True).stdout)
c_count = data["C"]["code"]
rust_count = data["Rust"]["code"]
unsafe_count = str(subprocess.run(["ag", "unsafe"], capture_output=True).stdout).count('unsafe')

print(json.dumps({
    "C": c_count,
    "Rust": rust_count,
    "unsafe": unsafe_count,
}))
