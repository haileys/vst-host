use std::env;
use std::os::raw::c_void;
use std::path::PathBuf;
use std::ptr;
use std::sync::{Arc, Mutex};

use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
use vst::api;
use vst::editor::Rect;
use vst::host::{Dispatch, PluginLoader};
use vst::plugin::{OpCode, Plugin};
use winit::dpi::LogicalSize;
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

struct Host;

impl vst::host::Host for Host {
    fn automate(&self, index: i32, value: f32) { todo!() }

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

fn main() {
    let path = PathBuf::from(env::args().nth(1).expect("pass plugin path on cmdline"));

    let mut loader = PluginLoader::load(&path, Arc::new(Mutex::new(Host))).unwrap();

    let mut plugin = loader.instance().unwrap();
    plugin.init();
    println!("{:?}", plugin.get_info());

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

    let event_loop = EventLoop::new();

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

    event_loop.run(move |event, _, cflow| {
        // println!("event: {:?}", event);
    });
}
