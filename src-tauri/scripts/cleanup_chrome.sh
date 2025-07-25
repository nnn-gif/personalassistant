#!/bin/bash

echo "Chrome Profile Cleanup Script"
echo "============================"

# Parse command line arguments
CLEAN_PERSISTENT=false
FORCE_KILL=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --clean-persistent)
            CLEAN_PERSISTENT=true
            shift
            ;;
        --force-kill)
            FORCE_KILL=true
            shift
            ;;
        *)
            echo "Usage: $0 [--clean-persistent] [--force-kill]"
            echo "  --clean-persistent  Also clean persistent profile (use with caution!)"
            echo "  --force-kill        Kill all Chrome processes"
            exit 1
            ;;
    esac
done

# Kill Chrome processes if requested
if [ "$FORCE_KILL" = true ]; then
    echo "Looking for Chrome processes..."
    ps aux | grep -E "(chromiumoxide-runner|Chrome.*--remote-debugging-port)" | grep -v grep | while read -r line; do
        pid=$(echo "$line" | awk '{print $2}')
        echo "Killing Chrome process PID: $pid"
        kill -9 "$pid" 2>/dev/null
    done
fi

# Clean up temporary chromiumoxide session directories
echo "Cleaning up temporary session directories..."
find /tmp -name "chromiumoxide-session-*" -type d -mmin +5 -exec rm -rf {} + 2>/dev/null
find /private/var/folders -name "chromiumoxide-runner" -type d -exec rm -rf {} + 2>/dev/null

# Handle persistent profile
PERSISTENT_PROFILE="$HOME/Library/Application Support/PersonalAssistant/ChromeProfile"

if [ -d "$PERSISTENT_PROFILE" ]; then
    if [ "$CLEAN_PERSISTENT" = true ]; then
        echo "Cleaning persistent profile at: $PERSISTENT_PROFILE"
        read -p "Are you sure you want to delete the persistent profile? (y/N) " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            rm -rf "$PERSISTENT_PROFILE"
            echo "Persistent profile deleted."
        else
            echo "Skipping persistent profile deletion."
        fi
    else
        # Just clean up stale lock files in persistent profile
        if [ -f "$PERSISTENT_PROFILE/SingletonLock" ]; then
            echo "Found SingletonLock in persistent profile"
            # Check if Chrome is using this profile
            if ! ps aux | grep -E "Chrome.*--user-data-dir.*PersonalAssistant/ChromeProfile" | grep -v grep > /dev/null; then
                echo "No Chrome process using persistent profile, removing lock..."
                rm -f "$PERSISTENT_PROFILE/SingletonLock"
            else
                echo "Chrome is currently using the persistent profile."
            fi
        fi
    fi
fi

echo "Cleanup complete!"