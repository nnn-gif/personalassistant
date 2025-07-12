# Audio Playback Fix

## Issue
The audio recordings were being created successfully but playback was failing with:
- "Failed to load resource: unsupported URL"
- "NotSupportedError: The operation is not supported"

## Solution
1. Updated the playRecording function to:
   - Fetch the audio file using the converted URL
   - Create a blob from the response
   - Create a blob URL for playback
   - Clean up the blob URL when done

2. Updated CSP (Content Security Policy) in tauri.conf.json to allow:
   - media-src for asset: and blob: protocols
   - Proper permissions for audio playback

## Code Changes
- Modified `playRecording` to be async and use blob URLs
- Added proper error handling and logging
- Updated button onClick to handle async function with `void`

## Testing
1. The app should now properly play audio recordings
2. Check console for "Audio URL:" log to see the converted URL
3. Recordings should play when clicking the play button
4. Duration should show correctly (not 0:00)