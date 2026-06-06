# AIUsageBar — Provider API Reference

Endpoint URLs, token sources, and response shapes for each provider.
All endpoints are unofficial and subject to change.

---

## OpenAI (Codex)

**Token:** read `~/.codex/auth.json`, field `accessToken`.

**Endpoint:**
```
GET https://chatgpt.com/backend-api/codex/usage
Authorization: Bearer <accessToken>
```

**Response (relevant fields):**
```json
{
  "primary_window":   { "utilization": 0.39, "resets_at": "..." },
  "secondary_window": { "utilization": 0.15, "resets_at": "..." }
}
```
`utilization` is a fraction 0–1; multiply by 100 for `percent_used`.
`limit` and `remaining` are not exposed.

**Error handling:**
- File not found → `NotConfigured`
- HTTP 401 → `Stale`
- HTTP 429 → `Stale` (rate limited)
- Network / parse error → `Error`

---

## Anthropic (Claude)

**Token (in order):**
1. Keychain generic-password: service `Claude Code-credentials`, account = current macOS username.
   JSON value: `{ "claudeAiOauth": { "accessToken": "sk-ant-oat01-...", "expiresAt": <epoch ms>, ... } }`
2. Fallback: `~/.claude/.credentials.json` (same schema).
3. Neither found → `NotConfigured`.

Pre-flight: if `expiresAt` is in the past → `Stale`, skip HTTP call entirely.
First Keychain read triggers a macOS permission dialog — expected, user clicks "Always Allow" once.

**Endpoint:**
```
GET https://api.anthropic.com/api/oauth/usage
Authorization: Bearer <accessToken>
User-Agent: <value matching the Claude Code CLI>
```
Minimum polling interval: **180 seconds**. Wrong/missing `User-Agent` = instant persistent HTTP 429.

**Response (relevant fields):**
```json
{
  "five_hour": { "utilization": 44.0, "resets_at": "2026-06-06T19:30:01.307521+00:00" },
  "seven_day":  { "utilization": 25.0, "resets_at": "2026-06-08T18:00:00.307588+00:00" },
  "seven_day_oauth_apps": null,
  "seven_day_opus": null,
  "seven_day_sonnet": null,
  "extra_usage": { "is_enabled": false, "monthly_limit": null, "used_credits": null, "utilization": null }
}
```

**Error handling:**
- HTTP 401 → `Stale` (do NOT refresh — token is single-use/rotating)
- HTTP 429 → return last cached state
- Network / parse error → `Error`

---

## GitHub (Copilot)

**Token (first hit wins):**
1. Env var `COPILOT_GITHUB_TOKEN` (fine-grained PAT, preferred)
2. Env var `GH_TOKEN`
3. Env var `GITHUB_TOKEN`
4. Keychain generic-password, service `copilot-cli`
5. `~/.copilot/config.json`
6. `~/.config/gh/hosts.yml`

None found → `NotConfigured`.

**Endpoint:**
```
GET https://api.github.com/copilot_internal/user
Authorization: Bearer <token>
```

**Response (relevant fields):**
```json
{
  "login": "username",
  "copilot_plan": "individual_pro",
  "quota_reset_date_utc": "2026-07-01T00:00:00.000Z",
  "quota_snapshots": {
    "premium_interactions": {
      "entitlement": 1500,
      "remaining": 1327,
      "percent_remaining": 88.5,
      "unlimited": false
    },
    "chat":        { "unlimited": true },
    "completions": { "unlimited": true }
  }
}
```

Build one `LimitWindow` per snapshot entry. `percent_used = 100.0 - percent_remaining`.
For `unlimited: true` entries: set `unlimited` flag, omit `percent_used` / `limit` / `remaining`.
Use `login` field for the per-account display label `GitHub (<login>)`.
Multiple accounts may be present — one menu section per account.

**Error handling:**
- HTTP 401 → `Stale`
- Network / parse error → `Error`
