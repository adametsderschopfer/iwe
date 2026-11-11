# Release notes

Read this rule before preparing a commit, application version, release, or website release-note
change.

## Source of truth

Store authored notes in `website/app/release-notes/releases.json`. The website renders the version
index and individual version pages from this file. GitHub release publication renders its English
body from the same items. Do not replace authored notes with generated commit-title lists.

Keep each item factual, concise, and useful to application users. Maintain English, Russian, and
Spanish text together. Use `added`, `changed`, `fixed`, or `maintenance` for its category. Give each
item a stable, unique identifier. Record inspected commit identifiers or repository-relative source
paths in `sources`. Never infer completed features, publication dates, package availability, or
successful checks.

## Every commit request

1. Inspect the exact changes authorized for that commit.
2. Create or update corresponding items in the `unreleased` record before committing.
3. Describe the resulting behavior while the implementation evidence remains available.
4. Include those items in the same commit as their implementation.
5. Preserve unrelated drafts outside that commit's scope.
6. Reuse existing items when they already describe the same change. Do not duplicate them.
7. Record factual maintenance work when a commit contains no visible behavior change.
8. Do not create an item for an empty or canceled commit request.

Release metadata commits can update their existing version record without duplicating its change
items. A commit request authorizes neither a version bump nor release publication by itself. Follow
the user's current restrictions on commits, tests, builds, pushes, and deployment.

An unreleased item has this shape:

```json
{
  "id": "clear-branch-selection-count",
  "category": "added",
  "text": {
    "en": "The branch list shows how many branches are selected.",
    "ru": "Список веток показывает количество выбранных веток.",
    "es": "La lista de ramas muestra cuántas ramas están seleccionadas."
  },
  "sources": ["src/ui/screens/git.rs"]
}
```

## Every version request

1. Read the accumulated unreleased items and compare them with the actual release scope.
2. Create a record whose version exactly matches Cargo.toml and the intended stable tag.
3. Move the included items into that record. Leave excluded work under `unreleased`.
4. Use `prepared` until publication succeeds. Keep unverified publication dates and tags absent.
5. Review all three translations and the website's version page.
6. Update download links only after verifying the corresponding published packages.
7. Run `python3 .github/scripts/release-notes.py --check --version VERSION`.
8. Complete the authorized checks, commit, tag, and publication steps.
9. Verify the actual GitHub release before setting `released`, `publishedAt`, and its public tag.
10. Update the website metadata through the authorized commit workflow after publication.

A version request includes preparation of its release notes. Do not postpone writing them until
after publication. Use one record and one public tag for each new version. Historical development
versions use `development` and do not claim published downloads. The unreleased record always uses
`prepared`, a null publication date, and an empty tag list. Keep already published notes intact
unless correcting a verified error.

## Verification and publication

Run `python3 .github/scripts/release-notes.py --check` after changing notes. This static validation
checks versions, item identifiers, categories, translations, and source references. It does not
prove that descriptions match implementation or that runtime checks passed.

CI validates the ledger during planning. Version tags also require a matching prepared or released
record with items. A missing release note stops release jobs before builds begin. The publication
script supplies the authored English body with `gh release create --notes-file`. Each body links to
the version's English, Russian, and Spanish website pages.

Publish the release only after packages for all six targets are available. Follow
[the CI guide](../ci.md#publish-a-version).

Follow [website instructions](../website.md) for formatting, lint, type checks, and route
verification. Do not run forbidden checks merely to complete this workflow.
