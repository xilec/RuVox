// KWin script: raise and focus the RuVox window.
// Matches by caption OR resourceClass so we are robust to Tauri/KWin
// combinations where one of those is empty/deferred.
var wins = typeof workspace.windowList === 'function'
    ? workspace.windowList()
    : workspace.windows;
for (var i = 0; i < wins.length; i++) {
    var w = wins[i];
    var caption = w.caption || '';
    var resourceClass = w.resourceClass || '';
    if (caption.indexOf('RuVox') !== -1
        || resourceClass.toLowerCase().indexOf('ruvox') !== -1) {
        try { w.minimized = false; } catch (e) {}
        workspace.activeWindow = w;
        workspace.raiseWindow(w);
        break;
    }
}
