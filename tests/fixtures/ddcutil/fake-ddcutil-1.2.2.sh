#!/usr/bin/env bash
# Fake ddcutil 1.2.2

if [[ "$*" == *"--version"* ]]; then
    echo "ddcutil 1.2.2"
    exit 0
fi

if [[ "$*" == *"--help"* ]]; then
    echo "Usage: ddcutil [options] command"
    echo "  --noconfig"
    echo "  --noverify"
    echo "  --terse"
    exit 0
fi

if [[ "$*" == *"detect"* ]]; then
    if [[ "$*" == *"--terse"* ]]; then
        echo "Display 1"
        echo "   I2C bus:             /dev/i2c-2"
        echo "   DRM connector:       card0-DP-1"
        echo "   Monitor:             DEL:DELL U2720Q:123456"
        exit 0
    else
        echo "Display 1"
        echo "   I2C bus:             /dev/i2c-2"
        echo "   DRM connector:       card0-DP-1"
        echo "   Monitor:             DEL:DELL U2720Q:123456"
        exit 0
    fi
fi

if [[ "$*" == *"getvcp 10"* ]]; then
    if [[ "$*" == *"--terse"* ]]; then
        echo "VCP 10 C 50 100"
    else
        echo "VCP code 0x10 (Brightness): current value = 50, max value = 100"
    fi
    exit 0
fi

if [[ "$*" == *"setvcp 10"* ]]; then
    exit 0
fi

exit 1
