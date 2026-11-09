# Git actions

Git actions operate on the repository that owns the selected branch. Selecting history references
alone never changes the checked-out branch. Use the branch toolbar or right-click a branch to
execute an action. Dialogs retain the original repository when other selections change.

Projects can contain repositories in immediate subdirectories. Their branch groups and history rows
identify the owning repository. **Fetch all remotes** visits every discovered repository. Each
repository receives its own completion notification. Other actions affect only the selected branch's
repository.

The branch toolbar places equally sized action buttons above the branch tree. It stays visible while
the tree scrolls. The scrollbar reaches the panel's right edge without covering branch names.

| Action            | Behavior                                                                                      |
| ----------------- | --------------------------------------------------------------------------------------------- |
| Fetch all remotes | Fetch every configured remote using Git's fetch-all behavior.                                 |
| Checkout          | Switch to a local branch, create a tracking branch, or detach at a tag.                       |
| New branch from…  | Create a named branch from the selected reference. Switching is optional.                     |
| Update selected   | Update the selected local branch from its upstream using fast-forward changes only.           |
| Push              | Push the selected local branch to the chosen remote and destination branch. Set its upstream. |
| Rename            | Rename the selected local branch without replacing an existing branch.                        |
| Delete            | Confirm deletion of the selected local or remote branch.                                      |

Updating an inactive branch keeps the current checkout unchanged. Updating a remote reference
fetches its remote. Updates reject branches checked out in another worktree. Diverged histories
require resolution outside IWE Code. Checkout and current-branch updates require clean working
files. Creating a branch without switching can preserve existing working changes.

Creation and renaming validate branch names before changing references. Checkout does not overwrite
an existing local tracking branch or ignored local files. Push never forces updates or implicitly
includes annotated tags. Selected pushes and remote deletions disable configured repository
mirroring. Local deletion rejects the current branch and branches Git considers unmerged. Remote
deletion explicitly identifies its remote effect in the confirmation dialog. Tags cannot be updated,
renamed, pushed, or deleted through these branch controls.

Pending actions disable further Git actions. Failures retain dialog inputs and never retry the
operation automatically. Refresh repository state before retrying an ambiguous failure. Subprocesses
use a 180-second deadline and bounded output. A timeout can occur after a remote accepts an action.

Completed operations show a notification with the affected branch or project. Failed operations show
a localized error notification and preserve any open dialog. Obsolete or duplicate responses never
produce new notifications.

Git uses existing local configuration, SSH agents, and credential helpers. Pipeline account sign-in
does not configure Git authentication. Interactive terminal prompts are disabled. Configure Git
access outside IWE Code before using remotes.

Automated tests use temporary repositories and local bare remotes. They cover branch operations,
dirty files, conflicting histories, linked worktrees, dialogs, and stale results. These tests never
push to a real provider or change the user's repositories.
