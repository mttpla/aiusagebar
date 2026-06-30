# Changelog

## [v1.1.0] - 2026-06-30

### Bug Fixes

- Tolerate disabled spend on Pro/Max usage body

### Documentation

- Kanban folder restructure — status subfolders + date-prefixed filenames
- Add plan, close card 57 to done
- Close raw-json-details-window card #45 to done
- Claude enterprise usage parse — dual-shape support
- Add card #58 Claude enterprise usage parse
- Pin anonymized test fixtures for enterprise usage parse
- Move to doing, add implementation plan
- Close to done — enterprise usage parse implemented
- Document Claude Pro/Max vs Enterprise usage in README; close card

### Features

- Add money fields to LimitWindow and money-aware detail line
- Parse enterprise spend window via optional usage structs
- Derive plan label from organization_type with flag fallback

### Refactoring

- Migrate cards into status subfolders with date-prefixed filenames

### Tests

- Smoke-guard that every real usage body parses

## [v1.0.0] - 2026-06-22

### Documentation

- Update spec/card, add implementation plan
- Add spec and kanban card
- Add spec and kanban card
- Add spec and kanban card
- Add MIT LICENSE and license metadata
- Badges, Configuration, Keychain trim, Requirements/Development split, Releasing rewrite, License section
- Record Tasks 2,3,6,7,8 done; screenshots remain
- Add Gatekeeper and Keychain first-run screenshots
- Finish README revamp — Install (Gatekeeper), Keychain depth, Troubleshooting blocked-dialog
- Close card — all tasks done
- Close card — manual acceptance passed

### Miscellaneous

- Gitignore .claude/ (local settings + worktrees)

### Refactoring

- Extract 24h update-check interval to settings const

## [v0.5.0] - 2026-06-22

### Bug Fixes

- Accept null resets_at from API
- Remove unused setup menu fields pending main.rs implementation
- Remove unused raw_json trait method
- Remove show() function with expect(dead_code) suppression
- Remove premature details click handler
- Use GetResult alias in get_returns_tuple test annotation
- On_success advances next_allowed_at to prevent immediate re-poll
- Advance backoff timer on all non-429/5xx outcomes
- Address review — testable diag core, format_args macro, pasteboard error log
- One global timer, tick fetches all providers

### Documentation

