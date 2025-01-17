#!/bin/bash
tokei -o json | jq -c '{C: .C.code, Rust: .Rust.code}'
