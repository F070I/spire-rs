#import "AutoSnakeSaverView.h"

static NSString * const kModule = @"com.autosnakesaver.monterey.intel";

@implementation AutoSnakeSaverView

- (instancetype)initWithFrame:(NSRect)frame isPreview:(BOOL)isPreview {
    self = [super initWithFrame:frame isPreview:isPreview];
    if (!self) return nil;
    self.animationTimeInterval = 1.0 / 60.0;
    ScreenSaverDefaults *defaults = [ScreenSaverDefaults defaultsForModuleWithName:kModule];
    [defaults registerDefaults:@{@"uiMode":@"full", @"gridDensity":@"fine", @"worldMode":@"wall", @"clockMode":@"minute", @"clockBrightness":@"subtle", @"theme":@"classic", @"speed":@"normal", @"fpsMode":@"auto", @"fpsCustom":@"90"}];
    self.wantsLayer = YES;
    WKWebViewConfiguration *configuration = [[WKWebViewConfiguration alloc] init];
    self.webView = [[WKWebView alloc] initWithFrame:self.bounds configuration:configuration];
    self.webView.autoresizingMask = NSViewWidthSizable | NSViewHeightSizable;
    self.webView.navigationDelegate = self;
    self.webView.wantsLayer = YES;
    self.webView.layer.backgroundColor = NSColor.blackColor.CGColor;
    [self addSubview:self.webView];
    NSURL *resourceURL = [[NSBundle bundleForClass:self.class] resourceURL];
    NSURL *webRoot = [resourceURL URLByAppendingPathComponent:@"AutoSnakeWeb" isDirectory:YES];
    NSURL *pageURL = [webRoot URLByAppendingPathComponent:@"index.html"];
    [self.webView loadFileURL:pageURL allowingReadAccessToURL:resourceURL];
    return self;
}

- (void)startAnimation { [super startAnimation]; [self.webView evaluateJavaScript:@"if(window.__AUTO_SNAKE_RESUME__)window.__AUTO_SNAKE_RESUME__()" completionHandler:nil]; [self.webView setNeedsDisplay:YES]; }
- (void)stopAnimation { [self.webView evaluateJavaScript:@"if(window.__AUTO_SNAKE_PAUSE__)window.__AUTO_SNAKE_PAUSE__()" completionHandler:nil]; [super stopAnimation]; }
- (void)animateOneFrame { /* WebKit drives requestAnimationFrame; ScreenSaverView schedules lifecycle. */ }
- (BOOL)hasConfigureSheet { return YES; }

- (NSPopUpButton *)popupWithItems:(NSArray<NSString *> *)items selected:(NSString *)selected {
    NSPopUpButton *p = [[NSPopUpButton alloc] initWithFrame:NSMakeRect(0,0,220,26) pullsDown:NO];
    [p addItemsWithTitles:items];
    NSInteger i = [items indexOfObject:selected];
    if (i != NSNotFound) [p selectItemAtIndex:i];
    return p;
}

- (NSWindow *)configureSheet {
    if (self.settingsSheet) return self.settingsSheet;
    ScreenSaverDefaults *d = [ScreenSaverDefaults defaultsForModuleWithName:kModule];
    [d registerDefaults:@{@"uiMode":@"full", @"gridDensity":@"fine", @"worldMode":@"wall", @"clockMode":@"minute", @"clockBrightness":@"subtle", @"theme":@"classic", @"speed":@"normal", @"fpsMode":@"auto", @"fpsCustom":@"90"}];
    self.settingsSheet = [[NSWindow alloc] initWithContentRect:NSMakeRect(0,0,500,430) styleMask:(NSWindowStyleMaskTitled|NSWindowStyleMaskClosable) backing:NSBackingStoreBuffered defer:NO];
    self.settingsSheet.title = @"AutoSnakeSaver 设置";
    NSView *content = [[NSView alloc] initWithFrame:NSMakeRect(0,0,500,430)];
    self.settingsSheet.contentView = content;
    NSArray *labels = @[@"界面显示", @"网格密度", @"世界边界", @"点阵时钟", @"时钟亮度", @"配色主题", @"蛇的速度", @"帧率上限"];
    NSArray *keys = @[@"uiMode", @"gridDensity", @"worldMode", @"clockMode", @"clockBrightness", @"theme", @"speed", @"fpsMode"];
    NSArray *items = @[
      @[@"完整界面",@"极简（仅时钟）",@"纯贪吃蛇"],
      @[@"精细（v21）",@"标准",@"粗颗粒"],
      @[@"边界避障",@"穿越屏幕边缘"],
      @[@"关闭",@"分钟",@"秒钟"],
      @[@"淡雅",@"标准",@"清晰"],
      @[@"经典绿",@"薄荷绿",@"青蓝"],
      @[@"慢速",@"标准",@"快速"],
      @[@"自动",@"30",@"60",@"120",@"144",@"165",@"240",@"自定义"]
    ];
    NSArray *values = @[
      @[@"full",@"minimal",@"snake_only"], @[@"fine",@"standard",@"coarse"], @[@"wall",@"wrap"],
      @[@"off",@"minute",@"second"], @[@"subtle",@"standard",@"clear"], @[@"classic",@"mint",@"cyan"],
      @[@"slow",@"normal",@"fast"], @[@"auto",@"30",@"60",@"120",@"144",@"165",@"240",@"custom"]
    ];
    for (NSInteger row=0; row<labels.count; row++) {
      NSTextField *label = [NSTextField labelWithString:labels[row]];
      label.frame = NSMakeRect(24, 386-row*40, 130, 24);
      [content addSubview:label];
      NSPopUpButton *popup = [self popupWithItems:items[row] selected:items[row][MAX(0,[values[row] indexOfObject:[d stringForKey:keys[row]]])]];
      popup.frame = NSMakeRect(170, 384-row*40, 280, 26);
      popup.tag = row;
      [content addSubview:popup];
      switch(row) { case 0:self.uiModePopup=popup;break; case 1:self.gridPopup=popup;break; case 2:self.worldPopup=popup;break; case 3:self.clockPopup=popup;break; case 4:self.brightnessPopup=popup;break; case 5:self.themePopup=popup;break; case 6:self.speedPopup=popup;break; case 7:self.fpsPopup=popup;break; }
    }
    NSTextField *fpsLabel = [NSTextField labelWithString:@"自定义 FPS（15–360）"];
    fpsLabel.frame = NSMakeRect(24, 54, 145, 24); [content addSubview:fpsLabel];
    self.customFpsField = [[NSTextField alloc] initWithFrame:NSMakeRect(170, 52, 90, 26)];
    self.customFpsField.stringValue = [d stringForKey:@"fpsCustom"] ?: @"90"; [content addSubview:self.customFpsField];
    NSButton *cancel = [NSButton buttonWithTitle:@"取消" target:self action:@selector(cancelSettings:)];
    cancel.frame = NSMakeRect(300, 12, 75, 30); [content addSubview:cancel];
    NSButton *save = [NSButton buttonWithTitle:@"保存" target:self action:@selector(saveSettings:)];
    save.frame = NSMakeRect(385, 12, 75, 30); save.keyEquivalent = @"\r"; [content addSubview:save];
    return self.settingsSheet;
}

