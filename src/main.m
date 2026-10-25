// 寓+签到 - macOS 原生 WebKit 窗口。macOS 12+ / Intel, 无第三方 GUI 依赖。
#import <Cocoa/Cocoa.h>
#import <WebKit/WebKit.h>

static NSString *YJEscape(NSString *s) {
    s = s ?: @"";
    return [[[[s stringByReplacingOccurrencesOfString:@"&" withString:@"&amp;"]
              stringByReplacingOccurrencesOfString:@"<" withString:@"&lt;"]
              stringByReplacingOccurrencesOfString:@">" withString:@"&gt;"]
              stringByReplacingOccurrencesOfString:@"\"" withString:@"&quot;"];
}

@interface YJDelegate : NSObject <NSApplicationDelegate, WKUIDelegate, WKNavigationDelegate>
@property (nonatomic, strong) NSWindow *window;
@property (nonatomic, strong) WKWebView *web;
@property (nonatomic, strong) NSTextField *status;
@property (nonatomic, strong) NSButton *startButton;
@property (nonatomic, copy) NSString *project;
@property (nonatomic) BOOL working;
@property (nonatomic) BOOL stoppedByUser;
@property (nonatomic) BOOL openedPage;
@end

@implementation YJDelegate

- (NSTextField *)label:(NSString *)text size:(CGFloat)size color:(NSColor *)color {
    NSTextField *l = [NSTextField labelWithString:text];
    l.font = [NSFont systemFontOfSize:size weight:(size > 15 ? NSFontWeightSemibold : NSFontWeightRegular)];
    l.textColor = color;
    l.lineBreakMode = NSLineBreakByTruncatingTail;
    return l;
}

- (NSButton *)button:(NSString *)title selector:(SEL)selector {
    NSButton *b = [NSButton buttonWithTitle:title target:self action:selector];
    b.bezelStyle = NSBezelStyleRounded;
    b.controlSize = NSControlSizeRegular;
    b.translatesAutoresizingMaskIntoConstraints = NO;
    return b;
}

- (BOOL)validProject:(NSString *)path {
    if (!path.length) return NO;
    NSFileManager *fm = [NSFileManager defaultManager];
    NSString *runtime = [[path stringByDeletingLastPathComponent]
                          stringByAppendingPathComponent:@"runtime/YujiaService/YujiaService"];
    return [fm isExecutableFileAtPath:runtime] &&
        [fm fileExistsAtPath:[path stringByAppendingPathComponent:@"install_launchd.sh"]] &&
        [fm fileExistsAtPath:[path stringByAppendingPathComponent:@"uninstall_launchd.sh"]] &&
        [fm fileExistsAtPath:[path stringByAppendingPathComponent:@"migrate_data.sh"]];
}

- (NSString *)dataDirectory {
    return [NSHomeDirectory() stringByAppendingPathComponent:@"Library/Application Support/yujia-signin"];
}

- (void)showAlert:(NSString *)title message:(NSString *)message {
    NSAlert *a = [[NSAlert alloc] init];
    a.messageText = title;
    a.informativeText = message ?: @"";
    [a addButtonWithTitle:@"知道了"];
    [a runModal];
}

- (void)setIndicator:(NSString *)s {
    self.status.stringValue = s ?: @"";
}

