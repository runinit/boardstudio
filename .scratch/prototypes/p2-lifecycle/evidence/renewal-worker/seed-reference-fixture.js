(async () => {
  const response = await fetch("http://127.0.0.1:4396/reviung41.json");
  if (!response.ok) throw new Error(`fixture fetch failed: ${response.status}`);
  const bytes = await response.arrayBuffer();
  const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", bytes));
  const fixtureSha256 = [...digest].map((byte) => byte.toString(16).padStart(2, "0")).join("");
  const document = JSON.parse(new TextDecoder().decode(bytes));
  await new Promise((resolve, reject) => {
    const request = indexedDB.open("boardstudio-v2", 1);
    request.onerror = () => reject(request.error);
    request.onsuccess = () => {
      const database = request.result;
      const transaction = database.transaction(["projects", "assets"], "readwrite");
      transaction.objectStore("projects").put(document);
      transaction.oncomplete = () => {
        localStorage.setItem("boardstudio-v2-active-project", document.id);
        database.close();
        resolve();
      };
      transaction.onerror = () => reject(transaction.error);
      transaction.onabort = () => reject(transaction.error);
    };
  });
  return JSON.stringify({
    origin: location.origin,
    fixtureSha256,
    id: document.id,
    name: document.name,
    revision: document.revision,
    parts: document.parts.length,
    matrices: document.matrices.map(({ id }) => id),
  });
})()
