(() => {
  const notices = Array.from(document.querySelectorAll('[role=status]')).filter(e => (e.textContent||'').includes('Update Available'));
  return {
    loadMark: window.__loadMark || null,
    pcbActive: !!document.querySelector('[role=region][aria-label="PCB workspace"]'),
    updateNoticeCount: notices.length,
  };
})()
