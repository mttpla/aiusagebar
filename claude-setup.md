# Claude Setup

AIUsageBar reads your Claude session token to display live usage data in the menu bar.

## How to sign in

1. Open [claude.ai](https://claude.ai) in your browser and sign in to your account.
2. Open the Claude desktop app (macOS) and sign in there too.
   AIUsageBar reads the token that the Claude app stores in your macOS Keychain.
3. On first launch, macOS will show a dialog asking whether to allow AIUsageBar to
   read the Keychain item — click **Always Allow**.

## Troubleshooting

- **Still shows "not signed in" after signing in:** click ↺ Refresh in the menu bar.
- **Keychain dialog never appeared:** open Keychain Access, search for
  `Claude Code-credentials`, and grant access manually.
- **Usage shows "account unavailable":** your Claude plan may not expose usage data
  via the API. Max plan subscribers see full data.
