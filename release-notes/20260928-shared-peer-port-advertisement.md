---
category: fixed
audience: operators
area: native-peer-networking
action: Use the same advertised peer port for plain and obfuscated traffic in the native current profile.
breaking: true
---
The native current profile now advertises obfuscated traffic on the same public peer port as plain traffic, including NAT mappings. Conflicting obfuscated advertised ports are rejected during configuration validation. No additional listener port is created.
