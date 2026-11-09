# Windows installer and signing

The release workflow packages an existing Windows binary with Inno Setup 7.1.1. It verifies the
compiler download checksum and its publisher signature before installation. The `windows_package`
job runs only for version tags after all six application builds pass. The publication job requires
both installer jobs to succeed.

Installer text comes from the shared Fluent catalogs. Built-in Inno Setup translations cover
standard pages. The branded welcome panel uses the existing application icon. Users can choose their
language, installation folder, desktop shortcut, and whether to launch the application afterward.

## Unsigned installation

Leave `IWE_WINDOWS_SIGNING_ENABLED` unset or set it to `false`. The workflow builds unsigned
installers, portable ZIP files, and signed update archives. Update signatures verify package
integrity. They do not establish a trusted Windows publisher. Windows SmartScreen can display an
unknown-publisher warning for unsigned downloads.

## Prepare Microsoft Artifact Signing

This integration remains inactive until an owner configures an Azure account and a public-trust
certificate profile. Microsoft requires identity validation. The service can incur charges. No Azure
resources or certificates are created by these scripts.

1. Create an Artifact Signing account and a public-trust certificate profile in Azure.
2. Register an application identity for GitHub OIDC authentication.
3. Grant the identity the Artifact Signing Certificate Profile Signer role on that profile.
4. Add a federated identity with issuer `https://token.actions.githubusercontent.com`.
5. Set its audience to `api://AzureADTokenExchange`.
6. Set its subject to `repo:adametsderschopfer/iwe-code:environment:windows-signing`.
7. Create the GitHub environment `windows-signing` and restrict deployments to trusted release tags.
8. Configure the repository variables below. Enable signing only after configuring every required
   value.

| Repository variable           | Value                                    |
| ----------------------------- | ---------------------------------------- |
| `IWE_WINDOWS_SIGNING_ENABLED` | `true` to require signing                |
| `IWE_AZURE_CLIENT_ID`         | Registered application client ID         |
| `IWE_AZURE_TENANT_ID`         | Azure tenant ID                          |
| `IWE_AZURE_SUBSCRIPTION_ID`   | Azure subscription ID                    |
| `IWE_SIGNING_ENDPOINT`        | Regional HTTPS Artifact Signing endpoint |
| `IWE_SIGNING_ACCOUNT`         | Artifact Signing account name            |
| `IWE_SIGNING_PROFILE`         | Public-trust certificate profile name    |

GitHub OIDC avoids a stored Azure client secret. Only the packaging job requests an OIDC token. The
pinned signing client uses Azure CLI credentials from `azure/login`. The signing script requires a
SHA-256 signature and a trusted timestamp. Inno Setup invokes the same script for its installer and
uninstaller. Failed signing prevents packaging and publication. It never falls back to unsigned
output.

A valid signature identifies the publisher. SmartScreen also evaluates download reputation. Signing
does not guarantee that every Windows security warning disappears.

## Sources

- [Microsoft SmartScreen reputation](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/smartscreen-reputation)
- [Artifact Signing integrations](https://learn.microsoft.com/en-us/azure/artifact-signing/how-to-signing-integrations)
- [Inno Setup compiler downloads](https://jrsoftware.org/isdl.php)

## Local verification limits

Source checks can validate script syntax without producing an installer. Installer compilation and
native installation checks require Windows. Live signing requires the configured Azure identity and
certificate profile.
