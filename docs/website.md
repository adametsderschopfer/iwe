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

`DownloadSection.vue` links to verified packages from release `0.1.11`. Update its version when
selecting a newer release. The release-notes link always opens the latest release.

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
3. Verify generated pages and their local assets.
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
