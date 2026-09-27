import { describe, expect, it, vi } from 'vitest';
import { emptyProject, type MechanicalExtraction, type MechanicalPartProfile } from '@boardstudio/v2-contracts';
import { createMechanicalConfiguration } from '../mechanicalPresets';
import { MechanicalProfileController, applyMechanicalExtraction } from './mechanicalProfileController';

const profile: MechanicalPartProfile = { definitionId: 'switch', source: 'MX', cutouts: [], plateToPcb: 3.5 };
const configuration = () => createMechanicalConfiguration(emptyProject('project', 'Project'), 'board');
function deferred() {
  let resolve!: (value: MechanicalPartProfile) => void;
  let reject!: (error: Error) => void;
  const promise = new Promise<MechanicalPartProfile>((done, fail) => { resolve = done; reject = fail; });
  return { promise, resolve, reject };
}
function setup() {
  const controller = new MechanicalProfileController();
  const config = configuration();
  controller.receive(config, 'project:board:1');
  const feedback = { setPending: vi.fn(), setError: vi.fn(), update: vi.fn() };
  return { controller, config, feedback };
}

describe('mechanical profile controller', () => {
  it('loads family dimensions and assigns once without changing other configuration', async () => {
    const { controller, config, feedback } = setup();
    const load = vi.fn(async () => profile);
    await controller.assign('switch', 'choc-v1-switch', load, feedback);
    expect(load).toHaveBeenCalledWith('switch', 'choc-v1-switch', 2.2);
    expect(feedback.update).toHaveBeenCalledWith({ plateThickness: 1.3, plateToPcb: 2.2, plateFoamThickness: 2, profiles: [{ ...profile, switchFamily: 'choc-v1', plateToPcb: 2.2 }] });
    expect(config.profiles).toEqual([]);
    controller.receive({ ...config, profiles: [profile] }, 'project:board:1');
    await controller.assign('switch', 'mx-switch', load, feedback);
    expect(load).toHaveBeenCalledTimes(1);
    expect(feedback.setPending.mock.calls).toEqual([[true], [false]]);
  });

  it.each(['project:other-board:1', 'other-project:board:1', 'project:board:2'])('discards a profile after switching to %s', async scope => {
    const { controller, config, feedback } = setup();
    const pending = deferred();
    const request = controller.assign('switch', 'mx-switch', () => pending.promise, feedback);
    controller.receive(config, scope);
    pending.resolve(profile);
    await request;
    expect(feedback.update).not.toHaveBeenCalled();
    expect(feedback.setPending).toHaveBeenLastCalledWith(false);
  });

  it('does not overwrite a newer stabilizer edit or an undone configuration', async () => {
    const { controller, config, feedback } = setup();
    const pending = deferred();
    const request = controller.assignStabilizer('key', 'switch', 'mx-stab2u', () => pending.promise, feedback);
    controller.receive({ ...config, stabilizers: [{ partId: 'key', kind: 'plate-mount', units: 6.25 }] }, 'project:board:1');
    pending.resolve(profile);
    await request;
    expect(feedback.update).not.toHaveBeenCalled();
  });

  it('preserves other stabilizers and settings when assigning a current result', async () => {
    const { controller, config, feedback } = setup();
    const first = { partId: 'first', kind: 'pcb-mount' as const, units: 2 };
    const second = { partId: 'second', kind: 'plate-mount' as const, units: 6.25, rotation: 90 };
    controller.receive({ ...config, stabilizers: [first, second] }, 'project:board:1');
    await controller.assignStabilizer('second', 'switch', 'mx-stab625u', async () => profile, feedback);
    expect(feedback.update).toHaveBeenCalledWith({ stabilizers: [first, { ...second, profile }] });
  });

  it('does not publish an old request error into another board or clear a newer busy state', async () => {
    const { controller, config, feedback } = setup();
    const first = deferred();
    const second = deferred();
    const oldRequest = controller.assign('switch', 'mx-switch', () => first.promise, feedback);
    controller.receive(config, 'project:other-board:1');
    const newRequest = controller.assign('switch', 'mx-switch', () => second.promise, feedback);
    first.reject(new Error('Old board profile unavailable'));
    await oldRequest;
    expect(feedback.setError.mock.calls).toEqual([[''], ['']]);
    expect(feedback.setPending.mock.calls).toEqual([[true], [true]]);
    second.resolve(profile);
    await newRequest;
    expect(feedback.update).toHaveBeenCalledTimes(1);
    expect(feedback.setPending).toHaveBeenLastCalledWith(false);
  });

  it('shows current errors and discards completion after the editor unmounts', async () => {
    const { controller, feedback } = setup();
    await controller.assign('switch', 'mx-switch', async () => { throw new Error('Unavailable'); }, feedback);
    expect(feedback.setError).toHaveBeenLastCalledWith('Unavailable');
    const pending = deferred();
    const request = controller.assign('switch', 'mx-switch', () => pending.promise, feedback);
    controller.invalidate();
    pending.resolve(profile);
    await request;
    expect(feedback.update).not.toHaveBeenCalled();
  });

  it('applies extracted contours and preserves unrelated mechanical data', () => {
    const extraction: MechanicalExtraction = { geometry: { primitives: [] }, sourceGeometry: { text: 'footprint', sha256: 'hash', mappings: [], sourceIds: ['p'] }, pcbHoles: [], clearanceEnvelopes: [], plateCutouts: [[{ x: 0, y: 0 }, { x: 1, y: 0 }, { x: 0, y: 1 }]] };
    const original = { ...profile, switchFamily: 'mx' as const, supportedThickness: { x: 1, y: 2 } };
    expect(applyMechanicalExtraction(original, extraction, 'Part')).toEqual({ ...original, source: 'KiCad Part', sourceGeometry: extraction.sourceGeometry, pcbHoles: [], clearances: [], cutouts: extraction.plateCutouts });
  });
});