- (void)applicationDidFinishLaunching:(NSNotification *)notification {
    [NSApp setActivationPolicy:NSApplicationActivationPolicyRegular];
    NSString *resIcon = [[[NSBundle mainBundle] resourcePath] stringByAppendingPathComponent:@"AppIcon.png"];
    if (![[NSFileManager defaultManager] fileExistsAtPath:resIcon]) {
        NSString *override = [[[NSProcessInfo processInfo] environment] objectForKey:@"YUJIA_APP_ICON"];
        resIcon = override ?: resIcon;
    }
    NSImage *appIcon = [[NSImage alloc] initWithContentsOfFile:resIcon];
    if (appIcon) [NSApp setApplicationIconImage:appIcon];

    NSMenu *mainMenu = [[NSMenu alloc] init];
    NSMenuItem *appItem = [[NSMenuItem alloc] init];
    [mainMenu addItem:appItem];
    NSMenu *sub = [[NSMenu alloc] initWithTitle:@"寓+签到"];
    [sub addItemWithTitle:@"启动或修复后台" action:@selector(startAction:) keyEquivalent:@"r"];
    [sub addItemWithTitle:@"导入旧版账号和记录…" action:@selector(importLegacyAction:) keyEquivalent:@""];
    [sub addItem:[NSMenuItem separatorItem]];
    [sub addItemWithTitle:@"退出寓+签到（后台继续运行）" action:@selector(terminate:) keyEquivalent:@"q"];
    appItem.submenu = sub;
    [NSApp setMainMenu:mainMenu];

    self.window = [[NSWindow alloc] initWithContentRect:NSMakeRect(0, 0, 1200, 800)
                      styleMask:(NSWindowStyleMaskTitled | NSWindowStyleMaskClosable | NSWindowStyleMaskMiniaturizable | NSWindowStyleMaskResizable)
                      backing:NSBackingStoreBuffered defer:NO];
    self.window.title = @"寓+签到 · 管理中心";
    // Match the lightweight Aqua dashboard even when system appearance is dark.
    self.window.appearance = [NSAppearance appearanceNamed:NSAppearanceNameAqua];
    self.window.minSize = NSMakeSize(850, 570);
    [self.window center];

    NSView *root = self.window.contentView;
    root.wantsLayer = YES;
    root.layer.backgroundColor = [NSColor colorWithCalibratedRed:0.97 green:0.97 blue:0.98 alpha:1.0].CGColor;

    NSVisualEffectView *top = [[NSVisualEffectView alloc] init];
    top.translatesAutoresizingMaskIntoConstraints = NO;
    top.material = NSVisualEffectMaterialHeaderView;
    top.blendingMode = NSVisualEffectBlendingModeWithinWindow;
    top.state = NSVisualEffectStateActive;
    [root addSubview:top];

    NSTextField *heading = [self label:@"寓+签到" size:17 color:NSColor.labelColor];
    heading.translatesAutoresizingMaskIntoConstraints = NO;
    [top addSubview:heading];

    self.status = [self label:@"正在检查后台…" size:12 color:NSColor.secondaryLabelColor];
    self.status.translatesAutoresizingMaskIntoConstraints = NO;
    [top addSubview:self.status];

    self.startButton = [self button:@"启动 / 修复" selector:@selector(startAction:)];
    NSButton *logs = [self button:@"查看日志" selector:@selector(logsAction:)];
    NSButton *refresh = [self button:@"刷新" selector:@selector(refreshAction:)];
    NSButton *stop = [self button:@"停止后台" selector:@selector(stopAction:)];
    [top addSubview:self.startButton]; [top addSubview:logs]; [top addSubview:refresh]; [top addSubview:stop];

    WKWebViewConfiguration *cfg = [[WKWebViewConfiguration alloc] init];
    cfg.websiteDataStore = [WKWebsiteDataStore nonPersistentDataStore];
    self.web = [[WKWebView alloc] initWithFrame:NSZeroRect configuration:cfg];
    self.web.translatesAutoresizingMaskIntoConstraints = NO;
    self.web.UIDelegate = self;
    self.web.navigationDelegate = self;
    [root addSubview:self.web];

    [NSLayoutConstraint activateConstraints:@[
        [top.topAnchor constraintEqualToAnchor:root.topAnchor],
        [top.leadingAnchor constraintEqualToAnchor:root.leadingAnchor],
        [top.trailingAnchor constraintEqualToAnchor:root.trailingAnchor],
        [top.heightAnchor constraintEqualToConstant:60],
        [heading.leadingAnchor constraintEqualToAnchor:top.leadingAnchor constant:20],
        [heading.centerYAnchor constraintEqualToAnchor:top.centerYAnchor constant:-9],
        [self.status.leadingAnchor constraintEqualToAnchor:heading.leadingAnchor],
        [self.status.centerYAnchor constraintEqualToAnchor:top.centerYAnchor constant:13],
        [self.status.trailingAnchor constraintLessThanOrEqualToAnchor:self.startButton.leadingAnchor constant:-12],
        [stop.trailingAnchor constraintEqualToAnchor:top.trailingAnchor constant:-15],
        [stop.centerYAnchor constraintEqualToAnchor:top.centerYAnchor],
        [refresh.trailingAnchor constraintEqualToAnchor:stop.leadingAnchor constant:-7],
        [refresh.centerYAnchor constraintEqualToAnchor:top.centerYAnchor],
        [logs.trailingAnchor constraintEqualToAnchor:refresh.leadingAnchor constant:-7],
        [logs.centerYAnchor constraintEqualToAnchor:top.centerYAnchor],
        [self.startButton.trailingAnchor constraintEqualToAnchor:logs.leadingAnchor constant:-7],
        [self.startButton.centerYAnchor constraintEqualToAnchor:top.centerYAnchor],
        [self.web.topAnchor constraintEqualToAnchor:top.bottomAnchor],
        [self.web.bottomAnchor constraintEqualToAnchor:root.bottomAnchor],
        [self.web.leadingAnchor constraintEqualToAnchor:root.leadingAnchor],
        [self.web.trailingAnchor constraintEqualToAnchor:root.trailingAnchor]
    ]];
    [self.window makeKeyAndOrderFront:nil];
    [NSApp activateIgnoringOtherApps:YES];

    self.project = [[[NSBundle mainBundle] resourcePath] stringByAppendingPathComponent:@"backend"];
    if (![self validProject:self.project]) {
        [self displayOffline:@"应用文件不完整" detail:@"找不到内置签到运行环境，请重新下载完整 App。无需安装 Python 或单独的项目文件夹。"];
        return;
    }
    [self displayOffline:@"正在启动本地后台" detail:@"自动检查并修复 launchd 后台服务，断网不会影响本地页面的启动。"];
    [self ensureStarted];
}

