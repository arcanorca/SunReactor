#!/usr/bin/env bash
# Fake ddcutil 2.1.2 (Modern)

if [[ "$*" == *"--version"* ]]; then
    echo "ddcutil 2.1.2"
    exit 0
fi

if [[ "$*" == *"--help"* ]]; then
    echo "Usage: ddcutil [options] command"
    echo "  --noconfig"
    echo "  --noverify"
    echo "  --terse"
    echo "  --brief"
    exit 0
fi

if [[ "$*" == *"detect"* ]]; then
    if [[ "$*" == *"--terse"* ]] || [[ "$*" == *"--brief"* ]]; then
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
    if [[ "$*" == *"--terse"* ]] || [[ "$*" == *"--brief"* ]]; then
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
