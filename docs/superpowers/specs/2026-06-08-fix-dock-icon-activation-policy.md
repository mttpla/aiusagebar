# Fix: Remove Dock icon using winit activation policy

**Date:** 2026-06-08

## Problem

`aiusagebar` appears in the Dock with a terminal/console icon when launched at login.
Root cause: `set_accessory_policy()` (raw objc2 call) runs before `EventLoop::new()`, but winit 0.30 re-initialises `NSApplication` internally during `EventLoop::new()`, resetting the activation policy to `Regular` (0 = show in Dock).

## Fix

Use `winit::platform::macos::EventLoopBuilderExtMacOS::with_activation_policy(ActivationPolicy::Accessory)` when constructing the event loop. winit applies the policy *during* `NSApplication` init, so it is never overwritten.

Remove `set_accessory_policy()` entirely (dead code after the fix).

## Scope

- `src/main.rs`: replace `EventLoop::new()` + `set_accessory_policy()` with `EventLoop::builder().with_activation_policy(ActivationPolicy::Accessory).build()`
- No other files affected.

## Non-goals

- App bundle / Info.plist changes (not needed)
- Dock icon customisation
