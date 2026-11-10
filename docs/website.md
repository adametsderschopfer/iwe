# Website

The public website introduces IWE Code and links to GitHub releases. The footer credits
[vamez.ru](https://vamez.ru) as the developer.

Source files live in `website/`, independently from the Rust application. The website uses Nuxt with
Vue and TypeScript. Nuxt generates static HTML for GitHub Pages. This website-specific toolchain
does not change the native application toolchain.

The default address is
[adametsderschopfer.github.io/iwe-code](https://adametsderschopfer.github.io/iwe-code/). The website
does not configure a custom domain.

## Local development

Use Node.js 24.15.0 or newer within the 24.x release line. The website keeps its dependencies in
`website/package-lock.json`.

Run these commands from `website/`:

```sh
npm ci
npm run dev
```

Nuxt prints the local preview address.

The website uses Prettier for its files. The root dprint configuration excludes `website/` to keep
one formatter responsible for each file.

Run these checks after changes:

```sh
npm run format:check
npm run lint
npm run typecheck
npm audit --audit-level=moderate
npm run generate
npm test
npx playwright install chromium
npm run test:browser
```

Generation writes static files to `website/.output/public`. The generated-page tests require that
output. Set `NUXT_APP_BASE_URL=/iwe-code/` when checking the GitHub Pages deployment path locally.
Use your shell's environment syntax before generation and generated-page tests.

Run root `make check` and `make build` before committing or pushing repository changes.

## Content and assets

Keep English, Russian, and Spanish website text together in the website's translation catalogs.
Update every translation when changing website text. Use product screenshots that show current
application behavior. Exclude credentials and private project content from screenshots.

Catalogs live in `website/app/i18n/`. The root route uses English. `/ru/` and `/es/` provide Russian
and Spanish pages. Each page includes its language, canonical address, and alternate-language links.

The screenshots come from existing Iced rendering tests with fictional fixture projects. They show
the actual application interface in English. The website opens in English by default. Regenerate the
screenshots when the illustrated interface changes.

The feature section covers editing, deployment, Git, pipelines, the terminal, and spaces. A separate
deployment section explains connections, comparisons, and manual transfers. The workspace section
explains optional modules. All text appears in English, Russian, and Spanish. The gallery identifies
its screenshots as development-build previews. Published downloads may differ.

`app::tests::render_website_screenshots_when_requested` renders the five website views from
fictional English fixtures. Set `IWE_RENDER_DIR` to a new temporary directory and run that exact
rendering target when authorized. Convert its PNG outputs to the five `public/images/*-en.webp`
files with lossless `cwebp` encoding. Keep the current image dimensions and inspect every capture
before replacing website assets. This rendering target does not read project files, contact
providers, or authenticate accounts. Respect explicit user restrictions on compiling or running
tests before invoking it.

The captures show the editable workspace, Git history, comparison, pipelines, and deployment
settings. Deployment settings use a fictional Staging profile and an invalid example hostname. The
gallery preserves each complete screenshot at every viewport width. Its buttons scroll horizontally
on small screens.

All website bitmap images use lossless WebP, including the product icon. Generated-page tests reject
other bitmap formats in the published files.

`DownloadSection.vue` links to verified packages from release `0.1.11`. Update its version when
selecting a newer release. The release-notes link opens that download version's page on the website.

The download cards use Apple, Windows, and Tux SVG paths from
[Bootstrap Icons v1.13.1](https://github.com/twbs/icons/tree/v1.13.1). The website embeds these
paths locally and uses the card text color. The MIT license remains in
`website/licenses/bootstrap-icons.txt`.

The download section detects desktop systems in the browser after the page loads. It moves the
matching system first and highlights its card. Explicit browser architecture hints select a matching
package. The website does not infer a Mac processor from the browser's Intel identification. Without
reliable architecture hints, macOS prioritizes Apple Silicon and keeps Intel available. Mobile
devices keep all desktop downloads available without a detected-system badge. ARM devices without a
matching package receive a compatibility note. Detection never starts a download.

Selected sections appear once when they enter the viewport. The reveal uses CSS transitions and
`IntersectionObserver`. The website honors reduced-motion preferences and reveals keyboard-focused
content immediately. Content remains visible without JavaScript or observer support. The image
reveal clips its image, while the observed container remains unclipped. Browser regression tests
cover scrolling, navigation links, reduced motion, keyboard focus, and screenshot selection in all
three languages.

## Release notes

The header and footer link to `/releases/`, `/ru/releases/`, or `/es/releases/`. Each version has
its own page beneath that route, such as `/ru/releases/0.1.12/`. Language links preserve the
selected version. Unknown versions return the localized error page. Nuxt explicitly prerenders every
version and language for GitHub Pages.

`app/release-notes/releases.json` contains authored changes, translations, and inspected source
references. The index includes public releases and identified historical development versions. The
`unreleased` page marks current drafts as unpublished. Source references remain authoring metadata,
outside the displayed change text. Publication dates and package links appear only when verified.
Platform releases share one version page.

Follow [release-note rules](rules/release-notes.md) on every commit and version request. Commit
preparation collects items under `unreleased`. Version preparation moves reviewed items into the
matching version record. CI validates that record before release builds. GitHub publication uses the
same authored notes instead of generated commit titles.

## License

The footer links to `/license/`, `/ru/license/`, or `/es/license/`. Each page localizes its
introduction and navigation while displaying the original English license. `LicensePage.vue` imports
the root `LICENSE` as raw text. Do not maintain a separate copy or rewrite its legal text. Language
links preserve the license page. Nuxt explicitly prerenders all three routes for GitHub Pages. The
website workflow also runs when the root `LICENSE` changes.

## Dependency security

The lockfile uses patched `minimatch` and `glob` versions through scoped npm overrides. Their
upstream consumers still request affected versions. Keep the overrides until upstream dependencies
adopt patched versions. Run all website checks and generation after changing these overrides.

The November 2026 audit reports seven low-severity dependency entries from one `deepmerge` advisory.
[GHSA-gcqx-w48m-w2h4](https://github.com/advisories/GHSA-gcqx-w48m-w2h4) has no patched release.
`@rollup/plugin-node-resolve` uses `deepmerge` once to copy constant internal defaults. This call
receives no visitor input or project configuration objects. The website publishes static files and
ships no server dependency runtime. CI still audits the complete dependency tree and rejects
moderate, high, and critical advisories. Recheck this assessment when dependencies or deployment
behavior change.

## Automatic deployment

The [Website workflow](../.github/workflows/website.yml) runs for website changes on `main` and pull
requests. Maintainers can also start the workflow manually.

The workflow runs these stages:

1. Check formatting, lint, TypeScript types, and dependency advisories in separate jobs.
2. Generate static files after every check passes.
3. Verify generated pages, local assets, and browser interactions.
4. Upload the static files and deploy them from `main`.

Pull requests run checks and generation without deploying. Deployment uses the `github-pages`
environment. Only the deployment job receives Pages write permission and identity-token permission.
The workflow pins external actions to commit identifiers.

The build reads the Pages base path through `actions/configure-pages`. Pull requests use the
repository name as their base path. Nuxt receives this path through `NUXT_APP_BASE_URL`. Generated
asset URLs must retain that path.

The existing native application workflow remains independent. Website deployment does not publish
application releases.

## Repository configuration

A repository administrator must enable GitHub Pages before the first deployment. In repository
settings, select **Pages**, then choose **GitHub Actions** under **Build and deployment**. The
repository uses its default GitHub Pages address. No deployment secret or custom domain is required.

For API setup, an administrator can create the Pages configuration:

```sh
gh api --method POST repos/adametsderschopfer/iwe-code/pages -f build_type=workflow
```

Use `PUT` instead of `POST` when updating an existing Pages configuration. Fine-grained credentials
require Pages write permission and Administration write permission for this setup. The workflow's
default token cannot enable Pages itself.

The first deployment creates the `github-pages` environment when necessary. Limit that environment's
deployment branches to `main` in repository settings.

Review the Website workflow result before sharing a deployment address. Investigate failed checks
before rerunning or publishing.

See
[GitHub's workflow guide](https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages)
and [Nuxt's GitHub Pages guide](https://nuxt.com/deploy/github-pages) for the upstream deployment
contracts.
