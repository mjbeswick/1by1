#import <Cocoa/Cocoa.h>

extern "C" void md_window_opaque(void* handle) {
    if (!handle) return;
    NSWindow* window = (NSWindow*)handle;
    [window setOpaque:YES];
    [window setHasShadow:NO];
    [window setAlphaValue:1.0];
    [window setBackgroundColor:[NSColor blackColor]];
}
