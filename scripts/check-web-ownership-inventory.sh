#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

inventory="docs/dev/web-ownership-inventory.md"
failed=0

require() {
  local pattern="$1"
  local path="$2"
  local label="$3"
  if rg -n --fixed-strings -- "$pattern" "$path" >/dev/null; then
    printf 'PASS %s\n' "$label"
  else
    printf 'FAIL %s\n' "$label" >&2
    failed=1
  fi
}

require '`web/src/components/AppContext.js` is active.' "$inventory" \
  "AppContext ownership decision is recorded"
require '`AppRouteTable.jsx`, which owns lazy page imports, route definitions, and' "$inventory" \
  "route-table ownership decision is recorded"
require '`AppNavigationActivity.js` owns bounded room-activity storage, chat/room' "$inventory" \
  "navigation-activity ownership decision is recorded"
require '`web/src/components/Pods/Pods.jsx` has no production route import.' "$inventory" \
  "legacy Pods ownership decision is recorded"
require "import AppContext from './AppContext';" web/src/components/App.jsx \
  "App imports the active context"
require "import AppRouteTable from './AppRouteTable';" web/src/components/App.jsx \
  "App imports the route table"
require "<AppRouteTable" web/src/components/App.jsx \
  "App composes the route table"
require "import AppNavigationActivity from './AppNavigationActivity';" web/src/components/App.jsx \
  "App imports the navigation-activity owner"
require "this.navigationActivity.start()" web/src/components/App.jsx \
  "App starts navigation activity with its lifecycle"
require "this.navigationActivity.stop()" web/src/components/App.jsx \
  "App stops navigation activity with its lifecycle"
require "path=\"/pods\"" web/src/components/AppRouteTable.jsx \
  "the route table declares the /pods route boundary"
require "<Messaging" web/src/components/AppRouteTable.jsx \
  "Messaging is rendered by the route table"
require "export { default } from '../Pods/PortForwarding';" \
  web/src/components/PortForwarding/PortForwarding.jsx \
  "historical PortForwarding import remains an explicit alias"

if rg -n "import .*Pods.* from .*Pods/Pods|from './Pods/Pods'" \
  web/src/components/App.jsx web/src/components/PortForwarding web/src/components/Pods \
  --glob '!Pods.jsx' --glob '!Pods.test.jsx' >/dev/null; then
  printf 'FAIL a production component still imports the legacy Pods route implementation\n' >&2
  failed=1
else
  printf 'PASS no production component imports the legacy Pods route implementation\n'
fi

if [[ "$failed" -ne 0 ]]; then
  exit 1
fi

printf 'web ownership inventory check passed\n'
