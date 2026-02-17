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


## Versioning during development

Every plan must start with a version bump. Choose the approach based on scope:

- **Single-step** (recommended for mechanical/focused changes): bump directly to
  `X.Y.Z`, implement in one commit. Simpler history.
- **Multi-step** (for exploratory/large changes): bump to `X.Y.Z-devN`, implement
  across multiple commits, final commit removes `-devN`.

The plan should recommend one approach and get user approval before starting.

For multi-step:
1. Bump version to `X.Y.Z-devN` with a plan and commit as a chore marker
2. Implement in one or more `-devN` commits (bump N as needed)
3. Final commit removes `-devN`, updates todo/chores — this is the "done" marker

The final release commit (without `-devN`) signals completion rather than amending
prior commits. This keeps the git history readable and makes it easy to see which
commits were exploratory vs final.

## Todo format

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