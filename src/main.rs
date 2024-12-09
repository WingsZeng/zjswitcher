use once_cell::sync::Lazy;
use std::collections::{BTreeMap, HashMap, HashSet};
use zellij_tile::prelude::*;

static SHELL: Lazy<String> = Lazy::new(|| std::env::var("SHELL").unwrap_or_default());

fn is_normal_or_locked_mode(mode: &InputMode) -> bool {
    mode == &InputMode::Normal || mode == &InputMode::Locked
}

#[derive(Default)]
struct State {
    got_permission: bool,
    programs_in_locked_mode: HashSet<String>,
    focused_pane_id: Option<PaneId>,
    active_tab_pos: usize,
    pane_mode_map: HashMap<PaneId, InputMode>,
    input_mode: InputMode,
    last_pane_event: Option<PaneManifest>,
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
                    match (input_mode, is_normal_or_locked_mode(&self.input_mode)) {
                        (InputMode::Normal, true) | (InputMode::Locked, _) => {
                            self.pane_mode_map.insert(pane_id, input_mode);
                        }
                        (InputMode::Normal, false) => {
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
                if active_tab_pos != self.active_tab_pos {
                    self.active_tab_pos = active_tab_pos;
                    if let Some(last_pane_event) = self.last_pane_event.clone() {
                        self.handle_pane_update(&last_pane_event);
                    }
                }
            }
            Event::PaneUpdate(manifest) => {
                self.handle_pane_update(&manifest);
                self.last_pane_event = Some(manifest);
            }
            Event::PaneClosed(PaneId::Terminal(id)) => {
                self.pane_mode_map.remove(&PaneId::Terminal(id));
            }
            _ => {}
        };
        should_render
    }

    fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
        if pipe_message.name == "Event::CommandUpdate" {
            if let Some(cmdline) = pipe_message.payload {
                if let Some(program) = parse_program_from_cmdline(&cmdline) {
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
                }
                pane_id
            });
        if focused_pane_id != self.focused_pane_id {
            if let Some(pane_id) = focused_pane_id {
                if let Some(input_mode) = self.pane_mode_map.get(&pane_id) {
                    if is_normal_or_locked_mode(&self.input_mode) {
                        switch_to_input_mode(input_mode);
                    }
                };
            }
            self.focused_pane_id = focused_pane_id;
        }
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
