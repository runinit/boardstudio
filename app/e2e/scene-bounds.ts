import type { Page } from '@playwright/test';

/** Read the camera used by the last draw, rather than reconstructing Fit from worker replies. */
export async function sceneProjection(page: Page) {
  return page.locator('.wb-assembly-scene canvas').evaluate(element => {
    const canvas = element as HTMLCanvasElement;
    const gl = canvas.getContext('webgl2');
    if (!gl) throw new Error('The assembly canvas has no WebGL2 context');
    const program = (gl as any).__lastSceneProgram as WebGLProgram | undefined;
    const location = program && gl.getUniformLocation(program, 'viewProjection');
    if (!program || !location) throw new Error('The last assembly draw has no camera matrix');
    const matrix = Array.from(gl.getUniform(program, location) as Float32Array);
    if (matrix.length !== 16 || matrix.some(value => !Number.isFinite(value))) throw new Error('Invalid assembly camera matrix');
    const { x, y, width, height } = canvas.getBoundingClientRect();
    return { matrix, x, y, width, height };
  });
}

export function projectScenePoint(view: Awaited<ReturnType<typeof sceneProjection>>, x: number, y: number, z: number) {
  const m = view.matrix;
  const w = m[3] * x + m[7] * y + m[11] * z + m[15];
  if (w <= 0) throw new Error('The requested gesture is behind the camera');
  return {
    x: view.x + (1 + (m[0] * x + m[4] * y + m[8] * z + m[12]) / w) * view.width / 2,
    y: view.y + (1 - (m[1] * x + m[5] * y + m[9] * z + m[13]) / w) * view.height / 2,
  };
}

/** Bounds of the retained, visible geometry used by Fit in these all-layers-on fixtures. */
export async function auditSceneBounds(page: Page) {
  await page.addInitScript(() => {
    if ((window as any).__sceneBoundsAudit) return;
    (window as any).__sceneBoundsAudit = true;
    // three-d unbinds the program after drawing. Retain only the last program
    // reference so setup can read its camera without sampling every uniform.
    const useProgram = WebGL2RenderingContext.prototype.useProgram;
    WebGL2RenderingContext.prototype.useProgram = function(program) {
      if (program) (this as any).__lastSceneProgram = program;
      return useProgram.call(this, program);
    };
    const post = Worker.prototype.postMessage, observed = new WeakSet<Worker>();
    Worker.prototype.postMessage = function(message: any, ...rest: any[]) {
      if (!observed.has(this)) {
        observed.add(this);
        // Renderer request IDs restart when a project creates a new worker.
        let objects: any[] = [], revision = -1;
        this.addEventListener('message', event => {
          const packet = event.data;
          if (!packet.prepared || packet.id < revision) return;
          revision = packet.id;
          const changed = packet.prepared.objects;
          if (packet.patch) {
            const replaced = new Set([...changed.map((object: any) => object.id), ...(packet.prepared.removed ?? [])]);
            objects = [...objects.filter(object => !replaced.has(object.id)), ...changed];
          } else objects = changed;
          const low = [Infinity, Infinity, Infinity], high = [-Infinity, -Infinity, -Infinity];
          for (const object of objects) {
            if (object.id === 'pcb-selection') continue;
            const points = object.mesh.positions;
            for (let i = 0; i < points.length; i += 3) for (let axis = 0; axis < 3; axis++) {
              low[axis] = Math.min(low[axis], points[i + axis]);
              high[axis] = Math.max(high[axis], points[i + axis]);
            }
          }
          (window as any).__fitBounds = [...low.map((value, axis) => (value + high[axis]) / 2),
            Math.hypot(...low.map((value, axis) => high[axis] - value)) / 2];
        });
      }
      return post.call(this, message, ...rest);
    };
  });
}
