use miniquad::gl::*;
use miniquad::native::apple::{apple_util::*, frameworks::*};
use std::ffi::c_void;
use std::ffi::CString;
use std::mem;

#[repr(C)]
#[derive(Default)]
struct Window {
    should_close: bool,
}

#[repr(C)]
struct Vertex {
    pos: [f32; 2],
    color: [f32; 3],
}

#[rustfmt::skip]
const VERTICES: [Vertex; 3] = [
    Vertex { pos : [ -0.6, -0.4 ], color: [1., 0., 0.] },
    Vertex { pos : [  0.6, -0.4 ], color: [0., 1., 0.] },
    Vertex { pos : [  0.0,  0.6 ], color: [0., 0., 1.] },
];

pub const VERTEX: &str = r#"#version 100
attribute vec3 vCol;
attribute vec2 vPos;
varying lowp vec3 color;
void main() {
    gl_Position = vec4(vPos, 0.0, 1.0);
    color = vCol;
}"#;

pub const FRAGMENT: &str = r#"#version 100
varying lowp vec3 color;
void main() {
    gl_FragColor = vec4(color, 1.0);
}"#;

macro_rules! msg_send_ {
    ($obj:expr, $name:ident) => ({
        let res: ObjcId = msg_send!($obj, $name);
        res
    });
    ($obj:expr, $($name:ident : $arg:expr)+) => ({
        let res: ObjcId = msg_send!($obj, $($name: $arg)*);
        res
    });
}

