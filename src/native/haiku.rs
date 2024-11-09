use {
    crate::{
        event::{EventHandler, MouseButton},
        native::NativeDisplayData,
        KeyCode, KeyMods,
    },
    std::{
        convert::{TryFrom, TryInto},
        mem::size_of,
        os::raw::c_void,
        thread,
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

#[derive(Debug)]
enum Message {
    ViewChanged {
        width: i32,
        height: i32,
    },
    ViewCreated,
    ViewDestroyed,
    MouseMoved {
        x: f32,
        y: f32,
    },
    MouseButtonDown {
        x: f32,
        y: f32,
    },
    MouseButtonUp {
        x: f32,
        y: f32,
    },
    Character {
        character: char,
        modifiers: i32,
        repeat: bool,
    },
    KeyDown {
        keycode: i32,
        modifiers: i32,
        repeat: bool,
    },
    KeyUp {
        keycode: i32,
        modifiers: i32,
    },
}
unsafe impl Send for Message {}
const MSG_SIZE: usize = size_of::<Message>();

#[allow(non_camel_case_types)]
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
            3 => Ok(MouseMoved { x: 0., y: 0. }),
            4 => Ok(MouseButtonDown { x: 0., y: 0. }),
            5 => Ok(MouseButtonUp { x: 0., y: 0. }),
            6 => Ok(Character {
                character: 'a',
                modifiers: 0,
                repeat: false,
            }),
            7 => Ok(KeyDown {
                keycode: 0,
                modifiers: 0,
                repeat: false,
            }),
            8 => Ok(KeyUp {
                keycode: 0,
                modifiers: 0,
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
            MouseMoved { .. } => Ok(3),
            MouseButtonDown { .. } => Ok(4),
            MouseButtonUp { .. } => Ok(5),
            Character { .. } => Ok(6),
            KeyDown { .. } => Ok(7),
            KeyUp { .. } => Ok(8),
        }
    }
}

static mut MSG_PORT: port_id = 0;

#[allow(non_camel_case_types)]
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
            }
            ViewCreated => {
                res[0] = 1;
            }
            ViewDestroyed => {
                res[0] = 2;
            }
            MouseMoved { x, y } => {
                let mut index = 0;
                res[0] = 3;
                index += 1;
                let bytes = x.to_ne_bytes();
                for i in 0..size_of::<f32>() {
                    res[index + i] = bytes[i];
                }
                index += size_of::<f32>();
                let bytes = y.to_ne_bytes();
                for i in 0..size_of::<f32>() {
                    res[index + i] = bytes[i];
                }
            }
            MouseButtonDown { x, y } => {
                let mut index = 0;
                res[0] = 4;
                index += 1;
                let bytes = x.to_ne_bytes();
                for i in 0..size_of::<f32>() {
                    res[index + i] = bytes[i];
                }
                index += size_of::<f32>();
                let bytes = y.to_ne_bytes();
                for i in 0..size_of::<f32>() {
                    res[index + i] = bytes[i];
                }
            }
            MouseButtonUp { x, y } => {
                let mut index = 0;
                res[0] = 5;
                index += 1;
                let bytes = x.to_ne_bytes();
                for i in 0..size_of::<f32>() {
                    res[index + i] = bytes[i];
                }
                index += size_of::<f32>();
                let bytes = y.to_ne_bytes();
                for i in 0..size_of::<f32>() {
                    res[index + i] = bytes[i];
                }
            }
            Character {
                character,
                modifiers,
                repeat,
            } => {
                let mut index = 0;
                res[0] = 6;
                index += 1;
                let bytes = (*character as u32).to_ne_bytes();
                for i in 0..size_of::<u32>() {
                    res[index + i] = bytes[i];
                }
                index += size_of::<u32>();
                let bytes = modifiers.to_ne_bytes();
                for i in 0..size_of::<i32>() {
                    res[index + i] = bytes[i];
                }
                index += size_of::<i32>();
                res[index] = *repeat as u8;
            }
            KeyDown {
                keycode,
                modifiers,
                repeat,
            } => {
                let mut index = 0;
                res[0] = 7;
                index += 1;
                let bytes = keycode.to_ne_bytes();
                for i in 0..size_of::<i32>() {
                    res[index + i] = bytes[i];
                }
                index += size_of::<i32>();
                let bytes = modifiers.to_ne_bytes();
                for i in 0..size_of::<i32>() {
                    res[index + i] = bytes[i];
                }
                index += size_of::<i32>();
                res[index] = *repeat as u8;
            }
            KeyUp {
                keycode,
                modifiers,
            } => {
                let mut index = 0;
                res[0] = 8;
                index += 1;
                let bytes = keycode.to_ne_bytes();
                for i in 0..size_of::<i32>() {
                    res[index + i] = bytes[i];
                }
                index += size_of::<i32>();
                let bytes = modifiers.to_ne_bytes();
                for i in 0..size_of::<i32>() {
                    res[index + i] = bytes[i];
                }
            }
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
                let bytes: [u8; size_of::<i32>()] =
                    v[index..index + size_of::<i32>()].try_into().unwrap();
                let width = i32::from_ne_bytes(bytes);
                index += size_of::<i32>();
                let bytes: [u8; size_of::<i32>()] =
                    v[index..index + size_of::<i32>()].try_into().unwrap();
                let height = i32::from_ne_bytes(bytes);
                let res = ViewChanged { width, height };
                Ok(res)
            }
            1 => Ok(ViewCreated),
            2 => Ok(ViewDestroyed),
            3 => {
                let mut index = 1;
                let bytes: [u8; size_of::<f32>()] =
                    v[index..index + size_of::<f32>()].try_into().unwrap();
                let x = f32::from_ne_bytes(bytes);
                index += size_of::<f32>();
                let bytes: [u8; size_of::<f32>()] =
                    v[index..index + size_of::<f32>()].try_into().unwrap();
                let y = f32::from_ne_bytes(bytes);
                let res = MouseMoved { x, y };
                Ok(res)
            }
            4 => {
                let mut index = 1;
                let bytes: [u8; size_of::<f32>()] =
                    v[index..index + size_of::<f32>()].try_into().unwrap();
                let x = f32::from_ne_bytes(bytes);
                index += size_of::<f32>();
                let bytes: [u8; size_of::<f32>()] =
                    v[index..index + size_of::<f32>()].try_into().unwrap();
                let y = f32::from_ne_bytes(bytes);
                let res = MouseButtonDown { x, y };
                Ok(res)
            }
            5 => {
                let mut index = 1;
                let bytes: [u8; size_of::<f32>()] =
                    v[index..index + size_of::<f32>()].try_into().unwrap();
                let x = f32::from_ne_bytes(bytes);
                index += size_of::<f32>();
                let bytes: [u8; size_of::<f32>()] =
                    v[index..index + size_of::<f32>()].try_into().unwrap();
                let y = f32::from_ne_bytes(bytes);
                let res = MouseButtonUp { x, y };
                Ok(res)
            }
            6 => {
                let mut index = 1;
                let bytes: [u8; size_of::<u32>()] =
                    v[index..index + size_of::<u32>()].try_into().unwrap();
                let character = char::from_u32(u32::from_ne_bytes(bytes)).unwrap();
                index += size_of::<u32>();
                let bytes: [u8; size_of::<i32>()] =
                    v[index..index + size_of::<i32>()].try_into().unwrap();
                let modifiers = i32::from_ne_bytes(bytes);
                index += size_of::<i32>();
                let repeat = v[index] != 0;
                let res = Character { character, modifiers, repeat };
                Ok(res)
            }
            7 => {
                let mut index = 1;
                let bytes: [u8; size_of::<i32>()] =
                    v[index..index + size_of::<i32>()].try_into().unwrap();
                let keycode = i32::from_ne_bytes(bytes);
                index += size_of::<i32>();
                let bytes: [u8; size_of::<i32>()] =
                    v[index..index + size_of::<i32>()].try_into().unwrap();
                let modifiers = i32::from_ne_bytes(bytes);
                index += size_of::<i32>();
                let repeat = v[index] != 0;
                let res = KeyDown { keycode, modifiers, repeat };
                Ok(res)
            }
            8 => {
                let mut index = 1;
                let bytes: [u8; size_of::<i32>()] =
                    v[index..index + size_of::<i32>()].try_into().unwrap();
                let keycode = i32::from_ne_bytes(bytes);
                index += size_of::<i32>();
                let bytes: [u8; size_of::<i32>()] =
                    v[index..index + size_of::<i32>()].try_into().unwrap();
                let modifiers = i32::from_ne_bytes(bytes);
                let res = KeyUp { keycode, modifiers };
                Ok(res)
            }
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
    send_message(&Message::ViewChanged { width, height });
}