- (BOOL)applicationShouldTerminateAfterLastWindowClosed:(NSApplication *)sender { return YES; }

- (NSString *)runExecutable:(NSString *)executable args:(NSArray<NSString *> *)args path:(NSString *)cwd exit:(int *)exitCode {
    NSTask *t = [[NSTask alloc] init];
    t.launchPath = executable;
    t.arguments = args;
    if (cwd.length) t.currentDirectoryPath = cwd;
    NSPipe *p = [NSPipe pipe];
    t.standardOutput = p;
    t.standardError = p;
    @try {
        [t launch];
        NSData *d = [[p fileHandleForReading] readDataToEndOfFile];
        [t waitUntilExit];
        if (exitCode) *exitCode = t.terminationStatus;
        NSString *s = [[NSString alloc] initWithData:d encoding:NSUTF8StringEncoding];
        return s ?: @"";
    } @catch (NSException *e) {
        if (exitCode) *exitCode = 127;
        return e.reason ?: @"无法运行命令";
    }
}

- (BOOL)plistMatches:(NSString *)name script:(NSString *)unused {
    NSString *plist = [NSHomeDirectory() stringByAppendingPathComponent:[NSString stringWithFormat:@"Library/LaunchAgents/%@.plist", name]];
    NSDictionary *data = [NSDictionary dictionaryWithContentsOfFile:plist];
    NSArray *args = data[@"ProgramArguments"];
    NSString *runtime = [[[self.project stringByDeletingLastPathComponent]
                          stringByAppendingPathComponent:@"runtime/YujiaService/YujiaService"] stringByStandardizingPath];
    NSString *mode = [name isEqualToString:@"com.yujia.signin"] ? @"--if-due" : @"--panel";
    return [args isKindOfClass:[NSArray class]] && args.count >= 2 &&
           [args[0] isEqualToString:runtime] && [args[1] isEqualToString:mode];
}

