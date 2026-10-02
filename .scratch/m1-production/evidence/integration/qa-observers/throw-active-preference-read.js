// Test-only failure injection for the user-visible localStorage error path.
// This intentionally fails only the M1 active-project preference getItem call;
// writes and every other browser storage operation remain real.
(() => {
  const original = Storage.prototype.getItem;
  Storage.prototype.getItem = function (key) {
    if (typeof key === 'string' && key.startsWith('boardstudio-m1-active-project:boardstudio-m1-')) {
      throw new DOMException('QA injected active-project preference read error', 'SecurityError');
    }
    return original.call(this, key);
  };
})();
