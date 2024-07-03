# zjswitcher

A zellij plugin that automatically switches between normal mode and locked mode based on configuraion. 

## Why?

I configured Zellij with vim-like key bindings, treating normal mode as insert mode and locked mode similar to qutebrowser's pass-through mode. I use ESC to exit insert mode and Shift-ESC to exit pass-through mode. However, I often forget to switch to locked mode in programs that use ESC. To address this, I developed a plugin that automatically switches between normal mode and locked mode based on the active program in the focused pane.

## How does it work?

There is no `pane_current_command` or anything similar in `PaneInfo`, and the changing of the running command doesn't trigger a `PaneUpdate` event in the plugin API yet. The only way I found to get the running command of the focused pane is by using the `dump_session_layout` command or the `ListClients` action. It uses a hacky approach to update the running command of the focused pane by dumping the session layout in the `InputReceived` event, and then tries to switch modes when the command changes or the input mode changes.

## Installation and Setup

1. Download `zjswitcher.wasm` from the latest release.
2. Put it in the plugins directory of your Zellij.
3. Run `zellij plugin [OPTIONS] [--] file:path/to/your/zjswitcher.wasm`
4. Accept the request for permissions.
5. Close and configure your layout as described below.

## Configuration

- `programs_in_locked_mode`: A comma-separated list of programs that should run in locked mode.
- `hide`: hide the plugin after loading.

Note that the plugin loads only once but may run multiple times in different panes without hiding itself. Therefore, it's necessary to configure it to start only once and remain active. Here's an example configuration:

```kdl
layout {
    default_tab_template {
        pane size=1 borderless=true {
            plugin location="zellij:tab-bar"
        }
        children
        pane size=2 borderless=true {
            plugin location="zellij:status-bar"
        }
    }
    tab {
        pane
        pane borderless=true {
            plugin location="file:path/to/your/zjswitcher.wasm" {
                programs_in_locked_mode "nvim, yazi, sshs, lazygit, lazydocker, bluetuith"
                hide true
            }
        }
    }
}
```

The plugin works properly as long as the first tab remains open.

## Bugs and Limitations

- Sometimes switching fails because detecting command changes is not instantaneous.
- It can't handle panes configured with edit because `dump_session_layout` can't capture the command of the editor.
- It can't run in the background and must reside within a pane. So it hides itself after loading, causing some flickering during the process.
- Hiding self after loading hides the request for permissions that appear initially, necessitating more complex setup.
- It's not possible to ensure only one plugin runs; for example, placing the plugin in `default_tab_template` and opening multiple tabs results in multiple instances running simultaneously, and it doesn't hide in more than one tab because it `load`ed only once :(. Therefore, configuring the plugin to run only once and keeping it alive is necessary.
