mod bootstrap;
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
mod view_model;

use crossterm::event::{Event, EventStream as TermEventStream, KeyEventKind};
use futures::StreamExt;
use ratatui::{DefaultTerminal, buffer::Buffer, layout::Rect};
use tokio::select;

use self::{
    bootstrap::MonitorBootstrap,
    controls::{apply_command, command_for},
    model::InputCollection,
    plan::{RenderPlan, build_render_plan},
    render::frame::{FrameData, render_frame},
    state::MonitorState,
    touch::TouchState,
    view_model::MonitorViewModel,
};
use crate::{
    error::{ErrorArea, Result},
    selector::DeviceInfo,
};

pub(crate) enum MonitorExit {
    BackToSelector,
    ExitApp,
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
        let MonitorBootstrap {
            inputs,
            touch,
            state,
        } = MonitorBootstrap::from_device(&device);
        let device_stream = device.into_event_stream().map_err(|err| {
            ErrorArea::Monitor.evdev(format!("open device stream ({identifier})"), err)
        })?;

        Ok(Self {
            device_stream,
            inputs,
            identifier,
            touch,
            state,
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
                .map_err(|err| ErrorArea::Monitor.io("monitor draw", err))?;

            select! {
                event = term_events.next() => {
                    match event {
                        Some(Ok(Event::Key(key))) if key.kind == KeyEventKind::Press => {
                            let area = terminal
                                .size()
                                .map(Rect::from)
                                .map_err(|err| ErrorArea::Monitor.io("terminal size", err))?;
                            let view_model = monitor.build_view_model();
                            let plan = monitor.sync_render_plan(area, &view_model);
                            let navigation = plan.navigation_context();
                            if let Some(exit) = apply_command(
                                command_for(key, monitor.state.active_popup()),
                                &mut monitor.state,
                                &mut monitor.inputs,
                                navigation,
                            ) {
                                return Ok(exit);
                            }
                        }
                        Some(Ok(_)) => {}
                        Some(Err(err)) => {
                            return Err(ErrorArea::Monitor.io("terminal event stream", err));
                        }
                        None => {
                            return Err(ErrorArea::Monitor.stream_ended("terminal event stream"));
                        }
                    }
                }
                event = monitor.device_stream.next_event() => {
                    let event = event.map_err(|err| {
                        ErrorArea::Monitor
                            .evdev(format!("device event stream ({})", monitor.identifier), err)
                    })?;
                    monitor.inputs.handle_event(&event);
                    monitor.touch.update(&event);
                }
            }
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let view_model = self.build_view_model();
        let plan = self.sync_render_plan(area, &view_model);
        let frame_data = FrameData::new(
            &self.identifier,
            &self.state,
            &self.inputs,
            &self.touch,
            &view_model,
        );
        render_frame(area, buf, &frame_data, &plan);
    }

    fn build_view_model(&self) -> MonitorViewModel {
        MonitorViewModel::from_inputs(
            self.state.counts(),
            &self.inputs,
            &self.touch,
            self.state.joystick_invert_y(),
        )
    }

    fn sync_render_plan(&mut self, area: Rect, view_model: &MonitorViewModel) -> RenderPlan {
        let plan = build_render_plan(area, &self.state, view_model);
        self.state.sync_from_navigation(plan.navigation_context());
        plan
    }
}