- (void)cancelSettings:(id)sender { [self.settingsSheet.sheetParent endSheet:self.settingsSheet returnCode:NSModalResponseCancel]; }

- (void)saveSettings:(id)sender {
    NSArray *popups = @[self.uiModePopup,self.gridPopup,self.worldPopup,self.clockPopup,self.brightnessPopup,self.themePopup,self.speedPopup,self.fpsPopup];
    NSArray *keys = @[@"uiMode",@"gridDensity",@"worldMode",@"clockMode",@"clockBrightness",@"theme",@"speed",@"fpsMode"];
    NSArray *values = @[
      @[@"full",@"minimal",@"snake_only"], @[@"fine",@"standard",@"coarse"], @[@"wall",@"wrap"],
      @[@"off",@"minute",@"second"], @[@"subtle",@"standard",@"clear"], @[@"classic",@"mint",@"cyan"],
      @[@"slow",@"normal",@"fast"], @[@"auto",@"30",@"60",@"120",@"144",@"165",@"240",@"custom"]
    ];
    ScreenSaverDefaults *d = [ScreenSaverDefaults defaultsForModuleWithName:kModule];
    for (NSInteger i=0;i<popups.count;i++) [d setObject:values[i][popups[i].indexOfSelectedItem] forKey:keys[i]];
    NSInteger fps = self.customFpsField.stringValue.integerValue;
    fps = MAX(15, MIN(360, fps > 0 ? fps : 90));
    [d setObject:[NSString stringWithFormat:@"%ld",(long)fps] forKey:@"fpsCustom"];
    [d synchronize];
    [self applySavedPreferences];
    [self.settingsSheet.sheetParent endSheet:self.settingsSheet returnCode:NSModalResponseOK];
}

- (void)applySavedPreferences {
    ScreenSaverDefaults *d = [ScreenSaverDefaults defaultsForModuleWithName:kModule];
    NSDictionary *map = @{
      @"uiMode":@"uiMode", @"gridDensity":@"gridDensity", @"worldMode":@"worldMode", @"clockMode":@"clockMode",
      @"clockBrightness":@"clockBrightness", @"theme":@"theme", @"speed":@"speed", @"fpsMode":@"fpsMode", @"fpsCustom":@"fpsCustom"
    };
    NSMutableDictionary *s = [NSMutableDictionary dictionary];
    for (NSString *key in map) s[map[key]] = [d stringForKey:key] ?: @"";
    NSString *jsonString = [[NSString alloc] initWithData:[NSJSONSerialization dataWithJSONObject:s options:0 error:nil] encoding:NSUTF8StringEncoding];
    NSString *js = [NSString stringWithFormat:@"if(window.__AUTO_SNAKE_APPLY_SETTINGS__){window.__AUTO_SNAKE_APPLY_SETTINGS__(%@)}else{window.__AUTO_SNAKE_PENDING_SETTINGS__=%@}", jsonString, jsonString];
    [self.webView evaluateJavaScript:js completionHandler:nil];
}

- (void)webView:(WKWebView *)webView didFinishNavigation:(WKNavigation *)navigation { [self applySavedPreferences]; }
@end
