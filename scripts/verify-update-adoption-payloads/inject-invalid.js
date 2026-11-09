(() => {
  // Stand-in for a panel field whose Draft value the panel refuses to commit:
  // the shared field controls expose that state as aria-invalid, which is the
  // contract the Runtime gate reads. The mounted suite proves the same gate;
  // this payload proves it in the packaged build beside real input.
  let field = document.getElementById('019-injected-invalid');
  if (!field) {
    field = document.createElement('input');
    field.id = '019-injected-invalid';
    field.setAttribute('aria-label', 'Injected invalid draft');
    field.setAttribute('aria-invalid', 'true');
    field.value = 'not-a-number';
    field.style.cssText = 'position:fixed;left:-9999px;top:0;width:10px;height:10px';
    document.body.appendChild(field);
  }
  return { injected: !!document.getElementById('019-injected-invalid') };
})()
