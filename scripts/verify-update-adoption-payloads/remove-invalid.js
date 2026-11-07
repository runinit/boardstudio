(() => {
  const field = document.getElementById('019-injected-invalid');
  if (field) field.remove();
  return { present: !!document.getElementById('019-injected-invalid') };
})()
