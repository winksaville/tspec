# Notes

This directory contains various notes and documentation related to the project. Each file is organized by topic for easy reference.

By default there are chores-*.md and todo.md. Chores are general notes
about maintenance tasks and todo.md contains short term tasks and their status.

As the number of chores files increase they should be put archived in /notes/chores/
although links will need to be updated. I expect we may want to create a "notes"
database in the furture to better manage the information, TBD.

Examples chore file:
```
# Chores-1.md
 
General maintenance tasks and considerations for the project see other files for more specific topics. Chores generally don't neeed detailed explanations or dicussions, they can but if the expand to much a speararte file should be created.

## 20260202 - Should tspec augment or replace cargo

> Tspec could either augment cargo by providing additional functionality on top of it, or it could replace cargo entirely by offering a new way to manage Rust projects. The decision depends on the specific needs of the project and the desired user experience.
```


Todo.md contains two main sections "Todo" and "Done" each item is a
short explanations of a tasks and links to more details using 1 or more
references.

Multiple references must be separated: `[2],[3]` not `[2,3]` or `[2][3]`.
In markdown, `[2,3]` is a single ref key (won't resolve) and `[2][3]`
is parsed as display text `2` with ref key `3` (so `[2]` won't resolve).

Examples:

# Todo
- Add new feature X [details](features.md#feature-x)
- Fix bug Y [1]

# Done
- Fixed issue Z [2],[3]

[1]: bugs.md#bug-y
[2]: issues.md#issue-z
[3]: fixes.md#fix-z