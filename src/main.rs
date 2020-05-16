use std::env;
use std::f64;
use std::os::raw::c_void;
use std::path::PathBuf;
use std::ptr;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
use vst::api;
use vst::buffer::AudioBuffer;
use vst::editor::Rect;
use vst::host::{Dispatch, PluginLoader};
use vst::plugin::{OpCode, Plugin};
use winit::dpi::LogicalSize;
use winit::event::Event;
use winit::event_loop::{EventLoop, ControlFlow};
use winit::window::WindowBuilder;

struct Host;

impl vst::host::Host for Host {
    fn automate(&self, index: i32, value: f32) {
        println!("automate: index = {:?}; value = {:?}", index, value);
    }

    fn get_plugin_id(&self) -> i32 { todo!() }

    fn idle(&self) { todo!() }

    fn get_info(&self) -> (isize, String, String) {
        (1, "Mixlab".to_owned(), "Mixlab".to_owned())
    }

    fn process_events(&self, events: &api::Events) { todo!() }

    fn get_time_info(&self, mask: i32) -> Option<api::TimeInfo> { todo!() }

    fn get_block_size(&self) -> isize { todo!() }

    fn update_display(&self) { todo!() }
}

const SAMPLE_RATE: usize = 44100;
const BLOCK_SIZE: usize = SAMPLE_RATE / 100;

fn main() {
    let path = PathBuf::from(env::args().nth(1).expect("pass plugin path on cmdline"));

    let mut loader = PluginLoader::load(&path, Arc::new(Mutex::new(Host))).unwrap();

    let mut plugin = loader.instance().unwrap();
    plugin.init();
    println!("{:?}", plugin.get_info());

    plugin.set_sample_rate(SAMPLE_RATE as f32);
    plugin.set_block_size(BLOCK_SIZE as i64);

    plugin.resume();

    let (window_width, window_height) = unsafe {
        let mut rect = ptr::null::<Rect>();
        plugin.dispatch(OpCode::EditorGetRect, 0, 0, &mut rect as *mut *const _ as *mut c_void, 0.0);

        if rect != ptr::null() {
            let rect = *rect;
            (rect.right - rect.left, rect.bottom - rect.top)
        } else {
            panic!("EditorGetRect failed");
        }
    };

    let event_loop = EventLoop::<Vec<f32>>::with_user_event();

    let event_loop_proxy = event_loop.create_proxy();

    let window = WindowBuilder::new()
        .with_inner_size(LogicalSize::new(window_width, window_height))
        .with_resizable(false)
        .with_title("VST Host")
        .build(&event_loop)
        .unwrap();

    let handle = window.raw_window_handle();

    let handle_ptr = match handle {
        RawWindowHandle::MacOS(macos) => macos.ns_view,
        _ => panic!("don't know this platform"),
    };

    unsafe {
        plugin.dispatch(OpCode::EditorOpen, 0, 0, handle_ptr, 0.0);
    }

    thread::spawn(move || {
        let start = Instant::now();
        let mut t = 0;

        loop {
            let mut samples = vec![0f32; BLOCK_SIZE];

            for i in 0..BLOCK_SIZE {
                let t_sec = (t + i) as f64 / SAMPLE_RATE as f64;
                samples[i] = f64::sin(t_sec * 2.0 * f64::consts::PI * 220.0) as f32 / 2.0;
            }

            event_loop_proxy.send_event(samples);

            t += BLOCK_SIZE;

            let wait_until = start + Duration::from_millis(t as u64 * 1000 / SAMPLE_RATE as u64);
            let now = Instant::now();
            if wait_until > now {
                thread::sleep(wait_until - now);
            }
        }
    });

    event_loop.run(move |event, _, cflow| {
        *cflow = ControlFlow::Wait;

        match event {
            Event::NewEvents(_) => {}
            Event::WindowEvent { window_id, event } => {
                // println!("WindowEvent({:?}): {:?}", window_id, event);
            }
            Event::DeviceEvent { device_id, event } => {
                // println!("DeviceEvent({:?}): {:?}", device_id, event);
            }
            Event::UserEvent(samples) => {
                static EMPTY_INPUTS: [f32; BLOCK_SIZE] = [0f32; BLOCK_SIZE];

                let inputs = [
                    samples.as_ptr(),
                    samples.as_ptr(),
                    EMPTY_INPUTS.as_ptr(),
                    EMPTY_INPUTS.as_ptr(),
                    EMPTY_INPUTS.as_ptr(),
                    EMPTY_INPUTS.as_ptr(),
                    EMPTY_INPUTS.as_ptr(),
                    EMPTY_INPUTS.as_ptr(),
                ];

                let mut output_buffers = [
                    vec![0f32; BLOCK_SIZE],
                    vec![0f32; BLOCK_SIZE],
                    vec![0f32; BLOCK_SIZE],
                    vec![0f32; BLOCK_SIZE],
                    vec![0f32; BLOCK_SIZE],
                    vec![0f32; BLOCK_SIZE],
                    vec![0f32; BLOCK_SIZE],
                    vec![0f32; BLOCK_SIZE],
                ];

                let mut outputs = output_buffers.iter_mut()
                    .map(|buff| buff.as_mut_ptr())
                    .collect::<Vec<_>>();

                let mut audio_buffer = unsafe {
                    AudioBuffer::from_raw(
                        inputs.len(),
                        outputs.len(),
                        inputs.as_ptr(),
                        outputs.as_mut_ptr(),
                        BLOCK_SIZE,
                    )
                };

                plugin.process(&mut audio_buffer);
            }
            Event::Suspended => {}
            Event::Resumed => {}
            Event::MainEventsCleared => {}
            Event::RedrawRequested(window_id) => {
                println!("redrawing window: {:?}", window_id);
            }
            Event::RedrawEventsCleared => {}
            Event::LoopDestroyed => {}
        }
    });
}
