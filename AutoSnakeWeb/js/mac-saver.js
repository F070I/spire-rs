// macOS ScreenSaver wrapper bridge. Native host can call __AUTO_SNAKE_APPLY_SETTINGS__ after page load.
(function () {
  window.__WALLPAPER_ENGINE__ = true; // suppress browser fullscreen, click and wake-lock behavior
  window.__AUTO_SNAKE_PENDING_SETTINGS__ = {
    uiMode: 'full', gridDensity: 'fine', worldMode: 'wall', clockMode: 'minute',
    clockBrightness: 'subtle', theme: 'classic', speed: 'normal', fpsMode: 'auto', fpsCustom: '90'
  };
})();
