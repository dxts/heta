# heta — source overview

## Application lifecycle

```
main()
  ├── errors::init()          panic/eyre hooks
  ├── logging::init()         tracing subscriber
  ├── App::new()              one-time setup
  │     ├── AwsState::init()         load default AWS config chain
  │     ├── Header::new()            seed with profile/region
  │     └── ProfilesList             register_action_handler → spawns async fetch
  │
  └── App::run()              enter terminal, start loop
        │
        ╭──────────── loop ────────────╮
        │                              │
        │  1. handle_events            │  poll terminal for key/tick/render/resize
        │       └─ handle_key_event    │  focus-aware routing (see below)
        │                              │
        │  2. handle_actions           │  drain action queue, three stages per action:
        │       ├─ Stage 1: App-level  │    quit, suspend, mode changes, view switch, AWS reload
        │       ├─ Stage 2: Chrome     │    header, command bar, breadcrumb always see every action
        │       └─ Stage 3: View       │    only the active view receives the action
        │                              │
        │  3. suspend / quit check     │
        │                              │
        ╰──────────────────────────────╯
```

## Key routing (handle_key_event)

Input focus flows through a priority chain. The first handler to return
`Some(Action)` claims the key — everything below it is skipped.

```
KeyEvent
  │
  ├─ 1. CommandBar (when active)     captures all input for : and / modes
  │
  ├─ 2. Active view                  j/k/Enter etc. — view-specific bindings
  │
  └─ 3. Global bindings              : → OpenCommandBar
                                      / → OpenFilterBar
                                      config keymaps (q → Quit, Ctrl-c, etc.)
```

## Frame layout (render)

```
┌─────────────────────────┐
│ Header (5 lines)        │  profile/region/account + context actions + logo + fps
├─────────────────────────┤
│ Command bar (0 or 1)    │  visible only in Command/Filter mode
├─────────────────────────┤
│ Resource area (fill)    │  delegates to active view (profiles, s3, lambda, etc.)
├─────────────────────────┤
│ Breadcrumb (1 line)     │  navigation trail: profile › view
└─────────────────────────┘
```

## Action flow

Components communicate exclusively through `Action`s on an unbounded mpsc channel.

- **Sync path:** key handler → `action_tx.send(action)` → drained in `handle_actions`
- **Async path:** component spawns a tokio task → task sends action when done
  (e.g. `ProfilesLoaded` after parsing `~/.aws/config`)
- **Chaining:** `component.update(action)` can return `Some(Action)` which gets re-queued

## Module map

| Module | Purpose |
|---|---|
| `main.rs` | Entry point — init, parse CLI, run app |
| `app.rs` | Event loop, key routing, action dispatch, frame rendering |
| `action.rs` | `Action` enum — every possible state transition |
| `tui.rs` | Terminal wrapper — raw mode, event stream, tick/render intervals |
| `config.rs` | Config loading, keybinding parsing, style parsing |
| `cli.rs` | CLI argument definitions (clap) |
| `resource_selector.rs` | `ResourceType` enum — maps `:command` names to views |
| `components.rs` | `Component` trait definition |
| `components/common/` | Always-visible chrome: header, command bar, breadcrumb, fps |
| `components/profiles.rs` | Profiles table view |
| `aws/state.rs` | `AwsState` — SDK config, clients, profile/region reload |
| `aws/profiles.rs` | Parse `~/.aws/config` → `ProfileInfo` list (memoized) |