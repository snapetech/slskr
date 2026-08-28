#!/usr/bin/env bash
#
# Check slskR route coverage against the webui endpoint list.
# Reads the canonical webui endpoint list from docs/webui-endpoints.txt
# and counts how many are implemented in crates/slskr/src/lib.rs.
#
# Usage: ./scripts/diff-webui-endpoints.sh

set -e

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WEBUI_ENDPOINTS="$REPO_ROOT/docs/webui-endpoints.txt"
SOURCE_RS="$REPO_ROOT/crates/slskr/src/lib.rs"

if [[ ! -f "$WEBUI_ENDPOINTS" ]]; then
    echo "Error: $WEBUI_ENDPOINTS not found"
    exit 1
fi

if [[ ! -f "$SOURCE_RS" ]]; then
    echo "Error: $SOURCE_RS not found"
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
    if [[ "$norm_path" == /* ]]; then
        api_norm_path="/api$norm_path"
    else
        api_norm_path="/api/$norm_path"
    fi
    dynamic_prefix=""
    api_dynamic_prefix=""
    if [[ "$norm_path" == *":var"* ]]; then
        dynamic_prefix="${norm_path%%:var*}"
        api_dynamic_prefix="${api_norm_path%%:var*}"
    fi
    
    # Try different patterns in the daemon implementation source.
    if grep -q "\"$method\".*\"$path\"" "$SOURCE_RS" || \
       grep -q "\"$method\".*\"$norm_path\"" "$SOURCE_RS" || \
       grep -q "\"$method\".*\"$api_norm_path\"" "$SOURCE_RS" || \
       grep -q "\"$method\".*\"/api/v0$norm_path\"" "$SOURCE_RS" || \
       grep -q "starts_with.*\"$path\"" "$SOURCE_RS" || \
       grep -q "ends_with.*\"$path\"" "$SOURCE_RS" || \
       grep -q "path == \"$path\"" "$SOURCE_RS" || \
       { [[ -n "$api_dynamic_prefix" ]] && grep -q "path_segment_after(path, \"$api_dynamic_prefix" "$SOURCE_RS"; } || \
       { [[ -n "$api_dynamic_prefix" ]] && grep -q "starts_with.*\"$api_dynamic_prefix" "$SOURCE_RS"; } || \
       { [[ -n "$dynamic_prefix" ]] && grep -q "starts_with.*\"$dynamic_prefix" "$SOURCE_RS"; }; then
        ((++IMPLEMENTED))
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
    echo "To implement more endpoints, add handlers to crates/slskr/src/lib.rs"
    exit 1
else
    echo "All webui endpoints are implemented! ✓"
    exit 0
fi
