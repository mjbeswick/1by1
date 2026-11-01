// Offscreen Milkdrop renderer for the bottom-left panel.
// projectM 4 renders through the current OpenGL context into an FBO.
#define GL_SILENCE_DEPRECATION 1
#include <OpenGL/OpenGL.h>
#include <OpenGL/gl.h>
#include <OpenGL/glext.h>
#include <projectM-4/projectM.h>
#include <projectM-4/playlist.h>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

struct Milkdrop {
    projectm_handle pm = nullptr;
    projectm_playlist_handle playlist = nullptr;
    CGLContextObj ctx = nullptr;
    GLuint fbo = 0;
    GLuint tex = 0;
    GLuint depth = 0;
    int w = 0;
    int h = 0;
    std::vector<uint8_t> pixels;
    bool ready = false;
};

static bool make_current(Milkdrop* m) {
    if (!m || !m->ctx) return false;
    return CGLSetCurrentContext(m->ctx) == kCGLNoError;
}

static bool create_buffers(Milkdrop* m, int w, int h) {
    if (!make_current(m)) return false;
    m->w = w;
    m->h = h;
    if (!m->tex) glGenTextures(1, &m->tex);
    glBindTexture(GL_TEXTURE_2D, m->tex);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
    glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA8, w, h, 0, GL_RGBA, GL_UNSIGNED_BYTE, nullptr);
    if (!m->depth) glGenRenderbuffersEXT(1, &m->depth);
    glBindRenderbufferEXT(GL_RENDERBUFFER_EXT, m->depth);
    glRenderbufferStorageEXT(GL_RENDERBUFFER_EXT, GL_DEPTH_COMPONENT24, w, h);
    if (!m->fbo) glGenFramebuffersEXT(1, &m->fbo);
    glBindFramebufferEXT(GL_FRAMEBUFFER_EXT, m->fbo);
    glFramebufferTexture2DEXT(GL_FRAMEBUFFER_EXT, GL_COLOR_ATTACHMENT0_EXT,
                              GL_TEXTURE_2D, m->tex, 0);
    glFramebufferRenderbufferEXT(GL_FRAMEBUFFER_EXT, GL_DEPTH_ATTACHMENT_EXT,
                                 GL_RENDERBUFFER_EXT, m->depth);
    glDrawBuffer(GL_COLOR_ATTACHMENT0_EXT);
    GLenum ok = glCheckFramebufferStatusEXT(GL_FRAMEBUFFER_EXT);
    if (ok != GL_FRAMEBUFFER_COMPLETE_EXT) {
        fprintf(stderr, "milkdrop fbo status %x\n", ok);
        return false;
    }
    m->pixels.assign((size_t)w * (size_t)h * 4, 0);
    return true;
}

extern "C" {

void* md_create(int w, int h, const char* preset_dir) {
    if (w < 16) w = 16;
    if (h < 16) h = 16;
    CGLPixelFormatAttribute attrs[] = {
        kCGLPFAAccelerated,
        kCGLPFAColorSize, (CGLPixelFormatAttribute)24,
        kCGLPFAAlphaSize, (CGLPixelFormatAttribute)8,
        kCGLPFADepthSize, (CGLPixelFormatAttribute)16,
        kCGLPFAOpenGLProfile, (CGLPixelFormatAttribute)kCGLOGLPVersion_GL3_Core,
        (CGLPixelFormatAttribute)0
    };
    CGLPixelFormatObj pix = nullptr;
    GLint npix = 0;
    if (CGLChoosePixelFormat(attrs, &pix, &npix) != kCGLNoError || !pix) {
        fprintf(stderr, "milkdrop: no pixel format\n");
        return nullptr;
    }
    Milkdrop* m = new Milkdrop();
    if (CGLCreateContext(pix, nullptr, &m->ctx) != kCGLNoError) {
        CGLReleasePixelFormat(pix);
        fprintf(stderr, "milkdrop: no GL context\n");
        delete m;
        return nullptr;
    }
    CGLReleasePixelFormat(pix);
    if (!create_buffers(m, w, h)) {
        CGLDestroyContext(m->ctx);
        delete m;
        return nullptr;
    }
    m->pm = projectm_create();
    if (!m->pm) {
        fprintf(stderr, "milkdrop: projectm_create failed\n");
        CGLDestroyContext(m->ctx);
        delete m;
        return nullptr;
    }
    projectm_set_window_size(m->pm, (size_t)w, (size_t)h);
    projectm_set_mesh_size(m->pm, 32, 24);
    projectm_set_fps(m->pm, 30);
    projectm_set_preset_duration(m->pm, 20.0);
    projectm_set_soft_cut_duration(m->pm, 3.0);
    projectm_set_aspect_correction(m->pm, true);
    m->playlist = projectm_playlist_create(m->pm);
    if (m->playlist) {
        uint32_t added = 0;
        if (preset_dir && *preset_dir) {
            added = projectm_playlist_add_path(m->playlist, preset_dir, true, false);
        }
        if (added == 0) {
            fprintf(stderr, "milkdrop: no presets in %s\n", preset_dir ? preset_dir : "");
        } else {
            projectm_playlist_sort(m->playlist, 0, added,
                                   SORT_PREDICATE_FILENAME_ONLY,
                                   SORT_ORDER_ASCENDING);
            projectm_playlist_set_shuffle(m->playlist, true);
            projectm_playlist_play_next(m->playlist, true);
        }
    }
    m->ready = true;
    return m;
}

void md_resize(void* ptr, int w, int h) {
    Milkdrop* m = static_cast<Milkdrop*>(ptr);
    if (!m || !m->ready || w < 16 || h < 16 || (w == m->w && h == m->h)) return;
    if (!create_buffers(m, w, h)) return;
    projectm_set_window_size(m->pm, (size_t)w, (size_t)h);
    m->ready = true;
}

void md_pcm(void* ptr, const float* stereo, int frames) {
    Milkdrop* m = static_cast<Milkdrop*>(ptr);
    if (!m || !m->ready || !stereo || frames <= 0) return;
    projectm_pcm_add_float(m->pm, stereo, (unsigned int)frames, PROJECTM_STEREO);
}

void md_render(void* ptr) {
    Milkdrop* m = static_cast<Milkdrop*>(ptr);
    if (!m || !m->ready || !make_current(m)) return;
    glBindFramebufferEXT(GL_FRAMEBUFFER_EXT, m->fbo);
    glDrawBuffer(GL_COLOR_ATTACHMENT0_EXT);
    glViewport(0, 0, m->w, m->h);
    glDisable(GL_SCISSOR_TEST);
    glClearColor(0, 0, 0, 1);
    glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);
    projectm_opengl_render_frame(m->pm);
    glPixelStorei(GL_PACK_ALIGNMENT, 1);
    glReadPixels(0, 0, m->w, m->h, GL_RGBA, GL_UNSIGNED_BYTE, m->pixels.data());
    int row = m->w * 4;
    std::vector<uint8_t> tmp(row);
    for (int y = 0; y < m->h / 2; ++y) {
        uint8_t* a = m->pixels.data() + (size_t)y * row;
        uint8_t* b = m->pixels.data() + (size_t)(m->h - 1 - y) * row;
        memcpy(tmp.data(), a, row);
        memcpy(a, b, row);
        memcpy(b, tmp.data(), row);
    }
}

