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

use crossterm::event::{Event, EventStream as TermEventStream, KeyEventKind};
use futures::StreamExt;
use ratatui::{DefaultTerminal, buffer::Buffer, layout::Rect};
use tokio::select;

use self::{
    controls::{apply_command, command_for},
    model::InputCollection,
    plan::{Counts, RenderPlan, build_render_plan},
    render::frame::render_frame,
    state::{MonitorState, build_device_info_lines},
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
                            if let Some(exit) = apply_command(
                                command_for(key, monitor.state.active_popup()),
                                &mut monitor.state,
                                &mut monitor.inputs,
                                &plan,
                            ) {
                                return Ok(exit);
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
