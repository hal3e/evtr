mod config;
mod controls;
mod layout;
mod math;
mod model;
mod plan;
mod render;
mod state;
mod theme;
mod touch;
mod ui;

use crossterm::event::{
    Event, EventStream as TermEventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
};
use futures::StreamExt;
use ratatui::{DefaultTerminal, buffer::Buffer, layout::Rect};
use tokio::select;

use self::{
    controls::Command,
    model::InputCollection,
    plan::{Counts, RenderPlan, build_render_plan},
    render::frame::render_frame,
    state::{ActivePopup, MonitorState, build_device_info_lines},
    touch::TouchState,
};
use crate::{
    device::selector::DeviceInfo,
    error::{Error, ErrorArea, Result},
};

pub(crate) enum MonitorExit {
    BackToSelector,
    ExitApp,
}

pub(super) struct ComponentBootstrap<T> {
    pub(super) value: T,
    pub(super) startup_warnings: Vec<String>,
}

impl<T> ComponentBootstrap<T> {
    fn new(value: T) -> Self {
        Self {
            value,
            startup_warnings: Vec::new(),
        }
    }
}

struct DeviceBootstrap {
    inputs: InputCollection,
    touch: TouchState,
    startup_warnings: Vec<String>,
}

impl DeviceBootstrap {
    fn from_device(device: &evdev::Device) -> Self {
        let inputs = InputCollection::from_device(device);
        let touch = TouchState::from_device(device);
        let mut startup_warnings = inputs.startup_warnings;
        startup_warnings.extend(touch.startup_warnings);

        Self {
            inputs: inputs.value,
            touch: touch.value,
            startup_warnings,
        }
    }
}

pub struct DeviceMonitor {
    device_stream: evdev::EventStream,
    inputs: InputCollection,
    identifier: String,
    touch: TouchState,
    state: MonitorState,
}

fn command_for(key_event: KeyEvent, popup: ActivePopup) -> Command {
    match popup {
        ActivePopup::Info => match key_event.code {
            KeyCode::Esc | KeyCode::Char('i') => Command::ToggleInfo,
            KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                Command::ExitApp
            }
            _ => Command::None,
        },
        ActivePopup::Help => match key_event.code {
            KeyCode::Esc | KeyCode::Char('?') => Command::ToggleHelp,
            KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                Command::ExitApp
            }
            _ => Command::None,
        },
        ActivePopup::None => match key_event.code {
            KeyCode::Esc => Command::BackToSelector,
            KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                Command::ExitApp
            }
            KeyCode::Char('r') => Command::Reset,
            KeyCode::Home | KeyCode::Char('g') => Command::Home,
            KeyCode::End | KeyCode::Char('G') => Command::End,
            KeyCode::Up | KeyCode::Char('k') => Command::Scroll(-1),
            KeyCode::Down | KeyCode::Char('j') => Command::Scroll(1),
            KeyCode::Char('i') => Command::ToggleInfo,
            KeyCode::Char('y') => Command::ToggleInvertY,
            KeyCode::Char('?') => Command::ToggleHelp,
            KeyCode::Char('J') => Command::FocusNext,
            KeyCode::Char('K') => Command::FocusPrev,
            KeyCode::PageUp => Command::Page(-1),
            KeyCode::PageDown => Command::Page(1),
            _ => Command::None,
        },
    }
}

impl DeviceMonitor {
    fn new(DeviceInfo { device, identifier }: DeviceInfo) -> Result<Self> {
        let bootstrap = DeviceBootstrap::from_device(&device);
        let counts = Counts::new(
            bootstrap.inputs.absolute_inputs().len(),
            bootstrap.inputs.relative_inputs().len(),
            bootstrap.inputs.button_inputs().len(),
        );
        let info_lines = build_device_info_lines(
            device.driver_version(),
            device.input_id(),
            device.physical_path(),
            &bootstrap.startup_warnings,
        );
        let device_stream = device.into_event_stream().map_err(|err| {
            Error::evdev(
                ErrorArea::Monitor,
                format!("open device stream ({identifier})"),
                err,
            )
        })?;

        Ok(Self {
            device_stream,
            inputs: bootstrap.inputs,
            identifier,
            touch: bootstrap.touch,
            state: MonitorState::new(counts, info_lines),
        })
    }

