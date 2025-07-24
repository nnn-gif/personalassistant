#!/bin/bash

echo "🔍 Debugging Personal Assistant App"
echo "===================================="

# 1. Check if processes are running
echo -e "\n1️⃣ Checking processes:"
if pgrep -f "personalassistant" > /dev/null; then
    echo "✅ personalassistant process is running"
    ps aux | grep personalassistant | grep -v grep
else
    echo "❌ personalassistant process not found"
fi

if pgrep -f "node.*vite" > /dev/null; then
    echo "✅ Vite dev server is running"
else
    echo "❌ Vite dev server not found"
fi

# 2. Check ports
echo -e "\n2️⃣ Checking ports:"
if lsof -i :5173 > /dev/null 2>&1; then
    echo "✅ Port 5173 (Vite) is in use"
else
    echo "❌ Port 5173 is not in use"
fi

# 3. Check if we can reach the frontend
echo -e "\n3️⃣ Testing frontend:"
if curl -s http://localhost:5173 > /dev/null; then
    echo "✅ Frontend is accessible at http://localhost:5173"
else
    echo "❌ Cannot reach frontend"
fi

# 4. Check for Tauri windows
echo -e "\n4️⃣ Looking for app windows:"
osascript -e 'tell application "System Events" to get name of every process whose name contains "personalassistant"' 2>/dev/null

# 5. Common issues
echo -e "\n5️⃣ Common issues to check:"
echo "- The window might be minimized or hidden"
echo "- Try pressing Cmd+Tab to see if the app is running"
echo "- Check Mission Control (F3) for the window"
echo "- The app might be on a different desktop/space"

echo -e "\n6️⃣ Quick fixes to try:"
echo "1. Kill all processes: killall node cargo personalassistant"
echo "2. Clear node_modules: rm -rf node_modules && npm install"
echo "3. Clear Rust target: cd src-tauri && cargo clean"
echo "4. Run with logging: RUST_LOG=debug npm run tauri dev"