- Close cards #15 #18 — superseded by ureq migration
- Defer #12 post-1.0, note backend already done
- Spec, plan, and card for resets_at null fix (#43)
- Spec and card for diagnostic log service
- Close card #13 — onboarding empty state done
- Add spec + plan for exponential backoff
- Close card #19 — backoff done
- Spec + card #45 — raw JSON details window
- Reclassify #16 post-1.0, tag #44 pre-1.0
- Reclassify #39 post-1.0, clarify scope
- #37 post-1.0, note release section audit in #34
- Update #44 spec, add #46 instrumentation sweep card
- Add #47 fix BackoffState::on_success not advancing next_allowed_at
- Add details-submenu card (#48) and raw-json plan
- Add diagnostic-log-service plan (#44), start card, tag diag-sweep pre-1.0
- Add Troubleshooting section for diagnostic log
- Plan + scope update for diag instrumentation sweep
- Spec + plan + card for Details-in-Other submenu (#48)
- Spec + plan + cards for provider error boundary diag (#50, #51)
- Spec + plan + card for non-provider diag gaps (#51)

### Features

- Clickable setup row for Claude NotConfigured state
- Clickable setup row for Copilot NotConfigured state
- Expose setup MenuIds in MenuBuild
- Expose setup MenuIds in MenuBuild
- Wire setup-page click handler + add provider setup docs
- Add ServerError(u16) variant for 5xx responses
- Add BackoffState with configurable factor and cap
- Per-provider exponential backoff on 429/5xx
- Strip + LTO release profile (#17)
- Http::get returns raw body alongside result
- Add raw_json() to UsageProvider trait with stub impls
- Cache last raw HTTP body in last_raw_json field
- Cache raw HTTP body per-account in last_raw_json
- Add details module with raw JSON window
- Add Details menu item to each provider section
- Wire Details menu items to show raw HTTP response window
- Add diagnostic log service with Other menu and error hooks
- Log Keychain read/enumerate failures, skip not-found
- Log network/status/body errors in http get_public
- Log Claude HTTP-error arms and profile parse failures
- Log Copilot per-account HTTP and parse errors
- Add other_entries decision for Other submenu contents
- Render provider Details inside the Other submenu
- Remove flat Details from provider sections
- Add state_diag_message boundary helper
- Log provider Error/Stale states at refresh_all boundary
- Log credential-file and Keychain enumerate read failures
- Log update-check parse, tray icon, and open-command failures

### Miscellaneous

- Mark diagnostic-log-service card (#44) done
- Tighten item visibility to pub(crate)
- Mark diag-instrumentation-sweep card done
- Mark details-submenu card #48 done

### Refactoring

- Replace fetch with fetch_with_http_error
- BackoffState owns base/factor/cap config

## [v0.4.0] - 2026-06-16

### Bug Fixes

- Use v-prefix in cliff.toml heading, backfill CHANGELOG
- Add User-Agent header to get_public
- Then_some and is_some_and

### Documentation

- Close cards #40 #41, link plan to #36
- Close card #36 release.sh hardening
- Drop OpenAI/Codex, unify provider names, add screenshot placeholder
- Re-scope #11 to binary release, create #42 DMG card, update #37 narrative
- Add demo screenshot for README
- Close card #10, update #34 narrative
- Update demo screenshot, close kanban card #11
- Add spec and plan for auto-update check
- Close card #20 auto-update check

### Features

- Add pre-flight checks (branch, clean tree, sync, tag-exists)
- Add ERR trap — rollback Cargo.toml + CHANGELOG on failure
- Annotated tag, interactive push prompt
- Add clippy + test quality gate before version bump
- Build, sign, and publish arm64 binary to GitHub Releases
- Add get_public for unauthenticated requests
- Add is_newer version comparison
- Add parse_release JSON extraction
- Implement check() via GitHub Releases API
- Prepend update row to tray menu when newer release available
- Wire up 24h update check with wall-clock timer

### Miscellaneous

- Add dist/ to .gitignore and add Installation section to README

## [v0.3.2] - 2026-06-16

### Bug Fixes

- Sync Cargo.lock in release.sh, commit alongside Cargo.toml

## [v0.3.1] - 2026-06-16

### Bug Fixes

- Explicitly set TlsProvider::NativeTls — ureq 3 defaults to Rustls
- Restore Refresh click — drop setView, remove tab stop
- Compare CFString directly instead of allocating to_string

### Documentation

- Add spec and kanban card #40 for ureq migration
- Add implementation plan for ureq migration (#40)
- Link plan to kanban card #40

## [v0.3.0] - 2026-06-15

### Bug Fixes

- Per-account error rows and clean header for Copilot multi-account
- Strip v-prefix from git describe so about shows clean version
- Pin menu width to 290px, derive bar dimensions from constants

### Documentation

- Card #36 release.sh hardening + split GH Action out of #11
- Card #37 GH Action on tag + cliff.toml v-prefix fix into #36
- Close card #31 and commit plan file
- Add cards #38 and #39 with spec and plan

### Features

- Render reset time in local OS timezone

### Miscellaneous

- Archive #14 — premise superseded by live Copilot provider
- Split #11 into ad-hoc DMG (pre-1.0) + #35 notarized DMG (post-1.0)

### Refactoring

- Hoist Local::now() out of window loop
- Centralize reset-time TZ conversion in ui/time.rs

## [v0.2.0] - 2026-06-13

### Bug Fixes

- Is_expired uses <= to correctly handle exact-expiry-ms tokens
- Use provider name() in build_menu, eliminate dead_code warning
- Replace all Italian string literals with English
- Remove left-click refresh that blocked event loop and prevented menu from opening
- Use correct field name 'utilization' from actual API response (API.md was wrong)
- Xml-escape binary path in plist_content, strengthen test
- Idempotent disable, cleanup plist on bootstrap failure, improve uid errors
- Correct launchctl bootout arg form, always cleanup plist on disable
- Launchctl bootstrap best-effort, plist persists on failure
- Use English string literals in copilot provider
- Add User-Agent header, drop env-var token sources
- Hide Dock icon via winit activation policy API
- Wire alert threshold through settings; prevent double-refresh on same tick
- Show timestamp on first refresh
- Check git-cliff on PATH before mutating anything
- Remove folder icon, force wider dialog via NSView spacer
- Remove unnecessary unsafe block around setFrame
- Center body text via NSTextField accessory view
- Button label www.matteopaoli.it
- Version in title, remove from body
- Update UsageState::Ok patterns for new profile field
- Update Ok pattern match for profile field
- Template icon for dark mode, strengthen PNG validation test
- Restore original menu order, consistent copilot pattern, pub(crate) visibility
- Remove redundant nested unsafe blocks in styled.rs attribute setters
- Use secondary color for pct label when usage unknown
- Resolve Clippy warnings in make_progress_row_view
- Dispatch on ProviderKind enum, restore Copilot section
- Restore footer order — Refresh, About, Quit per spec
- Style About item at 13pt labelColor to match Refresh/Quit
- Format Copilot reset_at as short date + align pct column to 290pt

### Documentation

- Add REQUIREMENTS and API specs
- Correct Claude API response schema (utilization not used_percentage)
- Mark Plan 1 complete, update architecture to reflect current state
- Document dev codesign setup, replace cargo run with make dev
- Add development setup section with codesign instructions to README
- Mark dev-codesign plan complete
- Add launch-at-login design spec
- Add launch-at-login implementation plan
- Add dynamic tray icon design spec
- Add dynamic tray icon implementation plan
- Add FA attribution and tray icon legend to README
- Add code review fixes implementation plan
- Add test coverage implementation plan
- Add Copilot provider design spec
- Add Copilot provider implementation plan
- Clarify CFString cast assumption in keychain enumeration
- Add polling mechanism & settings struct spec
- Add versioning design spec (vergen + git-cliff + release script)
- Add About window design spec
- Add Claude account identity display spec
- Add polling mechanism & settings implementation plan
- Document release workflow in README
- Spec for about icon generation via build.rs + ab_glyph
- Commit account identity plan, aesthetics spec and kanban card
- Close card #7 — ui module restructure done
- Preserve uncommitted kanban cards and specs from ui-aesthetics worktree
- Require spec-split check before kanban card creation
- Close card #8 — ui styled text done
- Close card #5 — about icon with version number done
- Add implementation plan for card #5 about-icon
- Add 22 backlog kanban cards + 2 reset-tz specs
- Close card #9 — UI progress bar rows done
- Add card #34 — README revamp for 1.0.0
- Tag cards #31, #32 as pre-1.0 release blockers

### Features

- Add UsageProvider trait and core types
- Generic HTTP GET helper and Keychain reader
- Claude credential loader with expiry check
- ClaudeProvider fetch with 401/429 handling
- Show Claude usage windows in tray menu
- Add launch_at_login module with plist_content
- Implement launch_at_login enable/disable
- Call launch_at_login::enable at startup
- Add tray icon assets (FA brain, CC BY 4.0)
- Add icon_for_state() with hardcoded 80% threshold
- Embed icons via include_bytes!(), switch icon on refresh
- Add Keychain enumeration for multi-account Copilot discovery
- Add CopilotProvider parser and fetch logic
- Add CopilotProvider struct with Keychain token loading
- Wire CopilotProvider, generalise App to multi-provider
- Add Settings struct with poll interval and alert threshold defaults
- Add last-refresh timestamp slot to build_menu
- Record and display last-refresh timestamp in tray menu
- Implement automatic polling via WaitUntil with Settings-driven interval
- Add app_version() with vergen git-describe embedding
- Add pure logic + unit tests
- Implement NSAlert show() via objc2-app-kit
- Wire About menu item and event handler
- Add profile serde types and parse_profile_response
- Wire profile lazy-fetch into ClaudeProvider
- Show account identity in section header
- Generate about icon PNG at compile time via build.rs + ab_glyph
- Display compiled about icon in NSAlert via NSImage
- Scaffold src/ui/ module structure
- Implement header_label, pct_label, append_claude_section
- Implement row_label, append_copilot_section
- Implement refresh_label, append_footer
- Implement build_menu with provider routing
- Add MenuLayout, ProviderKind, build_layout to mod.rs
- Add styled.rs — NSAttributedString helpers for menu styling
- Wire MenuLayout + style_menu into build_menu
- NSAttributedString styled menu — brand colors, Refresh tab stop, Quit red
- Add NSBox objc2-app-kit feature for progress bars
- Extend MenuLayout with window_items for progress bar wiring
- Add bar_fill_color, bar_fill_width, format_reset helpers
- Custom NSView progress bar rows in menu

### Miscellaneous

- Ignore __pycache__ directories
- Add cliff.toml for conventional commits changelog
- Add release.sh for bump + tag + changelog
- Trim objc2-app-kit to NSAlert feature only
- Add .superpowers to gitignore, add kanban workflow rule
- Add Courier Prime Bold font (OFL) for about icon generation

### Refactoring

- Migrate event loop to ApplicationHandler API
- Extract do_fetch for testability, add 8 fetch branch tests
- Extract for_providers, test multi-provider fold
- Move ALERT_THRESHOLD to settings::DEFAULT_ALERT_THRESHOLD_PCT
- Extract START_YEAR constant
- Extract menu building into src/ui/
- Derive Default on LimitWindow
- Append_*_section returns item count for index tracking
- Remove separate Updated label row, footer always 2 items
- Drop unused MenuLayout.about_idx field

### Tests

- Remove tautological derive-testing tests
- Add multi-window icon threshold coverage
- Tighten cache assertion in do_fetch success test
- Add 3 missing copilot tests, clamp percent_used to [0,100]
- Remove ignored copilot smoke test
- Add multi-provider window_items index test
- Make 7d format_reset test timezone-safe


