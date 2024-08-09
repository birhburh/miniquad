/* copied from glfw */

#include <stdint.h>

#include <Carbon/Carbon.h>
#include <IOKit/hid/IOHIDLib.h>

#define GL_SILENCE_DEPRECATION

#import <Cocoa/Cocoa.h>

#define GLAD_GL_IMPLEMENTATION
#include <gl.h>

#include <objc/runtime.h>
#include <stdlib.h>
#include <stddef.h>
#include <stdio.h>

#include "linmath.h"

typedef struct Window
{
    bool            shouldClose;
} Window;

@interface MyWindowDelegate : NSObject
{
    Window* window;
}

- (instancetype)initWithWindow:(Window *)initWindow;

@end

@implementation MyWindowDelegate

- (instancetype)initWithWindow:(Window *)initWindow
{
    self = [super init];
    window = initWindow;

    return self;
}

- (BOOL)windowShouldClose:(id)sender
{
   window->shouldClose = true;
   return NO;
}

@end

static CFBundleRef framework;

typedef void (*GLFWglproc)(void);
static GLFWglproc getProcAddressNSGL(const char* procname)
{
    CFStringRef symbolName = CFStringCreateWithCString(kCFAllocatorDefault,
                                                       procname,
                                                       kCFStringEncodingASCII);

    GLFWglproc symbol = CFBundleGetFunctionPointerForName(framework,
                                                          symbolName);

    CFRelease(symbolName);

    return symbol;
}

typedef struct Vertex
{
    vec2 pos;
    vec3 col;
} Vertex;

static const Vertex vertices[3] =
{
    { { -0.6f, -0.4f }, { 1.f, 0.f, 0.f } },
    { {  0.6f, -0.4f }, { 0.f, 1.f, 0.f } },
    { {   0.f,  0.6f }, { 0.f, 0.f, 1.f } }
};

static const char* vertex_shader_text =
"#version 100\n"
"attribute vec3 vCol;\n"
"attribute vec2 vPos;\n"
"varying lowp vec3 color;\n"
"void main()\n"
"{\n"
"    gl_Position = vec4(vPos, 0.0, 1.0);\n"
"    color = vCol;\n"
"}\n";

static const char* fragment_shader_text =
"#version 100\n"
"varying lowp vec3 color;\n"
"void main()\n"
"{\n"
"    gl_FragColor = vec4(color, 1.0);\n"
"}\n";

