#include <memory>

#include <Application.h>
#include <GLView.h>
#include <DirectWindow.h>
#include <Rect.h>

class ShimApp : public BApplication {
public:
    ShimApp(const char* sign): BApplication(sign) {}; 
};

class QuadView : public BGLView {
    public:
        bool            fLimitFps;
                        QuadView(BRect rect, const char* name,
                            ulong resizingMode, ulong options);
                        ~QuadView();

        virtual void    MouseDown(BPoint point);
        virtual void    MouseUp(BPoint point);
        virtual void    MouseMoved(BPoint point, uint32 transit, const BMessage *msg);

        virtual void    MessageReceived(BMessage* msg);
        virtual void    AttachedToWindow();
        virtual void    DetachedFromWindow();
        virtual void    FrameResized(float width, float height);

        sem_id          quittingSem;

    private:
        unsigned int    VAO;
        unsigned int    VBO;
        unsigned int    vertexShader = 0;
        unsigned int    fragmentShader = 0;
        unsigned int    shaderProgram = 0;
};

extern "C" {
    void miniquad_view_created(void);
    void miniquad_view_destroyed(void);
    void miniquad_view_changed(int width, int height);

    ShimApp* new_shim_app(const char* sign); 
    QuadView* new_quad_view();
    int shim_app_run(ShimApp* app, BRect* rect, const char* name, QuadView* view);
    BRect* new_brect(float left, float top, float right, float bottom);
}
