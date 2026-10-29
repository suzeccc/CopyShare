# Code signing policy

Status: signing is not yet active. CopyShare's Windows installers are currently unsigned.

Free code signing provided by [SignPath.io](https://signpath.io/), certificate by [SignPath Foundation](https://signpath.org/), if the project is accepted. The verified Windows publisher will be SignPath Foundation.

Only the CopyShare Windows installer and application built from [this repository](https://github.com/suzeccc/CopyShare) by the [Release workflow](../.github/workflows/release.yml) may be submitted for signing. Releases are built from version tags on GitHub-hosted runners. Every signing request requires approval before the signed installer is published. Unsigned builds must not be described as signed.

The repository owner, [suzeccc](https://github.com/suzeccc), is the project author, reviewer of external contributions, and release approver. Access to GitHub and SignPath must use multifactor authentication.

CopyShare handles clipboard data, files, and optional translation requests. Its [privacy and security boundaries](../README_EN.md#privacy-and-security-boundaries) describe local storage, LAN transfer, optional third-party services, and update checks. Users can uninstall CopyShare through their operating system's application settings.
