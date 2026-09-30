---
category: changed
audience: operators
area: native-web-builds
action: none
breaking: false
---
Native Web builds assemble ordered stylesheet source modules into the existing single `/styles.css` asset. The build command and browser requests remain the same, while the builder rejects missing or duplicate source modules instead of silently omitting styles.