- (BOOL)healthyBackend {
    int rc = 0;
    NSString *result = [self runExecutable:@"/usr/bin/curl"
                   args:@[@"--noproxy", @"*", @"--silent", @"--show-error", @"--fail", @"--connect-timeout", @"1", @"--max-time", @"2", @"http://127.0.0.1:8765/api/health"] path:nil exit:&rc];
    return rc == 0 && [result containsString:@"yujia-panel"];
}

- (BOOL)serviceReady {
    if (![self validProject:self.project]) return NO;
    if (![self plistMatches:@"com.yujia.signin" script:@"run_daily.py"] ||
        ![self plistMatches:@"com.yujia.signinweb" script:@"web_panel.py"]) return NO;
    int a = -1, b = -1;
    [self runExecutable:@"/bin/launchctl" args:@[@"list", @"com.yujia.signin"] path:nil exit:&a];
    [self runExecutable:@"/bin/launchctl" args:@[@"list", @"com.yujia.signinweb"] path:nil exit:&b];
    return a == 0 && b == 0 && [self healthyBackend];
}

- (void)ensureStarted {
    if (self.working || self.stoppedByUser) return;
    self.working = YES;
    self.startButton.enabled = NO;
    [self setIndicator:@"正在检查后台运行状态…"];
    dispatch_async(dispatch_get_global_queue(QOS_CLASS_USER_INITIATED, 0), ^{
        BOOL ready = [self serviceReady];
        dispatch_async(dispatch_get_main_queue(), ^{
            self.working = NO;
            self.startButton.enabled = YES;
            if (ready) [self showPanel];
            else [self runServiceScript:@"install_launchd.sh" isStop:NO];
        });
    });
}

- (void)runServiceScript:(NSString *)script isStop:(BOOL)stop {
    if (self.working) return;
    if (![self validProject:self.project]) {
        [self showAlert:@"应用文件损坏" message:@"缺少打包运行程序。请重新安装完整的 App。"];
        return;
    }
    self.working = YES;
    self.startButton.enabled = NO;
    [self setIndicator:(stop ? @"正在停止后台…" : @"正在启动 / 修复后台…")];
    [self displayOffline:(stop ? @"正在停止后台" : @"正在启动后台服务") detail:@"正在处理 launchd 服务，不需要打开终端。"];
    NSString *scriptName = [self.project stringByAppendingPathComponent:script];
    dispatch_async(dispatch_get_global_queue(QOS_CLASS_USER_INITIATED, 0), ^{
        int rc = -1;
        NSString *output = [self runExecutable:@"/bin/bash" args:@[scriptName] path:self.project exit:&rc];
        dispatch_async(dispatch_get_main_queue(), ^{
            self.working = NO;
            self.startButton.enabled = YES;
            if (rc != 0) {
                [self setIndicator:@"后台处理失败"];
                NSString *error = output.length ? output : [NSString stringWithFormat:@"退出码 %d", rc];
                [self displayOffline:@"后台服务启动失败" detail:[NSString stringWithFormat:@"可以点击「启动 / 修复」重试。\n\n%@", error]];
            } else if (stop) {
                self.stoppedByUser = YES;
                self.openedPage = NO;
                [self setIndicator:@"后台已停止 · 数据保留"];
                [self displayOffline:@"后台已停止" detail:@"账号、签到计划及历史仍然保留。点击顶部「启动 / 修复」即可重新开启。"];
            } else {
                [self setIndicator:@"后台已加载，正在等待面板响应…"];
                [self pollBackend:0];
            }
        });
    });
}

