use {
    crate::{
        conf::Icon,
        event::{EventHandler, MouseButton},
        native::{gl, NativeDisplayData, Request},
        native_display, CursorIcon, KeyCode, KeyMods,
    },
    std::{
       mem::size_of, cell::RefCell,
        collections::HashMap,
        convert::{TryFrom, TryInto},
        os::raw::c_void,
        sync::mpsc::{self, Receiver},
        thread,
        time::{Duration, Instant},
    },
};

#[repr(C)]
pub struct ShimApp {
    _private: [u8; 0],
}

#[repr(C)]
pub struct BRect {
    _private: [u8; 0],
}

#[repr(C)]
pub struct QuadView {
    _private: [u8; 0],
}

#[link(name = "shims_lib")]
extern "C" {
    fn new_shim_app(arg: *const libc::c_char) -> *mut ShimApp;
    fn new_quad_view() -> *mut QuadView;
    fn shim_app_run(
        app: *mut ShimApp,
        rect: *mut BRect,
        name: *const libc::c_char,
        view: *mut QuadView,
        fullscreen: bool,
    ) -> i32;
    fn new_brect(left: f32, top: f32, right: f32, bottom: f32) -> *mut BRect;
    fn lock_gl(view: *mut QuadView);
    fn unlock_gl(view: *mut QuadView);
    fn swap_buffers(view: *mut QuadView);
    fn accept_quitting(view: *mut QuadView);
}

#[no_mangle]
unsafe extern "C" fn miniquad_view_created() {
    println!("miniquad_view_created");
    send_message(&Message::ViewCreated);
}

#[no_mangle]
unsafe extern "C" fn miniquad_view_destroyed() {
    println!("miniquad_view_destroyed");
    send_message(&Message::ViewDestroyed);
}

#[no_mangle]
unsafe extern "C" fn miniquad_view_changed(width: i32, height: i32) {
    println!("miniquad_view_changed");
    send_message(&Message::ViewChanged {width, height});
}

#[derive(Debug)]
enum Message {
    ViewChanged { width: i32, height: i32 },
    ViewCreated,
    ViewDestroyed,
    Character { character: u32 },
    KeyDown { keycode: KeyCode },
    KeyUp { keycode: KeyCode },
}
unsafe impl Send for Message {}
const MSG_SIZE: usize = size_of::<Message>();

type port_id = i32;

impl TryFrom<i32> for Message {
    type Error = ();

    fn try_from(v: i32) -> Result<Self, Self::Error> {
        use Message::*;
        match v {
            0 => Ok(ViewChanged {
                width: 0,
                height: 0,
            }),
            1 => Ok(ViewCreated),
            2 => Ok(ViewDestroyed),
            3 => Ok(Character { character: 0 }),
            4 => Ok(KeyDown {
                keycode: KeyCode::Space,
            }),
            5 => Ok(KeyUp {
                keycode: KeyCode::Space,
            }),
            _ => Err(()),
        }
    }
}

impl TryFrom<&Message> for i32 {
    type Error = ();

    fn try_from(v: &Message) -> Result<Self, Self::Error> {
        use Message::*;
        match v {
            ViewChanged { .. } => Ok(0),
            ViewCreated => Ok(1),
            ViewDestroyed => Ok(2),
            _ => Err(()),
        }
    }
}

static mut MSG_PORT: port_id = 0;

type status_t = i32;
const B_OK: libc::ssize_t = 0;
const B_TIMEOUT: u32 = 8;

#[link(name = "root")]
extern "C" {
    fn create_port(queue_length: i32, name: *const libc::c_char) -> port_id;
    fn write_port(
        port: port_id,
        msg_code: i32,
        msg_buffer: *mut c_void,
        buffer_size: libc::size_t,
    ) -> status_t;
    fn read_port(
        port: port_id,
        msg_code: *mut i32,
        msg_buffer: *mut c_void,
        buffer_size: libc::size_t,
    ) -> libc::ssize_t;
    fn read_port_etc(
        port: port_id,
        msg_code: *mut i32,
        msg_buffer: *mut c_void,
        buffer_size: libc::size_t,
        flags: u32,
        timeout: i64,
    ) -> libc::ssize_t;
}

impl TryFrom<&Message> for [u8; MSG_SIZE] {
    type Error = ();

