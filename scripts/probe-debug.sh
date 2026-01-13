#!/usr/bin/env bash
set -e

dirPath=$1
firmware_version=$2

curl --progress-bar --proto '=https' --tlsv1.2 -fSLo $dirPath/rriv-firmware-debug.elf https://github.com/rrivirr/rriv-firmware/releases/download/$firmware_version/rriv-firmware-debug.elf

probe-rs attach $dirPath/rriv-firmware-debug.elf \
        --chip STM32F103RE  \
        --protocol swd \
        --allow-erase-all \
        --chip-erase