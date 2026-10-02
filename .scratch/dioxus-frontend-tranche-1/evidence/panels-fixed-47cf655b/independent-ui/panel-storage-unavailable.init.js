// Test-only browser storage-provider fault fixture for the isolated verifier session.
// Only the optional panel preference namespace throws. Project/board persistence remains untouched.
(() => {
  const prefix = 'boardstudio:v2:panel:';
  window.__panelStorageFaultCalls = { get: 0, set: 0 };
  const originalGet = Storage.prototype.getItem;
  const originalSet = Storage.prototype.setItem;
  Storage.prototype.getItem = function (key) {
    if (String(key).startsWith(prefix)) {
      window.__panelStorageFaultCalls.get += 1;
      throw new DOMException('Test fixture: optional panel storage unavailable', 'SecurityError');
    }
    return originalGet.call(this, key);
  };
  Storage.prototype.setItem = function (key, value) {
    if (String(key).startsWith(prefix)) {
      window.__panelStorageFaultCalls.set += 1;
      throw new DOMException('Test fixture: optional panel storage unavailable', 'SecurityError');
    }
    return originalSet.call(this, key, value);
  };
})();
