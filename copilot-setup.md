# GitHub Copilot Setup

AIUsageBar reads your Copilot token to display live seat / premium-request usage.

## Token priority

AIUsageBar checks these sources in order and uses the first one found:

1. `COPILOT_GITHUB_TOKEN` environment variable (fine-grained PAT — recommended)
2. `GH_TOKEN` environment variable
3. `GITHUB_TOKEN` environment variable
4. macOS Keychain item `copilot-cli`
5. `~/.copilot/config.json`
6. `~/.config/gh/hosts.yml` (set by `gh auth login`)

## Recommended: fine-grained PAT

1. Go to **GitHub → Settings → Developer settings → Personal access tokens →
   Fine-grained tokens**.
2. Create a token with **read-only** access to your account's Copilot usage.
3. Export it in your shell profile:
   ```sh
   export COPILOT_GITHUB_TOKEN="github_pat_..."
   ```
4. Restart AIUsageBar (or click ↺ Refresh).

## Troubleshooting

- **Still shows "not signed in":** run `gh auth status` to confirm `gh` has a valid
  token, then restart AIUsageBar.
- **Usage data is empty:** your Copilot plan may not expose usage metrics via the API.
  Business and Enterprise plans have full coverage; Individual plans may have limited
  data.