- (void)pollBackend:(NSInteger)tryCount {
    dispatch_async(dispatch_get_global_queue(QOS_CLASS_USER_INITIATED, 0), ^{
        BOOL ready = [self healthyBackend];
        dispatch_async(dispatch_get_main_queue(), ^{
            if (self.stoppedByUser) return;
            if (ready) {
                [self showPanel];
            } else if (tryCount < 10) {
                [self performSelector:@selector(pollFromTimer:) withObject:@(tryCount + 1) afterDelay:1.0];
            } else {
                [self setIndicator:@"后台已加载，但管理面板不可用"];
                [self displayOffline:@"后台尚未就绪" detail:@"面板端口可能被占用，或者内置服务启动失败。可点击「查看日志」排查，或选择「启动 / 修复」。"];
            }
        });
    });
}
- (void)pollFromTimer:(NSNumber *)n { [self pollBackend:n.integerValue]; }

- (void)showPanel {
    if (!self.project) return;
    [self setIndicator:@"● 后台正常 · 关闭窗口不影响签到"];
    if (!self.openedPage) {
        self.openedPage = YES;
        NSURL *url = [NSURL URLWithString:@"http://127.0.0.1:8765/"];
        [self.web loadRequest:[NSURLRequest requestWithURL:url cachePolicy:NSURLRequestReloadIgnoringLocalCacheData timeoutInterval:5]];
    }
}

- (void)displayOffline:(NSString *)title detail:(NSString *)detail {
    self.openedPage = NO;
    NSString *html = [NSString stringWithFormat:
        @"<!doctype html><html lang='zh'><meta charset='utf-8'><meta name='viewport' content='width=device-width,initial-scale=1'><style>body{margin:0;background:#0b1424;color:#e7eef9;font:14px/1.7 -apple-system,'PingFang SC',sans-serif;display:grid;place-items:center;min-height:100vh}.box{width:min(620px,85%%);background:linear-gradient(145deg,#16263e,#111b2d);border:1px solid #334960;border-radius:22px;padding:42px;box-shadow:0 24px 90px #0005}.mark{display:grid;place-items:center;width:58px;height:58px;border-radius:18px;background:linear-gradient(130deg,#7be4ff,#989fff);color:#092035;font-size:28px;font-weight:800}h2{font-size:23px;margin:18px 0 8px}p{color:#adbed0;white-space:pre-wrap;overflow-wrap:anywhere}.hint{border-top:1px solid #334960;margin-top:24px;padding-top:14px;color:#8296af;font-size:12px}</style><div class='box'><div class='mark'>寓+</div><h2>%@</h2><p>%@</p><div class='hint'>后台独立于窗口运行，重新登录 macOS 后也会自动恢复。</div></div></html>",
        YJEscape(title), YJEscape(detail)];
    [self.web loadHTMLString:html baseURL:nil];
}

