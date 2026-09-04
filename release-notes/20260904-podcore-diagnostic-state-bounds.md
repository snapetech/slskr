---
category: security
audience: users, operators
area: podcore-diagnostics
action: none
breaking: false
---
PodCore diagnostic dimension counters now reject malformed oversized labels and cap distinct domains, search types, and pod identifiers so high-cardinality activity cannot grow runtime memory without bound.
