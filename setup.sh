#!/usr/bin/bash

git clone https://github.com/microsoft/vcpkg.git --depth 1
cd vcpkg
release_output=$(cat /etc/os-release)
# Support both MSYS2 and Git Bash
if [[ $string == *"MSYS2"* ]] || [ -f /git-bash.exe ] || [ -f /msys2.exe ]; then
    echo "MSYS2 detected, bootstrapping vcpkg for Windows..."
    ".\bootstrap-vcpkg.bat"
    echo "Installing dependencies..."
    # OpenSSL
    ./vcpkg.exe install openssl
    pause
else
    echo "Other platform detected, but we don't need to bootstrap vcpkg :)"
fi
cd ..
echo "Done!"
