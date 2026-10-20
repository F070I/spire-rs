#import <ScreenSaver/ScreenSaver.h>
#import <WebKit/WebKit.h>

@interface AutoSnakeSaverView : ScreenSaverView <WKNavigationDelegate>
@property(nonatomic, strong) WKWebView *webView;
@property(nonatomic, strong) NSWindow *settingsSheet;
@property(nonatomic, strong) NSPopUpButton *uiModePopup;
@property(nonatomic, strong) NSPopUpButton *gridPopup;
@property(nonatomic, strong) NSPopUpButton *worldPopup;
@property(nonatomic, strong) NSPopUpButton *clockPopup;
@property(nonatomic, strong) NSPopUpButton *brightnessPopup;
@property(nonatomic, strong) NSPopUpButton *themePopup;
@property(nonatomic, strong) NSPopUpButton *speedPopup;
@property(nonatomic, strong) NSPopUpButton *fpsPopup;
@property(nonatomic, strong) NSTextField *customFpsField;
@end