pub fn define_window_delegate() -> *const Class {
    extern "C" fn init_with_window(this: &Object, _: Sel, init_window: *mut c_void) -> ObjcId {
        unsafe {
            let result: ObjcId = msg_send![super(this, class!(NSObject)), init];
            (*result).set_ivar("window", init_window);
            return result;
        }
    }

    extern "C" fn window_should_close(this: &Object, _: Sel, _: ObjcId) -> BOOL {
        unsafe {
            let ptr: *mut c_void = *this.get_ivar("window");
            (*(ptr as *mut Window)).should_close = true;
        }
        return NO;
    }
    let superclass = class!(NSObject);
    let mut decl = ClassDecl::new("MyWindowDelegate", superclass).unwrap();

    // Add callback methods
    unsafe {
        decl.add_method(
            sel!(initWithWindow:),
            init_with_window as extern "C" fn(&Object, Sel, *mut c_void) -> ObjcId,
        );
        decl.add_method(
            sel!(windowShouldClose:),
            window_should_close as extern "C" fn(&Object, Sel, ObjcId) -> BOOL,
        );
    }
    // Store internal state as user data
    decl.add_ivar::<*mut c_void>("window");

    return decl.register();
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
            b"/System/Library/Frameworks/OpenGL.framework/Versions/Current/OpenGL\0".as_ptr() as _,
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

fn main() {
    unsafe {
        let ns_app = msg_send_![class!(NSApplication), sharedApplication];
        msg_send_![
            ns_app,
            setActivationPolicy: NSApplicationActivationPolicy::NSApplicationActivationPolicyRegular
                as i64
        ];

        let mut window = Window::default();

        let window_delegate_class = define_window_delegate();
        let delegate = msg_send_![window_delegate_class, alloc];
        let delegate = msg_send_![delegate, initWithWindow:&window as *const _ as *mut c_void];

        if delegate == nil {
            panic!("Cocoa: Failed to create window delegate");
        }

        let content_rect = NSRect::new(0., 0., 640., 480.);
        let style_mask = NSWindowStyleMask::NSTitledWindowMask as u64
            | NSWindowStyleMask::NSClosableWindowMask as u64
            | NSWindowStyleMask::NSMiniaturizableWindowMask as u64
            | NSWindowStyleMask::NSResizableWindowMask as u64;
        let ns = msg_send_![class!(NSWindow), alloc];
        let ns = msg_send_![
            ns,
            initWithContentRect: content_rect
            styleMask: style_mask as u64
            backing: NSBackingStoreType::NSBackingStoreBuffered as u64
            defer: NO
        ];
        if ns == nil {
            panic!("Cocoa: Failed to create window");
        }
        msg_send_![ns, center];
        let view = msg_send_![class!(NSView), alloc];
        let view = msg_send_![view, init];
        msg_send_![ns, setContentView:view];
        msg_send_![ns, makeFirstResponder:view];
        let title = str_to_nsstring("OpenGLA Triangle");
        msg_send_![ns, setTitle:title];
        msg_send_![ns, setDelegate:delegate];
        msg_send_![ns, setAcceptsMouseMovedEvents:YES];
        msg_send_![ns, setRestorable:NO];

        use NSOpenGLPixelFormatAttribute::*;

        let mut attrs: Vec<u32> = vec![];

        attrs.push(NSOpenGLPFAOpenGLProfile as _);
        attrs.push(NSOpenGLPFAOpenGLProfiles::NSOpenGLProfileVersion3_2Core as _);
        attrs.push(NSOpenGLPFADoubleBuffer as _);
        attrs.push(0);

        let pixel_format = msg_send_![class!(NSOpenGLPixelFormat), alloc];
        let pixel_format = msg_send_![pixel_format, initWithAttributes: attrs.as_ptr()];
        if pixel_format == nil {
            panic!("NSGL: Failed to find a suitable pixel format");
        }

        let nsgl = msg_send_![class!(NSOpenGLContext), alloc];
        let nsgl = msg_send_![nsgl, initWithFormat: pixel_format shareContext: nil];
        if nsgl == nil {
            panic!("NSGL: Failed to create OpenGL context");
        }
        msg_send_![view, setWantsBestResolutionOpenGLSurface: YES];
        msg_send_![nsgl, setView: view];

        msg_send_![ns, orderFront: nil];
        msg_send_![ns_app, activateIgnoringOtherApps:YES];
        msg_send_![ns, makeKeyAndOrderFront:nil];

        msg_send_![nsgl, makeCurrentContext];
        load_gl_funcs(|proc| {
            let name = CString::new(proc).unwrap();

            get_proc_address(name.as_ptr() as _)
        });

        let mut vertex_buffer = 0;
        glGenBuffers(1, &mut vertex_buffer as *mut _);
        glBindBuffer(GL_ARRAY_BUFFER, vertex_buffer);
        glBufferData(
            GL_ARRAY_BUFFER,
            (mem::size_of::<Vertex>() * VERTICES.len()) as GLsizeiptr,
            VERTICES.as_ptr() as *const _,
            GL_STATIC_DRAW,
        );

        let vertex_shader = glCreateShader(GL_VERTEX_SHADER);
        let cstring = CString::new(VERTEX).unwrap();
        let csource = [cstring];
        glShaderSource(
            vertex_shader,
            1,
            csource.as_ptr() as *const _,
            std::ptr::null(),
        );
        glCompileShader(vertex_shader);

        let fragment_shader = glCreateShader(GL_FRAGMENT_SHADER);
        let cstring = CString::new(FRAGMENT).unwrap();
        let csource = [cstring];
        glShaderSource(
            fragment_shader,
            1,
            csource.as_ptr() as *const _,
            std::ptr::null(),
        );
        glCompileShader(fragment_shader);

        let program = glCreateProgram();
        glAttachShader(program, vertex_shader);
        glAttachShader(program, fragment_shader);
        glLinkProgram(program);

        let cname = CString::new("vPos").unwrap_or_else(|e| panic!("{}", e));
        let vpos_location = glGetAttribLocation(program, cname.as_ptr() as *const _);
        let cname = CString::new("vCol").unwrap_or_else(|e| panic!("{}", e));
        let vcol_location = glGetAttribLocation(program, cname.as_ptr() as *const _);

        dbg!(vpos_location);
        dbg!(vcol_location);

        let mut vertex_array = 0;

        glGenVertexArrays(1, &mut vertex_array as *mut _);
        glBindVertexArray(vertex_array);
        glEnableVertexAttribArray(vpos_location as GLuint);
        glVertexAttribPointer(
            vpos_location as GLuint,
            2,
            GL_FLOAT as GLenum,
            GL_FALSE as GLboolean,
            mem::size_of::<Vertex>() as i32,
            mem::offset_of!(Vertex, pos) as *mut _,
        );
        glEnableVertexAttribArray(vcol_location as GLuint);
        glVertexAttribPointer(
            vcol_location as GLuint,
            3,
            GL_FLOAT as GLenum,
            GL_FALSE as GLboolean,
            mem::size_of::<Vertex>() as i32,
            mem::offset_of!(Vertex, color) as *mut _,
        );

        glBindVertexArray(vertex_array);

        let distant_past: ObjcId = msg_send![class!(NSDate), distantPast];
        while !window.should_close {
            let fb_rect: NSRect = msg_send![view, convertRectToBacking:content_rect];

            let width = fb_rect.size.width;
            let height = fb_rect.size.height;
            dbg!((width, height));
            glViewport(0, 0, width as i32, height as i32);
            glClear(GL_COLOR_BUFFER_BIT);

            println!("DRAW!: {:?}", glGetString(GL_VERSION));
            glUseProgram(program);
            glDrawArrays(GL_TRIANGLES as GLenum, 0, 3);

            msg_send_![nsgl, flushBuffer];
            loop {
                let event: ObjcId = msg_send![ns_app, nextEventMatchingMask: NSEventMask::NSAnyEventMask untilDate: distant_past inMode:NSDefaultRunLoopMode dequeue:YES];
                if event == nil {
                    break;
                }
                let () = msg_send![ns_app, sendEvent:event];
            }
        }
    }
}
