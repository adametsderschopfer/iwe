# Authentication

Iwe connects directly to provider APIs. GitHub CLI and GitLab CLI are optional and unused. A
repository on GitHub does not register an OAuth application. Register Iwe separately with each
provider to enable browser sign-in. Users of a configured release do not register applications
themselves.

## GitHub

1. Open [GitHub OAuth applications](https://github.com/settings/developers).
2. Choose **New OAuth App**.
3. Name the application **Iwe** and use your project's homepage.
4. Supply a callback URL required by registration, such as your project's homepage.
5. Enable **Device Flow** in the application's settings.
6. Copy the public **Client ID** into IWE_GITHUB_CLIENT_ID.

The device flow does not use the registered callback or a client secret. Iwe requests a device code
and opens https://github.com/login/device. The dialog displays the code and waits for authorization.
Polling respects the provider's interval, slowdown responses, and expiration.

Iwe requests the repo scope to read Actions runs and control jobs in private repositories. This
GitHub OAuth scope grants broader repository permissions than Iwe uses. Iwe reads profiles, workflow
runs, and jobs. Explicit actions rerun jobs or cancel workflow runs. Review organizational access
restrictions when private repositories remain inaccessible.

The implementation follows GitHub's
[OAuth authorization documentation](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps).

## GitLab.com

1. Open [GitLab applications](https://gitlab.com/-/user_settings/applications).
2. Add an application named **Iwe**.
3. Set its redirect URI to http://127.0.0.1:46291/oauth/gitlab/callback.
4. Clear **Confidential** to register a public native client.
5. Select the **api** scope.
6. Copy the **Application ID** into IWE_GITLAB_CLIENT_ID.

Iwe uses authorization code flow with PKCE S256. Each attempt generates a fresh verifier and state
using operating-system randomness. The callback listener binds only to IPv4 loopback and closes when
authorization ends. The registered callback must match exactly. Another application using port 46291
prevents sign-in until that port becomes available. Requests with an unexpected state, host, method,
or callback path are rejected. The listener expires after five minutes.

No client secret is embedded, transmitted, or required. OAuth refresh replaces rotated credentials
in the system store before loading pipelines. The implementation follows GitLab's
[OAuth API documentation](https://docs.gitlab.com/api/oauth2/).

Previously authorized read_api credentials remain suitable for reading pipelines. Enable api in the
registered application and sign in again to authorize job actions. Refreshing an old credential
cannot expand its scope.

## Configure a development build

Set public Client IDs when running Iwe:

```sh
IWE_GITHUB_CLIENT_ID=your_public_github_id \
IWE_GITLAB_CLIENT_ID=your_public_gitlab_id \
make run
```

Release maintainers can set these variables while building to embed their public IDs. Runtime
environment variables override embedded IDs. Missing IDs display a configuration message when
browser sign-in starts. Do not add client secrets or personal access tokens to these variables or
repository files.

## Self-hosted servers

The remote determines the destination host and HTTPS port. Iwe automatically recognizes familiar
provider hostnames. For other hostnames, select GitLab or GitHub in the pipeline panel. The selected
provider is saved for that host. Credentials remain separate for each provider and host.

Self-hosted GitLab uses a personal access token with **api** for job actions. Choose **Sign in**,
open **Create a token**, then paste the token into Iwe. Use a personal token belonging to an account
with project access. Iwe validates it through the user API before saving. GitHub Enterprise servers
can similarly use a classic personal token with **repo**.

Existing read_api tokens can still display GitLab jobs. Replace them with api tokens to control
jobs. The account also needs the project's required role and any protected-environment permissions.

API requests require HTTPS with a valid certificate. Iwe does not downgrade transport, disable
certificate validation, or forward authorization through redirects. Servers installed beneath a URL
path prefix are not supported.

## Credential storage

| Platform | Storage            |
| -------- | ------------------ |
| macOS    | Keychain           |
| Windows  | Credential Manager |
| Linux    | Secret Service     |

No plaintext fallback exists. Unlock the system credential store if access fails. Linux requires a
running Secret Service. Workspace JSON contains project metadata, preferences, and provider choices
only.

Iwe reads each account from the credential store once per application session. Concurrent pipeline
and job requests share that result. Cached secrets remain in memory and clear when the session ends.
Token updates and sign-out update the cache only after the credential store accepts the operation.
Failed operations block further background reads. Use **Retry**, refresh the pipeline panel, or sign
in again after restoring access. Explicit retries preserve successfully cached credentials.

macOS can still request authorization when the credential store is locked or the application
identity changes. Local ad hoc signatures identify a specific build. Updates can therefore require
authorization again. Stable signing identities preserve application identity across updates. See
[Apple's code signing requirements](https://developer.apple.com/documentation/technotes/tn3127-inside-code-signing-requirements).

**Sign out** deletes the saved credential for the selected provider and server. Other projects using
that account will require sign-in after their next refresh. The provider's authorization grant
remains active until revoked through its account settings. Token fields clear after submission,
failure, or cancellation.

## Verification

Local fixtures verify protocol requests without contacting providers. Tests cover callback
validation, token refresh, invalid tokens, cancellation, response limits, and redirect rejection.
Job fixtures also verify pagination, stage order, action endpoints, permission errors, and
duplicate-submission guards. An in-memory credential store keeps test tokens outside the operating
system vault. Live browser sign-in requires registered Client IDs and user authorization. No live
provider sign-in is implied by passing fixture tests.