- (void)startAction:(id)sender {
    self.stoppedByUser = NO;
    [self runServiceScript:@"install_launchd.sh" isStop:NO];
}
- (void)stopAction:(id)sender {
    NSAlert *a = [[NSAlert alloc] init];
    a.messageText = @"停止后台签到？";
    a.informativeText = @"将停止定时任务及管理面板，但不会删除账号、Token 或签到记录。之后可一键重新启动。";
    [a addButtonWithTitle:@"停止后台"];
    [a addButtonWithTitle:@"取消"];
    if ([a runModal] == NSAlertFirstButtonReturn) [self runServiceScript:@"uninstall_launchd.sh" isStop:YES];
}
- (void)refreshAction:(id)sender {
    if (self.working) return;
    if (self.stoppedByUser) {
        [self displayOffline:@"后台已停止" detail:@"点击「启动 / 修复」重新启动。"];
    } else {
        self.openedPage = NO;
        [self ensureStarted];
    }
}
- (void)logsAction:(id)sender {
    if (self.openedPage) {
        [self.web evaluateJavaScript:@"document.getElementById('logs')?.scrollIntoView({behavior:'smooth',block:'center'}); if(typeof loadLogs==='function')loadLogs();"
                     completionHandler:nil];
    } else {
        NSString *p = [[self dataDirectory] stringByAppendingPathComponent:@"web.log"];
        if (![[NSFileManager defaultManager] fileExistsAtPath:p]) p = [[self dataDirectory] stringByAppendingPathComponent:@"signin.log"];
        if ([[NSFileManager defaultManager] fileExistsAtPath:p]) [[NSWorkspace sharedWorkspace] openFile:p];
        else [self showAlert:@"暂无日志" message:@"还没有运行日志。请尝试点击启动 / 修复查看错误信息。"];
    }
}
- (void)importLegacyAction:(id)sender {
    if (self.working) return;
    NSOpenPanel *panel = [NSOpenPanel openPanel];
    panel.title = @"导入旧版寓+签到数据";
    panel.message = @"仅首次迁移时需要：请选择旧目录（包含 config.json 或 records.db）。新电脑可直接在 App 内添加账号。";
    panel.canChooseFiles = NO;
    panel.canChooseDirectories = YES;
    panel.allowsMultipleSelection = NO;
    if ([panel runModal] != NSModalResponseOK) return;
    NSString *oldDir = panel.URL.path.stringByStandardizingPath;
    NSFileManager *fm = [NSFileManager defaultManager];
    if (![fm fileExistsAtPath:[oldDir stringByAppendingPathComponent:@"config.json"]] &&
        ![fm fileExistsAtPath:[oldDir stringByAppendingPathComponent:@"records.db"]]) {
        [self showAlert:@"找不到旧数据" message:@"请选择包含 config.json 或 records.db 的旧项目文件夹。"]; return;
    }
    NSAlert *ask = [[NSAlert alloc] init];
    ask.messageText = @"从旧版导入账号与历史？";
    ask.informativeText = @"导入会先停止现有签到后台，自动备份已有数据并替换。原目录不会被删除；导入后会重新启动服务。请先退出旧版调度器，以免同时执行签到。";
    [ask addButtonWithTitle:@"导入"]; [ask addButtonWithTitle:@"取消"];
    if ([ask runModal] != NSAlertFirstButtonReturn) return;
    self.working = YES;
    self.startButton.enabled = NO;
    [self setIndicator:@"正在导入旧版数据…"];
    NSString *scriptPath = [self.project stringByAppendingPathComponent:@"migrate_data.sh"];
    dispatch_async(dispatch_get_global_queue(QOS_CLASS_USER_INITIATED, 0), ^{
        int rc = -1;
        NSString *out = [self runExecutable:@"/bin/bash" args:@[scriptPath, oldDir] path:self.project exit:&rc];
        dispatch_async(dispatch_get_main_queue(), ^{
            self.working = NO;
            self.startButton.enabled = YES;
            if (rc != 0) {
                [self showAlert:@"数据导入失败" message:out.length ? out : @"请检查旧项目目录的文件权限。"];
            } else {
                self.stoppedByUser = NO;
                self.openedPage = NO;
                [self showAlert:@"导入完成" message:@"账号和历史已导入，原始数据仍在旧目录中。请不要同时运行旧版本服务。"];
                [self ensureStarted];
            }
        });
    });
}

