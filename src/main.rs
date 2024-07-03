use kdl::{KdlDocument, KdlValue};
use std::collections::{BTreeMap, HashSet};
use zellij_tile::prelude::*;

#[derive(Default)]
struct State {
    programs_in_locked_mode: HashSet<String>,
    input_mode: InputMode,
    focused_pane_command: Option<String>,
}

register_plugin!(State);

impl ZellijPlugin for State {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
        ]);
        subscribe(&[
            EventType::InputReceived,
            EventType::ModeUpdate,
            EventType::CustomMessage,
        ]);
        self.programs_in_locked_mode = configuration
            .get("programs_in_locked_mode")
            .map(|s| s.split(',').flat_map(|s| s.trim().parse().ok()).collect())
            .unwrap_or_default();
        if configuration.get("hide").map_or(false, |s| s == "true") {
            hide_self();
        }
        #[cfg(debug_assertions)]
        eprintln!(
            "programs_in_locked_mode = {:?}",
            self.programs_in_locked_mode
        );
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::InputReceived => {
                #[cfg(debug_assertions)]
                eprintln!("InputReceived");
                dump_session_layout();
            }
            Event::ModeUpdate(mode_info) => {
                self.input_mode = mode_info.mode;
                self.try_switch();
            }
            Event::CustomMessage(message, payload) => {
                if message.as_str() == "session_layout" {
                    let doc = payload
                        .parse::<KdlDocument>()
                        .expect("failed to parse layout");

                    let command = doc
                        .query("layout > tab[focus=true]")
                        .unwrap_or(None)
                        .and_then(|focused_tab| {
                            focused_tab
                                .query_get("pane[focus=true]", "command")
                                .ok()
                                .flatten()
                        })
                        .and_then(|value| match value {
                            KdlValue::String(command) => Some(command.to_string()),
                            _ => None,
                        });

                    if self.focused_pane_command != command {
                        self.focused_pane_command = command;
                        self.try_switch();
                    }
                }
            }
            _ => {}
        };
        false
    }
}

impl State {
    fn try_switch(&mut self) {
        let is_program_in_locked_mode = self
            .focused_pane_command
            .as_ref()
            .map(|command| self.programs_in_locked_mode.contains(command))
            .unwrap_or(false);

        #[cfg(debug_assertions)]
        eprintln!(
            "try_switch: input_mode = {:?}, focused_pane_command = {:?}, is_program_in_locked_mode = {}",
            self.input_mode, self.focused_pane_command, is_program_in_locked_mode,
        );

        match (self.input_mode, is_program_in_locked_mode) {
            (InputMode::Normal, true) => {
                switch_to_input_mode(&InputMode::Locked);
                #[cfg(debug_assertions)]
                eprintln!("switched to locked mode");
            }
            (InputMode::Locked, false) => {
                switch_to_input_mode(&InputMode::Normal);
                #[cfg(debug_assertions)]
                eprintln!("switched to normal mode");
            }
            _ => {}
        };
    }
}