    fn try_from(v: &Message) -> Result<Self, Self::Error> {
        use Message::*;
        let mut res = [0u8; MSG_SIZE];

        match v {
            ViewChanged { width, height } => {
                let mut index = 0;
                res[0] = 0;
                index += 1;
                let bytes = width.to_ne_bytes();
                for i in 0..size_of::<i32>() {
                    res[index + i] = bytes[i];
                }
                index += size_of::<i32>();
                let bytes = height.to_ne_bytes();
                for i in 0..size_of::<i32>() {
                    res[index + i] = bytes[i];
                }
            },
            ViewCreated => {
                res[0] = 1;
            },
            ViewDestroyed => {
                res[0] = 2;
            },
            _ => return Err(()),
        }
        Ok(res)
    }
}

impl TryFrom<[u8; MSG_SIZE]> for Message {
    type Error = ();

    fn try_from(v: [u8; MSG_SIZE]) -> Result<Self, Self::Error> {
        use Message::*;
        match v[0] {
            0 => {
                let mut index = 1;
                let bytes: [u8; size_of::<i32>()] = v[index..index+size_of::<i32>()].try_into().unwrap();
                let width = i32::from_ne_bytes(bytes);
                index += size_of::<i32>();
                let bytes: [u8; size_of::<i32>()] = v[index..index+size_of::<i32>()].try_into().unwrap();
                let height = i32::from_ne_bytes(bytes);
                let res = ViewChanged {width, height};
                Ok(res)
            },
            1 => Ok(ViewCreated),
            2 => Ok(ViewDestroyed),
            _ => Err(()),
        }
    }
}

fn send_message(message: &Message) {
    let msg_code = message.try_into().unwrap();
    let buffer: [u8; MSG_SIZE] = message.try_into().unwrap();
    unsafe {
        write_port(
            MSG_PORT,
            msg_code,
            buffer.as_ptr() as *mut c_void,
            buffer.len(),
        );
    }
}

struct MainThreadState {
    view: *mut QuadView,
    event_handler: Box<dyn EventHandler>,
    running: bool,
    quit: bool,
    fullscreen: bool,
    update_requested: bool,
    keymods: KeyMods,
}

impl MainThreadState {
    fn process_message(&mut self, msg: Message) {
        dbg!(&msg);
        match msg {
            Message::ViewCreated => unsafe {
                self.running = true;
            },
            Message::ViewDestroyed => unsafe {
                self.running = false;
                self.quit = true;
                unsafe { accept_quitting(self.view); }
            },
            Message::ViewChanged { width, height } => {
                {
                    let mut d = crate::native_display().lock().unwrap();
                    d.screen_width = width as _;
                    d.screen_height = height as _;
                }
                self.event_handler.resize_event(width as _, height as _);
            }
            Message::Character { character } => {
                if let Some(character) = char::from_u32(character) {
                    self.event_handler
                        .char_event(character, Default::default(), false);
                }
            }
            Message::KeyDown { keycode } => {
                match keycode {
                    KeyCode::LeftShift | KeyCode::RightShift => self.keymods.shift = true,
                    KeyCode::LeftControl | KeyCode::RightControl => self.keymods.ctrl = true,
                    KeyCode::LeftAlt | KeyCode::RightAlt => self.keymods.alt = true,
                    KeyCode::LeftSuper | KeyCode::RightSuper => self.keymods.logo = true,
                    _ => {}
                }
                self.event_handler
                    .key_down_event(keycode, self.keymods, false);
            }
            Message::KeyUp { keycode } => {
                match keycode {
                    KeyCode::LeftShift | KeyCode::RightShift => self.keymods.shift = false,
                    KeyCode::LeftControl | KeyCode::RightControl => self.keymods.ctrl = false,
                    KeyCode::LeftAlt | KeyCode::RightAlt => self.keymods.alt = false,
                    KeyCode::LeftSuper | KeyCode::RightSuper => self.keymods.logo = false,
                    _ => {}
                }
                self.event_handler.key_up_event(keycode, self.keymods);
            }
        }
    }

    fn frame(&mut self) {
        self.event_handler.update();
        self.update_requested = false;
        unsafe {
            if self.running {
                lock_gl(self.view);
                self.event_handler.draw();
                swap_buffers(self.view);
                unlock_gl(self.view);
            }
        }
    }

    fn process_request(&mut self, request: crate::native::Request) {
        use crate::native::Request::*;

        match request {
            ScheduleUpdate => {
                self.update_requested = true;
            }
            SetFullscreen(fullscreen) => {
                // unsafe {
                //     let env = attach_jni_env();
                //     set_full_screen(env, fullscreen);
                // }
                self.fullscreen = fullscreen;
            }
            ShowKeyboard(show) => unsafe {
                unimplemented!();
            },
            _ => {}
        }
    }
}

pub struct HaikuClipboard {}
impl HaikuClipboard {
    pub fn new() -> HaikuClipboard {
        HaikuClipboard {}
    }
}
impl crate::native::Clipboard for HaikuClipboard {
    fn get(&mut self) -> Option<String> {
        unimplemented!();
    }