// WKWebView 不会自动显示 JavaScript alert / confirm / prompt。实现这三个接口以确保编辑及删除账号可用。
- (void)webView:(WKWebView *)webView runJavaScriptAlertPanelWithMessage:(NSString *)message
 initiatedByFrame:(WKFrameInfo *)frame completionHandler:(void (^)(void))completionHandler {
    [self showAlert:@"寓+签到" message:message];
    completionHandler();
}
- (void)webView:(WKWebView *)webView runJavaScriptConfirmPanelWithMessage:(NSString *)message
 initiatedByFrame:(WKFrameInfo *)frame completionHandler:(void (^)(BOOL))completionHandler {
    NSAlert *a = [[NSAlert alloc] init]; a.messageText = @"请确认"; a.informativeText = message;
    [a addButtonWithTitle:@"确定"]; [a addButtonWithTitle:@"取消"];
    completionHandler([a runModal] == NSAlertFirstButtonReturn);
}
- (void)webView:(WKWebView *)webView runJavaScriptTextInputPanelWithPrompt:(NSString *)prompt
 defaultText:(NSString *)defaultText initiatedByFrame:(WKFrameInfo *)frame
 completionHandler:(void (^)(NSString * _Nullable))completionHandler {
    NSAlert *a = [[NSAlert alloc] init]; a.messageText = prompt;
    [a addButtonWithTitle:@"确定"]; [a addButtonWithTitle:@"取消"];
    NSTextField *f = [[NSTextField alloc] initWithFrame:NSMakeRect(0,0,370,26)];
    f.stringValue = defaultText ?: @""; a.accessoryView = f;
    completionHandler([a runModal] == NSAlertFirstButtonReturn ? f.stringValue : nil);
}

- (void)webView:(WKWebView *)webView decidePolicyForNavigationAction:(WKNavigationAction *)action
 decisionHandler:(void (^)(WKNavigationActionPolicy))decisionHandler {
    NSURL *u = action.request.URL;
    if (!u || [u.scheme isEqualToString:@"about"]) { decisionHandler(WKNavigationActionPolicyAllow); return; }
    if ([u.host isEqualToString:@"127.0.0.1"] && u.port.integerValue == 8765) {
        if ([u.path isEqualToString:@"/api/export"]) {
            decisionHandler(WKNavigationActionPolicyCancel);
            [self exportCSV];
            return;
        }
        decisionHandler(WKNavigationActionPolicyAllow); return;
    }
    decisionHandler(WKNavigationActionPolicyCancel);
    if ([u.scheme isEqualToString:@"http"] || [u.scheme isEqualToString:@"https"]) [[NSWorkspace sharedWorkspace] openURL:u];
}

- (void)exportCSV {
    NSSavePanel *p = [NSSavePanel savePanel];
    p.nameFieldStringValue = @"yujia-history.csv";
    [p beginSheetModalForWindow:self.window completionHandler:^(NSModalResponse result) {
        if (result != NSModalResponseOK) return;
        NSURL *dest = p.URL;
        NSURL *url = [NSURL URLWithString:@"http://127.0.0.1:8765/api/export"];
        NSURLSessionDataTask *task = [[NSURLSession sharedSession] dataTaskWithURL:url
          completionHandler:^(NSData *data, NSURLResponse *response, NSError *error) {
            BOOL valid = !error && [response isKindOfClass:[NSHTTPURLResponse class]] &&
                [(NSHTTPURLResponse *)response statusCode] == 200 && data.length > 0;
            NSError *saveError = nil;
            BOOL ok = valid && [data writeToURL:dest options:NSDataWritingAtomic error:&saveError];
            if (!ok) dispatch_async(dispatch_get_main_queue(), ^{
                [self showAlert:@"导出失败" message:(error ?: saveError).localizedDescription ?: @"面板未正常响应，请稍后重试。"];
            });
          }];
        [task resume];
    }];
}

- (void)webView:(WKWebView *)webView didFailProvisionalNavigation:(WKNavigation *)navigation withError:(NSError *)error {
    [self setIndicator:@"面板连接断开 · 点击刷新"];
    [self displayOffline:@"暂时无法连接后台" detail:@"后台可能正在重新启动。点击顶部「刷新」即可检查或自动修复。"];
}
@end

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        NSApplication *app = [NSApplication sharedApplication];
        YJDelegate *d = [[YJDelegate alloc] init];
        app.delegate = d;
        [app run];
    }
    return 0;
}
