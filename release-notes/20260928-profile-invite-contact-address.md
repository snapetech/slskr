---
category: fixed
audience: users
area: contacts
action: Recreate invites made with a masked display name.
breaking: false
---
Native profiles and invites now use the active account identity instead of the redacted session summary. The web contact adapter decodes bounded, unexpired invites into the username required by the contact API and displays username records. Imported contacts remain unverified; invites grant no authentication or share permission.