const unsigned char* md_pixels(void* ptr) {
    Milkdrop* m = static_cast<Milkdrop*>(ptr);
    if (!m || !m->ready) return nullptr;
    return m->pixels.data();
}

void md_next(void* ptr) {
    Milkdrop* m = static_cast<Milkdrop*>(ptr);
    if (!m || !m->ready || !m->playlist) return;
    projectm_playlist_play_next(m->playlist, false);
}

uint32_t md_preset_count(void* ptr) {
    Milkdrop* m = static_cast<Milkdrop*>(ptr);
    return (m && m->playlist) ? projectm_playlist_size(m->playlist) : 0;
}

int md_preset_name(void* ptr, uint32_t index, char* out, int cap) {
    Milkdrop* m = static_cast<Milkdrop*>(ptr);
    if (!m || !m->playlist || !out || cap <= 0) return 0;
    char* item = projectm_playlist_item(m->playlist, index);
    if (!item) return 0;
    const char* base = strrchr(item, '/');
    base = base ? base + 1 : item;
    const char* ext = strstr(base, ".milk");
    if (ext && ext[5] == '\0') {
        snprintf(out, (size_t)cap, "%.*s", (int)(ext - base), base);
    } else {
        snprintf(out, (size_t)cap, "%s", base);
    }
    projectm_playlist_free_string(item);
    return 1;
}

uint32_t md_current_preset(void* ptr) {
    Milkdrop* m = static_cast<Milkdrop*>(ptr);
    return (m && m->playlist) ? projectm_playlist_get_position(m->playlist) : 0;
}

void md_select_preset(void* ptr, uint32_t index) {
    Milkdrop* m = static_cast<Milkdrop*>(ptr);
    if (!m || !m->ready || !m->playlist) return;
    projectm_playlist_set_position(m->playlist, index, false);
}

void md_destroy(void* ptr) {
    Milkdrop* m = static_cast<Milkdrop*>(ptr);
    if (!m) return;
    make_current(m);
    if (m->playlist) projectm_playlist_destroy(m->playlist);
    if (m->pm) projectm_destroy(m->pm);
    if (m->fbo) glDeleteFramebuffersEXT(1, &m->fbo);
    if (m->depth) glDeleteRenderbuffersEXT(1, &m->depth);
    if (m->tex) glDeleteTextures(1, &m->tex);
    if (m->ctx) CGLDestroyContext(m->ctx);
    delete m;
}

}  // extern "C"

static GLuint g_display_tex = 0;

// Upload the latest RGBA frame and draw it full-window in the current FLTK GL context.
extern "C" void md_gl_draw_frame(const unsigned char* data, int w, int h, int win_w, int win_h) {
    if (!data || w <= 0 || h <= 0 || win_w <= 0 || win_h <= 0) return;
    if (!g_display_tex) glGenTextures(1, &g_display_tex);
    glViewport(0, 0, win_w, win_h);
    glDisable(GL_DEPTH_TEST);
    glDisable(GL_BLEND);
    glClearColor(0, 0, 0, 1);
    glClear(GL_COLOR_BUFFER_BIT);
    glEnable(GL_TEXTURE_2D);
    glBindTexture(GL_TEXTURE_2D, g_display_tex);
    glPixelStorei(GL_UNPACK_ALIGNMENT, 1);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
    glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA, w, h, 0, GL_RGBA, GL_UNSIGNED_BYTE, data);
    glMatrixMode(GL_PROJECTION);
    glLoadIdentity();
    glMatrixMode(GL_MODELVIEW);
    glLoadIdentity();
    glColor4f(1, 1, 1, 1);
    glBegin(GL_QUADS);
    glTexCoord2f(0, 0); glVertex2f(-1, 1);
    glTexCoord2f(1, 0); glVertex2f(1, 1);
    glTexCoord2f(1, 1); glVertex2f(1, -1);
    glTexCoord2f(0, 1); glVertex2f(-1, -1);
    glEnd();
    glDisable(GL_TEXTURE_2D);
    glBindTexture(GL_TEXTURE_2D, 0);
}
