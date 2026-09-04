---
category: fixed
audience: users, operators
area: chat-workflows
action: none
breaking: false
---

Chat hydration, conversation polling, message sends, and deletion now respect view lifecycles; deleting a conversation issues one backend delete, and acknowledgement failures no longer hide successfully loaded messages.