    pub async fn run(
        terminal: &mut DefaultTerminal,
        device_info: DeviceInfo,
    ) -> Result<MonitorExit> {
        let mut monitor = Self::new(device_info)?;
        let mut term_events = TermEventStream::new();

        loop {
            terminal
                .draw(|frame| monitor.render(frame.area(), frame.buffer_mut()))
                .map_err(|err| Error::io(ErrorArea::Monitor, "monitor draw", err))?;

            select! {
                event = term_events.next() => {
                    match event {
                        Some(Ok(Event::Key(key))) if key.kind == KeyEventKind::Press => {
                            let area = terminal
                                .size()
                                .map(Rect::from)
                                .map_err(|err| Error::io(ErrorArea::Monitor, "terminal size", err))?;
                            let plan = monitor.sync_render_plan(area);
                            match command_for(key, monitor.state.active_popup()) {
                                Command::BackToSelector => return Ok(MonitorExit::BackToSelector),
                                Command::ExitApp => return Ok(MonitorExit::ExitApp),
                                Command::Reset => monitor.inputs.reset_relative_axes(),
                                Command::Scroll(dir) => monitor.state.scroll_by(dir, &plan),
                                Command::Page(dir) => monitor
                                    .state
                                    .scroll_page(dir, &plan, config::PAGE_SCROLL_STEPS),
                                Command::Home => monitor.state.scroll_home(&plan),
                                Command::End => monitor.state.scroll_end(&plan),
                                Command::FocusNext => monitor.state.focus_next(&plan),
                                Command::FocusPrev => monitor.state.focus_prev(&plan),
                                Command::ToggleInfo => monitor.state.toggle_info(),
                                Command::ToggleHelp => monitor.state.toggle_help(),
                                Command::ToggleInvertY => monitor.state.toggle_invert_y(),
                                Command::None => {}
                            }
                        }
                        Some(Ok(_)) => {}
                        Some(Err(err)) => {
                            return Err(Error::io(
                                ErrorArea::Monitor,
                                "terminal event stream",
                                err,
                            ));
                        }
                        None => {
                            return Err(Error::stream_ended(
                                ErrorArea::Monitor,
                                "terminal event stream",
                            ));
                        }
                    }
                }
                event = monitor.device_stream.next_event() => {
                    let event = event.map_err(|err| {
                        Error::evdev(
                            ErrorArea::Monitor,
                            format!("device event stream ({})", monitor.identifier),
                            err,
                        )
                    })?;
                    monitor.inputs.handle_event(&event);
                    monitor.touch.update(&event);
                }
            }
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let plan = self.sync_render_plan(area);
        render_frame(
            area,
            buf,
            &self.identifier,
            &self.state,
            &self.inputs,
            &self.touch,
            &plan,
        );
    }

    fn sync_render_plan(&mut self, area: Rect) -> RenderPlan {
        let plan = build_render_plan(area, &self.state, &self.inputs, &self.touch);
        self.state.sync_from_plan(&plan);
        plan
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::{ActivePopup, command_for};
    use crate::device::monitor::controls::Command;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl_char(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    #[test]
    fn command_for_ctrl_c_exits_from_any_popup_state() {
        for popup in [ActivePopup::None, ActivePopup::Info, ActivePopup::Help] {
            assert_eq!(command_for(ctrl_char('c'), popup), Command::ExitApp);
        }
    }

    #[test]
    fn command_for_escape_backs_out_only_without_popup() {
        assert_eq!(
            command_for(key(KeyCode::Esc), ActivePopup::None),
            Command::BackToSelector
        );
        assert_eq!(
            command_for(key(KeyCode::Esc), ActivePopup::Info),
            Command::ToggleInfo
        );
        assert_eq!(
            command_for(key(KeyCode::Esc), ActivePopup::Help),
            Command::ToggleHelp
        );
    }
}
