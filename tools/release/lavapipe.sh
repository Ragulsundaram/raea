#!/usr/bin/env bash
# Mesa's lavapipe 26.2.4 (software Vulkan, MIT) on a Windows runner, for release checks that render: the runner has no
# GPU, and WARP crashes compiling one of Bevy's compute shaders (11-04). On the runner only, never in a zip.
set -euo pipefail
mkdir -p "$RUNNER_TEMP/mesa" && cd "$RUNNER_TEMP/mesa"
curl -fsSL -o mesa.7z https://github.com/pal1000/mesa-dist-win/releases/download/26.2.4/mesa3d-26.2.4-release-msvc.7z
7z x -y mesa.7z x64/vulkan_lvp.dll x64/lvp_icd.x86_64.json >/dev/null
icd=$(cygpath -w "$RUNNER_TEMP/mesa/x64/lvp_icd.x86_64.json")
# the runner's shell is elevated, and the Vulkan loader ignores VK_DRIVER_FILES for an elevated process: the driver is
# registered where an installed one is
pwsh -c "New-Item -Path HKLM:\\SOFTWARE\\Khronos\\Vulkan\\Drivers -Force | Out-Null; New-ItemProperty -Path HKLM:\\SOFTWARE\\Khronos\\Vulkan\\Drivers -Name '$icd' -PropertyType DWord -Value 0 -Force | Out-Null"
{ echo "WGPU_BACKEND=vulkan"; echo "VK_LOADER_DEBUG=error,driver"; } >> "$GITHUB_ENV"
