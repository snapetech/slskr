#!/usr/bin/env bash
#
# Check slskR route coverage against the webui endpoint list.
# Reads the canonical webui endpoint list from docs/webui-endpoints.txt
# and counts how many are implemented in crates/slskr/src/main.rs.
#
# Usage: ./scripts/diff-webui-endpoints.sh

set -e

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WEBUI_ENDPOINTS="$REPO_ROOT/docs/webui-endpoints.txt"
MAIN_RS="$REPO_ROOT/crates/slskr/src/main.rs"

if [[ ! -f "$WEBUI_ENDPOINTS" ]]; then
    echo "Error: $WEBUI_ENDPOINTS not found"
    exit 1
fi

if [[ ! -f "$MAIN_RS" ]]; then
    echo "Error: $MAIN_RS not found"
    exit 1
fi

echo "=== slskR WebUI Endpoint Coverage Report ==="
echo ""
TOTAL=$(wc -l < "$WEBUI_ENDPOINTS")
echo "Canonical webui endpoints: $TOTAL routes"
echo "Scanning slskr implementation..."
echo ""

IMPLEMENTED=0
MISSING=()

while IFS=' ' read -r method path; do
    [[ -z "$method" ]] && continue
    
    # Normalize path for matching (remove query string, replace variables)
    norm_path=$(echo "$path" | sed -E 's/\?.*$//' | sed -E 's/:[a-z]+/:var/g' | sed -E 's/\$\{[^}]+\}/:var/g')
    
    # Try different patterns in main.rs
    if grep -q "\"$method\".*\"$path\"" "$MAIN_RS" || \
       grep -q "\"$method\".*\"$norm_path\"" "$MAIN_RS" || \
       grep -q "\"$method\".*\"/api$norm_path\"" "$MAIN_RS" || \
       grep -q "\"$method\".*\"/api/v0$norm_path\"" "$MAIN_RS" || \
       grep -q "starts_with.*\"$path\"" "$MAIN_RS" || \
       grep -q "ends_with.*\"$path\"" "$MAIN_RS" || \
       grep -q "path == \"$path\"" "$MAIN_RS"; then
        ((IMPLEMENTED++))
        echo "✓ $method $path"
    else
        MISSING+=("$method $path")
        echo "✗ $method $path"
    fi
done < "$WEBUI_ENDPOINTS"

# Summary
echo ""
echo "=== Summary ==="
echo "Implemented:  $IMPLEMENTED / $TOTAL"
PERCENT=$((IMPLEMENTED * 100 / TOTAL))
echo "Coverage:     $PERCENT%"
MISSING_COUNT=$((TOTAL - IMPLEMENTED))
echo "Missing:      $MISSING_COUNT"
echo ""

if [[ ${#MISSING[@]} -gt 0 ]]; then
    echo "First 10 missing endpoints:"
    for ((i=0; i<${#MISSING[@]} && i<10; i++)); do
        echo "  ${MISSING[$i]}"
    done
    echo ""
    echo "To implement more endpoints, add handlers to crates/slskr/src/main.rs"
    exit 1
else
    echo "All webui endpoints are implemented! ✓"
    exit 0
fi
