#!/bin/bash

# Find the database file
DB_PATH="$HOME/Library/Application Support/com.personalassistant.app/personal_assistant.db"

if [ ! -f "$DB_PATH" ]; then
    echo "Database not found at: $DB_PATH"
    echo "Looking for database in other locations..."
    find ~ -name "personal_assistant.db" 2>/dev/null | head -5
    exit 1
fi

echo "Found database at: $DB_PATH"
echo "Database size: $(ls -lh "$DB_PATH" | awk '{print $5}')"
echo ""

# Check if activities table exists
echo "=== Checking activities table ==="
sqlite3 "$DB_PATH" ".tables" | grep -o "activities" || echo "Activities table not found!"

# Count total activities
echo ""
echo "=== Activity Statistics ==="
sqlite3 "$DB_PATH" "SELECT COUNT(*) as total_activities FROM activities;" 2>/dev/null || echo "Error querying activities"

# Get today's activities
echo ""
echo "=== Today's Activities ==="
sqlite3 "$DB_PATH" "
SELECT 
    COUNT(*) as count,
    SUM(duration_seconds) as total_seconds,
    ROUND(SUM(duration_seconds) / 3600.0, 2) as total_hours
FROM activities 
WHERE date(timestamp) = date('now', 'localtime');
" 2>/dev/null || echo "Error querying today's activities"

# Get recent activities
echo ""
echo "=== Last 10 Activities ==="
sqlite3 "$DB_PATH" "
SELECT 
    datetime(timestamp, 'localtime') as time,
    app_name,
    duration_seconds,
    is_productive
FROM activities 
ORDER BY timestamp DESC 
LIMIT 10;
" 2>/dev/null || echo "Error querying recent activities"

# Check for any activities in the last hour
echo ""
echo "=== Activities in Last Hour ==="
sqlite3 "$DB_PATH" "
SELECT 
    COUNT(*) as count,
    SUM(duration_seconds) as total_seconds
FROM activities 
WHERE timestamp > datetime('now', '-1 hour');
" 2>/dev/null || echo "Error querying recent hour"

# Show app usage breakdown for today
echo ""
echo "=== Today's App Usage ==="
sqlite3 "$DB_PATH" "
SELECT 
    app_name,
    COUNT(*) as sessions,
    SUM(duration_seconds) as total_seconds,
    ROUND(SUM(duration_seconds) / 60.0, 1) as total_minutes
FROM activities 
WHERE date(timestamp) = date('now', 'localtime')
GROUP BY app_name
ORDER BY total_seconds DESC
LIMIT 10;
" 2>/dev/null || echo "Error querying app usage"