#[no_mangle]
unsafe extern "C" fn miniquad_mouse_moved(x: f32, y: f32) {
    send_message(&Message::MouseMoved { x, y });
}

#[no_mangle]
unsafe extern "C" fn miniquad_mouse_button_down(x: f32, y: f32) {
    send_message(&Message::MouseButtonDown { x, y });
}

#[no_mangle]
unsafe extern "C" fn miniquad_mouse_button_up(x: f32, y: f32) {
    send_message(&Message::MouseButtonUp { x, y });
}

#[allow(non_camel_case_types)]
#[repr(i32)]
pub enum HaikuModifiers {
    B_SHIFT_KEY = 0x00000001,
    B_COMMAND_KEY = 0x00000002,
    B_CONTROL_KEY = 0x00000004,
    B_CAPS_LOCK = 0x00000008,
    B_SCROLL_LOCK = 0x00000010,
    B_NUM_LOCK = 0x00000020,
    B_OPTION_KEY = 0x00000040,
    B_MENU_KEY = 0x00000080,
    B_LEFT_SHIFT_KEY = 0x00000100,
    B_RIGHT_SHIFT_KEY = 0x00000200,
    B_LEFT_COMMAND_KEY = 0x00000400,
    B_RIGHT_COMMAND_KEY = 0x00000800,
    B_LEFT_CONTROL_KEY = 0x00001000,
    B_RIGHT_CONTROL_KEY = 0x00002000,
    B_LEFT_OPTION_KEY = 0x00004000,
    B_RIGHT_OPTION_KEY = 0x00008000,
}

