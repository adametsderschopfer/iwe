# Documentation and writing

Follow these rules when [AGENTS.md](../../AGENTS.md) directs you here.

## Documentation & Architectural Decisions

Documentation is a required part of application development. Update documentation when changing
application behavior, configuration, storage, architecture, or development workflows. Include
documentation in the same commit or pull request. Complete documentation before declaring the change
finished.

Before implementation, read the relevant project documentation. After implementation, update the
existing documentation or create the necessary document if none exists.

Maintain the following sources of truth:

- `README.md` — project overview, setup, dependencies, supported platforms, configuration, run,
  build, and verification commands.
- `docs/architecture.md` — the implemented system structure, module responsibilities, data flow, and
  dependency boundaries.
- `docs/<feature>.md` — explain feature purpose, user scenarios, behavior, data models, interfaces,
  errors, and limitations. Describe persistence and migrations when applicable.
- `docs/adr/` — lightweight Architecture Decision Records for significant architectural decisions:
  context, chosen approach, alternatives, trade-offs, and consequences.

Documentation rules:

- Follow [user-facing content rules](user-facing-content.md) when writing interface messages or
  describing their behavior.
- Apply [i18n rules](i18n.md) to application text. Maintain English, Russian, and Spanish catalogs
  together.
- Write project documentation and code comments primarily in English. This includes README,
  architecture documents, feature documents, ADRs, Rustdoc, and inline comments.
- Russian is the secondary language. Put English before Russian when providing both languages.
  Alternatively, link from the English document to a Russian translation.
- Write new documentation and comments in English first. Translate affected Russian-only
  documentation or comments into English during substantive updates. Retain Russian as a secondary
  translation where useful.
- Keep translations consistent with the English version; update affected translations in the same
  change. This language rule does not determine the application's UI language or the language used
  to communicate with the user.
- Describe the actual implementation. Clearly distinguish planned behavior from working features.
- Update affected instructions, examples, links, and diagrams when behavior or structure changes.
- Keep platform differences and known limitations explicit.
- Keep one authoritative description of each topic and link to it rather than duplicating it.
- Verify documented commands and examples where practical; report anything that could not be
  verified.
- Create documents when they contain useful information; do not create empty templates or
  directories.
- Do not duplicate obvious code or document trivial implementation details.
- Skip unrelated documentation changes when behavior and workflows remain unchanged.
- Review documentation alongside the code before declaring work complete. Run documentation
  formatting checks. Describe documentation changes in the completion report.

### Clear Technical Writing

Use the user's five writing rules inspired by ASD-STE100. Apply these rules to new and changed
responses, documentation, and code comments.

1. Use active voice. Name the actor when the action could otherwise be unclear.
2. Limit each sentence to 20 words.
3. Express one idea per sentence.
4. Use one consistent term for each concept. Do not alternate synonyms for the same concept.
5. Use plain, literal language. Avoid idioms and jargon.

Keep API names, commands, and code identifiers exact. Explain required technical terms when you
introduce them. Preserve code and command syntax when editing prose. Review new and changed text
against these rules before completing a task.
