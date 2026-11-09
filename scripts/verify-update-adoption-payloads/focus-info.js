(() => {
  const a = document.activeElement;
  const dialog = document.querySelector('[role=alertdialog]');
  return {
    tag: a ? a.tagName : null,
    label: a ? (a.getAttribute('aria-label') || (a.textContent || '').trim()).slice(0, 60) : null,
    inDialog: !!(a && a.closest('[role=alertdialog]')),
    inToast: !!(a && a.closest('[data-slot=toast-provider]')),
    dialogOpen: !!dialog,
    dialogText: dialog ? (dialog.textContent || '').slice(0, 400) : null,
  };
})()
