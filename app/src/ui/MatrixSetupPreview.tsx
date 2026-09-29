export function MatrixSetupPreview({ rows, columns }: { rows: number; columns: number }) {
  if (!Number.isInteger(rows) || !Number.isInteger(columns) || rows < 1 || columns < 1 || rows * columns > 4096) return null;
  return <figure className="wb-matrix-preview">
    <svg viewBox={`-1 -1 ${columns * 10 + 1} ${rows * 10 + 1}`} role="img" aria-label={`${rows} rows by ${columns} columns matrix preview`}>
      {Array.from({ length: rows * columns }, (_, index) => <rect key={index} x={(index % columns) * 10} y={Math.floor(index / columns) * 10} width="8" height="8" rx="1"/>)}
    </svg><figcaption>{rows * columns} keys · {rows} × {columns}</figcaption>
  </figure>;
}
