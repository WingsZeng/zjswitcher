use once_cell::sync::Lazy;
use std::collections::{BTreeMap, HashMap, HashSet};
use zellij_tile::prelude::*;

static SHELL: Lazy<String> = Lazy::new(|| std::env::var("SHELL").unwrap_or_default());

#[derive(Default)]
struct State {
    got_permission: bool,
    programs_in_locked_mode: HashSet<String>,
    focused_pane_id: Option<PaneId>,
    active_tab_pos: usize,
    pane_mode_map: HashMap<PaneId, InputMode>,
    input_mode: InputMode,
    last_pane_event: Option<PaneManifest>,
    log: bool,
}

register_plugin!(State);

impl ZellijPlugin for State {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
        ]);

        subscribe(&[
            EventType::PermissionRequestResult,
            EventType::ModeUpdate,
            EventType::TabUpdate,
            EventType::PaneUpdate,
            EventType::PaneClosed,
        ]);

        self.programs_in_locked_mode = configuration
            .get("programs_in_locked_mode")
            .map(|s| s.split(',').flat_map(|s| s.trim().parse().ok()).collect())
            .unwrap_or_default();

        self.log = configuration
            .get("log")
            .map(|s| s == "true")
            .unwrap_or(false);
    }

    fn update(&mut self, event: Event) -> bool {
        let mut should_render = false;
        match event {
            Event::PermissionRequestResult(PermissionStatus::Granted) => {
                self.got_permission = true;
                set_selectable(false);
                should_render = true;
            }
            Event::ModeUpdate(mode_manifest) => {
                let input_mode = mode_manifest.mode;
                if let Some(pane_id) = self.focused_pane_id {
                    match (input_mode, self.is_in_normal_or_locked_mode()) {
                        (InputMode::Normal, true) | (InputMode::Locked, _) => {
                            if self.log {
                                eprintln!(
                                    "ModeUpdate: Switching to {:?} and setting pane {:?} at tab {:?} to it",
                                    input_mode, pane_id, self.active_tab_pos,
                                );
                            }
                            self.pane_mode_map.insert(pane_id, input_mode);
                        }
                        (InputMode::Normal, false) => {
                            if self.log {
                                eprintln!(
                                    "ModeUpdate: Switching to {:?} and switching pane {:?} at tab {:?} to saved mode {:?}",
                                    input_mode, pane_id, self.active_tab_pos, self.pane_mode_map.get(&pane_id),
                                );
                            }
                            switch_to_input_mode(self.pane_mode_map.get(&pane_id).unwrap());
                        }
                        _ => {}
                    }
                };
                self.input_mode = input_mode;
            }
            Event::TabUpdate(tabs) => {
                let active_tab_pos = tabs
                    .iter()
                    .find(|tab| tab.active)
                    .map(|tab| tab.position)
                    .unwrap();
                if self.log {
                    eprintln!(
                        "TabUpdate: Active tab position changed from {:?} to {:?}",
                        self.active_tab_pos, active_tab_pos
                    );
                }
                if active_tab_pos != self.active_tab_pos {
                    self.active_tab_pos = active_tab_pos;
                    if let Some(last_pane_event) = self.last_pane_event.clone() {
                        if self.log {
                            eprintln!(
                                "TabUpdate: Handling pane update with last pane event {:?}",
                                last_pane_event
                            );
                        }
                        self.handle_pane_update(&last_pane_event);
                    }
                }
            }
            Event::PaneUpdate(manifest) => {
                if self.log {
                    eprintln!(
                        "PaneUpdate: Handling pane update with manifest {:?}",
                        manifest
                    );
                }
                self.handle_pane_update(&manifest);
                if self.log {
                    eprintln!(
                        "PaneUpdate: Setting last pane event to manifest {:?}",
                        manifest
                    );
                }
                self.last_pane_event = Some(manifest);
            }
            Event::PaneClosed(PaneId::Terminal(id)) => {
                if self.log {
                    eprintln!("PaneClosed: Removing pane {:?} from pane mode map", id);
                }
                self.pane_mode_map.remove(&PaneId::Terminal(id));
            }
            _ => {}
        };
        should_render
    }

    fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
        if pipe_message.name == "Event::CommandUpdate" {
            if let Some(cmdline) = pipe_message.payload {
                if self.log {
                    eprintln!("Pipe: Got command update with payload {:?}", cmdline);
                }
                if let Some(program) = parse_program_from_cmdline(&cmdline) {
                    if self.log {
                        eprintln!(
                            "Pipe: Parsed program {:?} from cmdline {:?}; Switching to input mode {:?}",
                            program, cmdline,
                            self.get_default_input_mode_by_program(program),
                        );
                    }
                    switch_to_input_mode(&self.get_default_input_mode_by_program(program));
                }
            };
        }
        false
    }
}

impl State {
    fn get_default_input_mode_by_program(&self, program: &str) -> InputMode {
        match self.programs_in_locked_mode.contains(program) {
            true => InputMode::Locked,
            false => InputMode::Normal,
        }
    }
    fn handle_pane_update(&mut self, manifest: &PaneManifest) {
        if self.log {
            eprintln!(
                "handle_pane_update: Handling pane update with manifest {:?}",
                manifest
            );
        }
        let focused_pane_id = manifest
            .panes
            .get(&self.active_tab_pos)
            .unwrap()
            .iter()
            .find(|pane| pane.is_focused && !pane.is_plugin)
            .map(|focused_pane| {
                let pane_id = PaneId::Terminal(focused_pane.id);
                #[allow(clippy::map_entry)]
                if !self.pane_mode_map.contains_key(&pane_id) {
                    let cmdline = focused_pane
                        .terminal_command
                        .clone()
                        .unwrap_or_else(|| SHELL.to_string());
                    let program = parse_program_from_cmdline(&cmdline).unwrap_or_default();
                    let default_input_mode = self.get_default_input_mode_by_program(program);
                    self.pane_mode_map.insert(pane_id, default_input_mode);
                    if self.log {
                        eprintln!(
                            "handle_pane_update: Inserted new pane {:?} with default input mode {:?} to pane mode map",
                            pane_id, default_input_mode,
                        );
                    }
                }
                pane_id
            });
        if focused_pane_id != self.focused_pane_id {
            if let Some(pane_id) = focused_pane_id {
                if let Some(input_mode) = self.pane_mode_map.get(&pane_id) {
                    if self.log {
                        eprintln!(
                            "handle_pane_update: focused_pane_id changed to {:?} with input mode {:?}",
                            pane_id, input_mode,
                        );
                    }
                    if self.is_in_normal_or_locked_mode() {
                        if self.log {
                            eprintln!(
                                "handle_pane_update: Switching to input mode {:?}",
                                input_mode,
                            );
                        }
                        switch_to_input_mode(input_mode);
                    }
                };
            }
            self.focused_pane_id = focused_pane_id;
        }
    }
    fn is_in_normal_or_locked_mode(&self) -> bool {
        self.input_mode == InputMode::Normal || self.input_mode == InputMode::Locked
    }
}

fn parse_program_from_cmdline(cmdline: &str) -> Option<&str> {
    let argv: Vec<&str> = cmdline.split_whitespace().collect();
    argv.first()
        .and_then(|s| s.split('/').last())
        .and_then(|s| match s {
            "sudo" | "doas" => argv.get(1).map(|s| s.to_owned()),
            _ => Some(s),
        })
}
