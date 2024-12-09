# zjswitcher

A zellij plugin that automatically switches between normal mode and locked mode based on configuration.

## Why?

I configured Zellij with vim-like key bindings, using normal mode as insert mode and locked mode like qutebrowser's pass-through mode. I use ESC to exit insert mode and Shift-ESC for pass-through mode. However, I often forget to switch to locked mode in programs that use ESC. This plugin automatically switches between normal and locked modes when a program starts and remembers the input mode for each pane.

## Features

- Automatically switches between normal and locked modes when a program starts.
- Remembers and maintains normal or locked modes for each pane. Switches to the saved mode when the focus changes if the current input mode is normal or locked.

## Installation

1. Download `zjswitcher.wasm` from the latest [release](https://github.com/WingsZeng/zjswitcher/releases).
2. Put it in the plugins directory of your Zellij.

## Configuration

- `programs_in_locked_mode`: A comma-separated list of programs that should run in locked mode at startup.

This plugin should load in background. Add this plugin in `load_plugins` section of your `config.kdl` file. Here's an example configuration:

```kdl
load_plugins {
    "file:target/wasm32-wasip1/debug/zjswitcher.wasm" {
        programs_in_locked_mode "vim, nvim"
    }
}
```

To enable the plugin to switch modes automatically when a program starts, you need to send a message to the plugin with the name of the program. This can be done, for example, by setting up `preexec` and `precmd` hooks in zsh configuration.

```sh
autoload -Uz add-zsh-hook
preexec_pipe_message_to_zjswitcher() {
    [[ -n "$ZELLIJ" ]] && zellij pipe --name Event::CommandUpdate -- $1
}
add-zsh-hook preexec preexec_pipe_message_to_zjswitcher

precmd_pipe_message_to_zjswitcher() {
    [[ -n "$ZELLIJ" ]] && zellij pipe --name Event::CommandUpdate -- $SHELL
}
add-zsh-hook precmd precmd_pipe_message_to_zjswitcher
```