unsafe fn key_mods(modifiers: i32) -> KeyMods {
    use HaikuModifiers::*;

    let mut mods = KeyMods::default();

    if modifiers | B_SHIFT_KEY as i32 != 0 {
        mods.shift = true;
    }
    if modifiers | B_CONTROL_KEY as i32 != 0 {
        mods.ctrl = true;
    }
    if modifiers | B_MENU_KEY as i32 != 0 {
        mods.alt = true;
    }
    if modifiers | B_OPTION_KEY as i32 != 0 {
        mods.logo = true;
    }

    mods
}


pub fn convert_keycode(keycode: i32) -> Option<KeyCode> {
    Some(match keycode {
        0x12 => KeyCode::Key1,
        0x13 => KeyCode::Key2,
        0x14 => KeyCode::Key3,
        0x15 => KeyCode::Key4,
        0x16 => KeyCode::Key5,
        0x17 => KeyCode::Key6,
        0x18 => KeyCode::Key7,
        0x19 => KeyCode::Key8,
        0x1a => KeyCode::Key9,
        0x1b => KeyCode::Key0,
        0x1c => KeyCode::Minus,
        0x1d => KeyCode::Equal,

        0x23 => KeyCode::KpDivide,
        0x24 => KeyCode::KpMultiply,
        0x25 => KeyCode::KpSubtract,
        0x26 => KeyCode::Tab,

        0x27 => KeyCode::Q,
        0x28 => KeyCode::W,
        0x29 => KeyCode::E,
        0x2A => KeyCode::R,
        0x2B => KeyCode::T,
        0x2C => KeyCode::Y,
        0x2D => KeyCode::U,
        0x2E => KeyCode::I,
        0x2F => KeyCode::O,
        0x30 => KeyCode::P,

        0x31 => KeyCode::LeftBracket,
        0x32 => KeyCode::RightBracket,
        0x33 => KeyCode::Backslash,

        0x3A => KeyCode::KpAdd,

        0x3C => KeyCode::A,
        0x3D => KeyCode::S,
        0x3E => KeyCode::D,
        0x3F => KeyCode::F,
        0x40 => KeyCode::G,
        0x41 => KeyCode::H,
        0x42 => KeyCode::J,
        0x43 => KeyCode::K,
        0x44 => KeyCode::L,

        0x45 => KeyCode::Semicolon,
        0x46 => KeyCode::Apostrophe,
        0x47 => KeyCode::Enter,

        0x4C => KeyCode::Z,
        0x4D => KeyCode::X,
        0x4E => KeyCode::C,
        0x4F => KeyCode::V,
        0x50 => KeyCode::B,
        0x51 => KeyCode::N,
        0x52 => KeyCode::M,

        0x53 => KeyCode::Comma,
        0x54 => KeyCode::Period,
        0x55 => KeyCode::Slash,

        0x5E => KeyCode::Space,

        0x57 => KeyCode::Up,
        0x61 => KeyCode::Left,
        0x62 => KeyCode::Down,
        0x63 => KeyCode::Right,

        _ => return None,
    })
}

