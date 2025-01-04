#!/usr/bin/bash

# This script will do nothing on Linux, but will set up the environment for Windows
release_output=$(cat /etc/os-release)
# Support both MSYS2 and Git Bash
if [[ $string == *"MSYS2"* ]] || [ -f /git-bash.exe ] || [ -f /msys2.exe ]; then
    export VCPKG_ROOT=$(realpath ./vcpkg)
    export OPENSSL_DIR="$VCPKG_ROOT/installed/x64-windows"
    export PATH="$VCPKG_ROOT:$PATH"
fi
