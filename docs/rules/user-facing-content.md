# User-facing content

Follow these rules when [AGENTS.md](../../AGENTS.md) directs you here.

## Required behavior

- Show only information that helps users understand their content, complete a task, or recover from
  a problem.
- Do not show implementation details in the interface.
- Apply this rule to screens, titles, dialogs, notifications, tooltips, placeholders, accessibility
  descriptions, and errors.
- Do not display framework names, rendering backends, database details, internal paths, stack
  traces, or exception messages.
- Do not display HTTP status codes, internal identifiers, translation keys, debug output, or raw API
  responses.
- Do not add developer status panels, technical badges, or implementation explanations to product
  screens.
- Describe problems through their effect on the user's task.
- Give a clear recovery action when one exists.
- Do not invent causes, successful outcomes, or recovery options.
- Keep all user-facing messages in English, Russian, and Spanish i18n catalogs.
- Keep user-authored content unchanged. This rule limits application-authored text.

## Errors and diagnostics

Map internal errors to deliberate user-facing messages before rendering them. Never pass raw errors,
diagnostic messages, or debug representations directly into widgets. Keep technical details in
developer diagnostics outside the product interface. The current startup launcher writes such
diagnostics to stderr. Do not copy that diagnostic output into dialogs, notifications, or other
interface elements. Exclude credentials, tokens, and private user content from diagnostic output.

Use a localized, nontechnical fallback when the application cannot show a specific message. Never
reveal a translation key or formatter error as fallback text. If no valid catalog remains, omit
unavailable text rather than expose internal identifiers.

## Examples

| Situation                    | User-facing message                                        | Developer diagnostics only                |
| ---------------------------- | ---------------------------------------------------------- | ----------------------------------------- |
| A save operation fails       | We could not save your changes. Try again.                 | Database errors and internal file paths   |
| A network request fails      | We could not connect. Check your connection and try again. | HTTP codes and transport errors           |
| A translation cannot resolve | This text is unavailable.                                  | Message identifiers and formatting errors |

These examples guide wording. Add translated catalog entries when implementing the corresponding
behavior.

## Review

- Review every new user-facing string for technical details.
- Check error paths and fallback messages, not only successful states.
- Add regression tests when an error path could expose internal details.
- Keep diagnostic catalog entries separate from messages intended for the interface.