#[no_mangle]
unsafe extern "C" fn miniquad_char(bytes: *const u8, byte_len: u8, modifiers: i32, repeat: i32) {
    let bytes = &*std::ptr::slice_from_raw_parts(bytes, byte_len as usize) as &[u8];
    let s = std::str::from_utf8(bytes).unwrap();
    let character = s.chars().next().unwrap();
    let repeat = repeat > 0;
    send_message(&Message::Character {
        character,
        modifiers,
        repeat,
    });
}

#[no_mangle]
unsafe extern "C" fn miniquad_key_down(keycode: i32, modifiers: i32, repeat: i32) {
    send_message(&Message::KeyDown {keycode, modifiers, repeat: repeat > 0});
}

#[no_mangle]
unsafe extern "C" fn miniquad_key_up(keycode: i32, modifiers: i32) {
    send_message(&Message::KeyUp {keycode, modifiers});
}

struct MainThreadState {
    view: *mut QuadView,
    event_handler: Box<dyn EventHandler>,
    running: bool,
    quit: bool,
    fullscreen: bool,
    update_requested: bool,
}

impl MainThreadState {
    fn process_message(&mut self, msg: Message) {
        dbg!(&msg);
        match msg {
            Message::ViewCreated => {
                self.running = true;
            }
            Message::ViewDestroyed => {
                self.running = false;
                self.quit = true;
                unsafe {
                    accept_quitting(self.view);
                }
            }
            Message::ViewChanged { width, height } => {
                {
                    let mut d = crate::native_display().lock().unwrap();
                    d.screen_width = width as _;
                    d.screen_height = height as _;
                }
                self.event_handler.resize_event(width as _, height as _);
            }
            Message::MouseMoved { x, y } => {
                self.event_handler.mouse_motion_event(x, y);
            }
            Message::MouseButtonDown { x, y } => {
                self.event_handler
                    .mouse_button_down_event(MouseButton::Left, x, y);
            }
            Message::MouseButtonUp { x, y } => {
                self.event_handler
                    .mouse_button_up_event(MouseButton::Left, x, y);
            }
            Message::Character {
                character,
                modifiers,
                repeat,
            } => {
                let modifiers = unsafe { key_mods(modifiers) };
                self.event_handler.char_event(character, modifiers, repeat);
            }
            Message::KeyDown {
                keycode,
                modifiers,
                repeat,
            } => {
                dbg!(convert_keycode(keycode));
                if let Some(key) = convert_keycode(keycode) {
                    let modifiers = unsafe { key_mods(modifiers) };
                    self.event_handler
                        .key_down_event(key, modifiers, repeat);
                }
            }
            Message::KeyUp {
                keycode,
                modifiers,
            } => {
                if let Some(key) = convert_keycode(keycode) {
                    let modifiers = unsafe { key_mods(modifiers) };
                    self.event_handler
                        .key_up_event(key, modifiers);
                }
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
            SetFullscreen(_) => {
                unimplemented!();
            }
            ShowKeyboard(_) => {
                unimplemented!();
            }
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

    fn set(&mut self, _data: &str) {
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
                            0,
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
        shim_app_run(
            app,
            new_brect(30., 30., window_width, window_height),
            title.as_ptr(),
            view,
            fullscreen,
        );
    };
}
