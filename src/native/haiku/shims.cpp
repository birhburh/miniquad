#include "shims.h"

#include <stdio.h>
#include <new>

#include <Application.h>
#include <Catalog.h>
#include <DirectWindow.h>
#include <InterfaceKit.h>
#include <Point.h>
#include <Rect.h>

QuadView::QuadView(BRect rect, const char *name, ulong resizingMode,
    ulong options)
    : BGLView(rect, name, resizingMode, 0, options)
{
    printf("[OpenGL Renderer]          %s\n", glGetString(GL_RENDERER));
    printf("[OpenGL Vendor]            %s\n", glGetString(GL_VENDOR));
    printf("[OpenGL Version]           %s\n", glGetString(GL_VERSION));
    GLint profile;  glGetIntegerv(GL_CONTEXT_PROFILE_MASK, &profile);
    printf("[OpenGL Profile]           %s\n", profile ? "Core" : "Compatibility");
    printf("[OpenGL Shading Language]  %s\n", glGetString(GL_SHADING_LANGUAGE_VERSION));

    quittingSem = create_sem(1, "quitting sem");
}

QuadView::~QuadView()
{
    delete_sem(quittingSem);
}

void
QuadView::AttachedToWindow()
{
    miniquad_view_created();
    BGLView::AttachedToWindow();
}

void
QuadView::DetachedFromWindow()
{
    miniquad_view_destroyed();
    BGLView::DetachedFromWindow();
}

void
QuadView::FrameResized(float width, float height)
{
    miniquad_view_changed(static_cast<int>(width), static_cast<int>(height));
    BGLView::FrameResized(width, height);
}

void
QuadView::MouseMoved(BPoint point, uint32 transit, const BMessage *msg)
{
}

void
QuadView::MouseUp(BPoint point)
{
}

void
QuadView::MouseDown(BPoint point)
{
}

void
QuadView::MessageReceived(BMessage* msg)
{
    BGLView::MessageReceived(msg);
}

class QuadWindow : public BDirectWindow {
        public:
            QuadWindow(BRect r, const char* name, QuadView* view);
                virtual bool    QuitRequested();
                virtual void    MessageReceived(BMessage* msg);
};

extern "C" {
    BRect* new_brect(float left, float top, float right, float bottom)
    {   
      return new BRect(left, top, right, bottom);
    }   

    ShimApp* new_shim_app(const char* sign) 
    {   
      return new ShimApp(sign);
    }   

    QuadView* new_quad_view() {
       GLenum type = BGL_RGB | BGL_DEPTH | BGL_DOUBLE;
       BRect bounds = {};
       return new(std::nothrow) QuadView(bounds, "objectView", B_FOLLOW_ALL_SIDES, type);
 
    }
    
    int shim_app_run(ShimApp* app, BRect* rect, const char* name, QuadView *view) {
        QuadWindow* fQuadWindow = new QuadWindow(*rect, name, view); 
        fQuadWindow->Show();
        return app->Run();
    }   

    void lock_gl(QuadView* view) {
        view->LockGL();
    }

    void unlock_gl(QuadView* view) {
        view->UnlockGL();
    }

    void swap_buffers(QuadView* view) {
        view->SwapBuffers();
    }
}

QuadWindow::QuadWindow(BRect rect, const char* name, QuadView *view)
        :   
        BDirectWindow(rect, name, B_TITLED_WINDOW, 0)
{
        Lock();
        BRect bounds = Bounds();
        BView *subView = new BView(bounds, "subview", B_FOLLOW_ALL, 0); 
        AddChild(subView); 
    
        subView->AddChild(view);

        bounds = subView->Bounds(); 
        view->MoveTo(bounds.left, bounds.top);
        view->ResizeTo(bounds.right, bounds.bottom);
        miniquad_view_changed(static_cast<int>(bounds.right), static_cast<int>(bounds.bottom));

        SetSizeLimits(32, 1024, 32, 1024);
        Unlock();
}


bool
QuadWindow::QuitRequested()
{
        be_app->PostMessage(B_QUIT_REQUESTED);
        return true;
}

void
QuadWindow::MessageReceived(BMessage* msg)
{
        switch (msg->what) {
                default:
                        BDirectWindow::MessageReceived(msg);
        }
}


