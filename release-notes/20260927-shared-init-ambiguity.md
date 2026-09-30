---
category: fixed
audience: operators
area: peer-listener
action: Reconnect rejected obfuscated peers to generate a fresh handshake key.
breaking: true
---
The shared peer listener now closes connections whose initialization prefix can represent both a plain and obfuscated handshake. Ordinary handshakes continue to use the same port. Peers that send the ambiguous legacy byte sequence must retry with a different obfuscation key.
