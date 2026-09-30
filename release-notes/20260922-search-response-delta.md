---
category: changed
audience: operators
area: search-persistence
action: none
breaking: false
---
Accepted search responses now append only newly admitted result rows while
updating the search projection transactionally, avoiding repeated full
delete-and-reinsert work during large response bursts.