int main () {
    @autoreleasepool {
        [NSApplication sharedApplication];

        [NSApp setActivationPolicy:NSApplicationActivationPolicyRegular];
    }

    Window* window = calloc(1, sizeof(Window));

    id delegate = [[MyWindowDelegate alloc] initWithWindow:window];
    if (delegate == nil)
    {
        printf("Cocoa: Failed to create window delegate\n");
        exit(EXIT_FAILURE);
    }

    NSRect contentRect;

    contentRect = NSMakeRect(0, 0, 640, 480);

    NSUInteger styleMask = NSWindowStyleMaskMiniaturizable |
                           NSWindowStyleMaskTitled |
                           NSWindowStyleMaskClosable |
                           NSWindowStyleMaskResizable;

    id ns = [[NSWindow alloc] initWithContentRect:contentRect
                              styleMask:styleMask
                              backing:NSBackingStoreBuffered
                              defer:NO];

    if (ns == nil)
    {
        printf("Cocoa: Failed to create window\n");
        exit(EXIT_FAILURE);
    }

   [(NSWindow*) ns center];

    id view = [[NSView alloc] init];

    [ns setContentView:view];
    [ns makeFirstResponder:view];
    [ns setTitle:@("OpenGLA Triangle")];
    [ns setDelegate:delegate];
    [ns setAcceptsMouseMovedEvents:YES];
    [ns setRestorable:NO];

    framework = CFBundleGetBundleWithIdentifier(CFSTR("com.apple.opengl"));
    if (framework == NULL)
    {
        printf("NSGL: Failed to locate OpenGL framework\n");
        exit(EXIT_FAILURE);
    }

#define ADD_ATTRIB(a) \
{ \
    assert((size_t) index < sizeof(attribs) / sizeof(attribs[0])); \
    attribs[index++] = a; \
}
#define SET_ATTRIB(a, v) { ADD_ATTRIB(a); ADD_ATTRIB(v); }

    NSOpenGLPixelFormatAttribute attribs[40];
    int index = 0;

    SET_ATTRIB(NSOpenGLPFAOpenGLProfile, NSOpenGLProfileVersion3_2Core);
    ADD_ATTRIB(NSOpenGLPFADoubleBuffer);

    ADD_ATTRIB(0);

#undef ADD_ATTRIB
#undef SET_ATTRIB

    id pixelFormat = [[NSOpenGLPixelFormat alloc] initWithAttributes:attribs];
    if (pixelFormat == nil)
    {
        printf("NSGL: Failed to find a suitable pixel format\n");
        exit(EXIT_FAILURE);
    }

    id nsgl = [[NSOpenGLContext alloc] initWithFormat:pixelFormat shareContext:nil];
    if (nsgl == nil)
    {
        printf("NSGL: Failed to create OpenGL context\n");
        exit(EXIT_FAILURE);
    }

    [view setWantsBestResolutionOpenGLSurface:TRUE];

    [nsgl setView:view];

    [ns orderFront:nil];
    [NSApp activateIgnoringOtherApps:YES];
    [ns makeKeyAndOrderFront:nil];

    [nsgl makeCurrentContext];
    gladLoadGL(getProcAddressNSGL);

    GLuint vertex_buffer;
    glGenBuffers(1, &vertex_buffer);
    glBindBuffer(GL_ARRAY_BUFFER, vertex_buffer);
    glBufferData(GL_ARRAY_BUFFER, sizeof(vertices), vertices, GL_STATIC_DRAW);

    const GLuint vertex_shader = glCreateShader(GL_VERTEX_SHADER);
    glShaderSource(vertex_shader, 1, &vertex_shader_text, NULL);
    glCompileShader(vertex_shader);

    const GLuint fragment_shader = glCreateShader(GL_FRAGMENT_SHADER);
    glShaderSource(fragment_shader, 1, &fragment_shader_text, NULL);
    glCompileShader(fragment_shader);

    const GLuint program = glCreateProgram();
    glAttachShader(program, vertex_shader);
    glAttachShader(program, fragment_shader);
    glLinkProgram(program);

    const GLint vpos_location = glGetAttribLocation(program, "vPos");
    const GLint vcol_location = glGetAttribLocation(program, "vCol");

    GLuint vertex_array;
    glGenVertexArrays(1, &vertex_array);
    glBindVertexArray(vertex_array);
    glEnableVertexAttribArray(vpos_location);
    glVertexAttribPointer(vpos_location, 2, GL_FLOAT, GL_FALSE,
                        sizeof(Vertex), (void*) offsetof(Vertex, pos));
    glEnableVertexAttribArray(vcol_location);
    glVertexAttribPointer(vcol_location, 3, GL_FLOAT, GL_FALSE,
                        sizeof(Vertex), (void*) offsetof(Vertex, col));

    glBindVertexArray(vertex_array);
    while (!window->shouldClose)
    {
        int width, height;

        const NSRect fbRect = [view convertRectToBacking:contentRect];

        width = (int) fbRect.size.width;
        height = (int) fbRect.size.height;

        printf("(width, height): (%d, %d)\n", width, height);
        glViewport(0, 0, width, height);
        glClear(GL_COLOR_BUFFER_BIT);

        printf("DRAW!: %s\n", glGetString(GL_VERSION));
        glUseProgram(program);
        glDrawArrays(GL_TRIANGLES, 0, 3);

        [nsgl flushBuffer];
        for (;;)
        {
            NSEvent* event = [NSApp nextEventMatchingMask:NSEventMaskAny
                                                untilDate:[NSDate distantPast]
                                                inMode:NSDefaultRunLoopMode
                                                dequeue:YES];
            if (event == nil)
                break;

            [NSApp sendEvent:event];
        }
    }

    exit(EXIT_SUCCESS);
    return 0;
}