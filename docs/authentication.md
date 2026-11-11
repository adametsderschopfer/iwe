# Authentication

IWE Code connects directly to provider APIs. GitHub CLI and GitLab CLI are optional and unused. A
repository on GitHub does not register an OAuth application. Register IWE Code separately with each
provider to enable browser sign-in. Users of a configured release do not register applications
themselves.

## GitHub

1. Open [GitHub OAuth applications](https://github.com/settings/developers).
2. Choose **New OAuth App**.
3. Name the application **IWE Code** and use your project's homepage.
4. Supply a callback URL required by registration, such as your project's homepage.
5. Enable **Device Flow** in the application's settings.
6. Copy the public **Client ID** into IWE_GITHUB_CLIENT_ID.

The device flow does not use the registered callback or a client secret. IWE Code requests a device
code and opens https://github.com/login/device. The dialog displays the code and waits for
authorization. Polling respects the provider's interval, slowdown responses, and expiration.

IWE Code requests the repo scope to read Actions runs and control jobs in private repositories. This
GitHub OAuth scope grants broader repository permissions than IWE Code uses. IWE Code reads
profiles, workflow runs, and jobs. Explicit actions rerun jobs or cancel workflow runs. Review
organizational access restrictions when private repositories remain inaccessible.

The implementation follows GitHub's
[OAuth authorization documentation](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps).

## GitLab.com

1. Open [GitLab applications](https://gitlab.com/-/user_settings/applications).
2. Add an application named **IWE Code**.
3. Set its redirect URI to http://127.0.0.1:46291/oauth/gitlab/callback.
4. Clear **Confidential** to register a public native client.
5. Select the **api** scope.
6. Copy the **Application ID** into IWE_GITLAB_CLIENT_ID.

IWE Code uses authorization code flow with PKCE S256. Each attempt generates a fresh verifier and
state using operating-system randomness. The callback listener binds only to IPv4 loopback and
closes when authorization ends. The registered callback must match exactly. Another application
using port 46291 prevents sign-in until that port becomes available. Requests with an unexpected
state, host, method, or callback path are rejected. The listener expires after five minutes.

No client secret is embedded, transmitted, or required. OAuth refresh replaces rotated credentials
in the local file before loading pipelines. The implementation follows GitLab's
[OAuth API documentation](https://docs.gitlab.com/api/oauth2/).

Previously authorized read_api credentials remain suitable for reading pipelines. Enable api in the
registered application and sign in again to authorize job actions. Refreshing an old credential
cannot expand its scope.

## Configure a development build

Set public Client IDs when running IWE Code:

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

The remote determines the destination host and HTTPS port. IWE Code automatically recognizes
familiar provider hostnames. For other hostnames, select GitLab or GitHub in the pipeline panel. The
selected provider is saved for that host. Credentials remain separate for each provider and host.

Self-hosted GitLab uses a personal access token with **api** for job actions. Choose **Sign in**,
open **Create a token**, then paste the token into IWE Code. Use a personal token belonging to an
account with project access. IWE Code validates it through the user API before saving. GitHub
Enterprise servers can similarly use a classic personal token with **repo**.

Existing read_api tokens can still display GitLab jobs. Replace them with api tokens to control
jobs. The account also needs the project's required role and any protected-environment permissions.

API requests require HTTPS with a valid certificate. IWE Code does not downgrade transport, disable
certificate validation, or forward authorization through redirects. Servers installed beneath a URL
path prefix are not supported.

## Credential storage

IWE Code saves provider tokens and deployment secrets in separate files under a local secrets
directory. Debug and release builds use the same storage. Each provider and host has its own file.
Workspace metadata and project files never contain these credentials.

| Platform | Default secrets directory                   |
| -------- | ------------------------------------------- |
| macOS    | `~/Library/Application Support/iwe/secrets` |
| Windows  | `%LOCALAPPDATA%\iwe\data\secrets`           |
| Linux    | `$XDG_DATA_HOME/iwe/secrets`                |

Linux defaults to `~/.local/share/iwe/secrets` when `XDG_DATA_HOME` is unavailable. `IWE_DATA_DIR`
places the secrets directory under the specified root. Keep that root private. On macOS and Linux,
the directory uses mode `0700` and credential files use mode `0600`. On Windows, files inherit
access permissions from their directory. Files contain unencrypted credentials. Programs running as
the same user can read them.

After updating from a version that used the system credential store, sign in to each provider again.
Re-enter and save any deployment passwords or SSH key passphrases. IWE Code does not read or remove
the old entries. You can remove them separately through your operating system.

IWE Code caches each account read within the application session. Concurrent requests share that
result. Token updates replace the file atomically. Failed operations block further background reads.
Use **Retry**, refresh the pipeline panel, or sign in again after restoring access. Explicit retries
preserve successfully cached credentials.

**Sign out** deletes the saved credential for the selected provider and server. Other projects using
that account will require sign-in after their next refresh. The provider's authorization grant
remains active until revoked through its account settings. Token fields clear after submission,
failure, or cancellation.

## Verification

Local fixtures verify protocol requests without contacting providers. Tests cover callback
validation, token refresh, invalid tokens, cancellation, response limits, and redirect rejection.
Job fixtures also verify pagination, stage order, action endpoints, permission errors, and
duplicate-submission guards. An in-memory credential store keeps test tokens outside the local
secrets directory. Live browser sign-in requires registered Client IDs and user authorization. No
live provider sign-in is implied by passing fixture tests.