    fn set(&mut self, data: &str) {
        unimplemented!();
    }
}

unsafe fn get_proc_address(name: *const u8) -> Option<unsafe extern "C" fn()> {
    mod libc {
        use std::ffi::{c_char, c_int, c_void};

        pub const RTLD_LAZY: c_int = 1;
        extern "C" {
            pub fn dlopen(filename: *const c_char, flag: c_int) -> *mut c_void;
            pub fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
        }
    }
    static mut OPENGL: *mut std::ffi::c_void = std::ptr::null_mut();

    if OPENGL.is_null() {
        OPENGL = libc::dlopen(
            b"/boot/system/lib/libGL.so\0".as_ptr() as _,
            libc::RTLD_LAZY,
        );
    }

    assert!(!OPENGL.is_null());

    let symbol = libc::dlsym(OPENGL, name as _);
    if symbol.is_null() {
        return None;
    }
    Some(unsafe { std::mem::transmute_copy(&symbol) })
}

pub unsafe fn run<F>(conf: crate::conf::Conf, f: F)
where
    F: 'static + FnOnce() -> Box<dyn EventHandler>,
{
    crate::native::gl::load_gl_funcs(|proc| {
        let name = std::ffi::CString::new(proc).unwrap();
        get_proc_address(name.as_ptr() as _)
    });

    let app = unsafe {
        new_shim_app("application/x-vnd.Haiku-Miniquad\0".as_ptr() as *const libc::c_char)
    };
    let view = unsafe { new_quad_view() };

    // if conf.fullscreen {
    //     let env = attach_jni_env();
    //     set_full_screen(env, true);
    // }

    // yeah, just adding Send to outer F will do it, but it will brake the API
    // in other backends
    struct SendHack<F>(F);
    unsafe impl<F> Send for SendHack<F> {}

    let f = SendHack(f);
    let hacked_view = SendHack(view);

    let queue_length = (MSG_SIZE * 8).try_into().unwrap();
    MSG_PORT = create_port(queue_length, "msg port\0".as_ptr() as *const libc::c_char);

    let title = std::ffi::CString::new(conf.window_title.as_str()).unwrap();
    let window_width = conf.window_width as f32;
    let window_height = conf.window_height as f32;
    let fullscreen = conf.fullscreen;

    thread::spawn(move || {
        let (tx, requests_rx) = std::sync::mpsc::channel();
        let clipboard = Box::new(HaikuClipboard::new());
        crate::set_display(NativeDisplayData {
            high_dpi: conf.high_dpi,
            blocking_event_loop: conf.platform.blocking_event_loop,
            ..NativeDisplayData::new(conf.window_width, conf.window_height, tx, clipboard)
        });

        lock_gl(hacked_view.0);
        let event_handler = f.0();
        unlock_gl(hacked_view.0);
        let mut s = MainThreadState {
            view: hacked_view.0,
            event_handler,
            running: false,
            quit: false,
            fullscreen: conf.fullscreen,
            update_requested: true,
            keymods: KeyMods {
                shift: false,
                ctrl: false,
                alt: false,
                logo: false,
            },
        };

        while !s.quit {
            while let Ok(request) = requests_rx.try_recv() {
                s.process_request(request);
            }

            let block_on_wait = conf.platform.blocking_event_loop && !s.update_requested;

            if block_on_wait {
                let message = [0u8; MSG_SIZE];
                let mut msg_code = 0;
                let res = unsafe {
                    read_port(
                        MSG_PORT,
                        &mut msg_code as *mut i32,
                        message.as_ptr() as *mut c_void,
                        message.len(),
                    )
                };

                if res > B_OK {
                    s.process_message(message.try_into().unwrap());
                }
            } else {
                // process all the messages from the main thread
                loop {
                    let message = [0u8; MSG_SIZE];
                    let mut msg_code = 0;
                    let res = unsafe {
                        read_port_etc(
                            MSG_PORT,
                            &mut msg_code as *mut i32,
                            message.as_ptr() as *mut c_void,
                            message.len(),
                            B_TIMEOUT,
                            0
                        )
                    };

                    if res > B_OK {
                        s.process_message(message.try_into().unwrap());
                    } else {
                        break;
                    }
                }
            }

            if !conf.platform.blocking_event_loop || s.update_requested {
                s.frame();
            }

            thread::yield_now();
        }
    });

    unsafe {
        shim_app_run(app, new_brect(30., 30., window_width, window_height), title.as_ptr(), view, fullscreen);
    };
}
