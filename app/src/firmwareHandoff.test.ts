import { describe, expect, it } from 'vitest';
import { demoProject } from './demo';
import { firmwareRequest } from './firmwareHandoff';
import type { ElectricalPlan, ProjectDoc } from '@boardstudio/v2-contracts';

const plan = (mode: 'matrix' | 'direct' = 'matrix'): ElectricalPlan => ({ instanceId:null, jumpers:[], moduleAliases:{}, peripheralTerminals:{}, mode, assignments: [{ keyId: 'm/r0c0', matrixId: 'm', row: 0, column: 0, rowPin: 'P21', columnPin: 'P20', locked: false, rowFirmwareGpio: mode === 'matrix' ? 'P0.31' : null, columnFirmwareGpio: mode === 'matrix' ? 'P0.29' : null, directGpio: mode === 'direct' ? 'P0.29' : null }], rowPins: mode === 'matrix' ? ['P21'] : [], columnPins: mode === 'matrix' ? ['P20'] : [], diagnostics: [], fingerprint: 'x', boardId: 'b', controllerPartId: 'mcu', revision: 1, controllerProfile: 'ceoloide/mcu_nice_nano', freePins: [], nets: [], diodeDirection: 'col2row', peripherals: [], peripheralPins: {} });

describe('firmware handoff conversion', () => {
  it('creates compact matrix and direct requests', () => {
    const matrix = firmwareRequest(demoProject(), plan()).request;
    expect(matrix.keys).toEqual([{ id: 'm/r0c0', row: 0, column: 0 }]);
    expect(matrix.rows[0].gpio).toBe('P0.31');
    expect(firmwareRequest(demoProject(), plan('direct')).request.direct_pins[0].gpio).toBe('P0.29');
  });
  it('blocks errors and unknown controllers', () => {
    const invalid = plan(); invalid.controllerProfile = null;
    expect(() => firmwareRequest(demoProject(), invalid)).toThrow(/controller profile/);
    const blocked = plan(); blocked.diagnostics = [{ code: 'x', severity: 'error', message: 'bad', keyId: null }];
    expect(() => firmwareRequest(demoProject(), blocked)).toThrow('bad');
  });
  it('requires a distinct right-hand split plan', () => {
    const doc = demoProject(); doc.hardware = { topology: 'split', transport: 'wireless', boards: [], instances: [], sharedConstruction: null };
    expect(() => firmwareRequest(doc, plan())).toThrow(/distinct peripheral/);
    const right = plan(); right.boardId = 'right';
    expect(firmwareRequest(doc, plan(), right).request.peripheral?.board_name).toBe('right');
  });
it('blocks peripherals without a real firmware profile', () => {
    const value = plan(); value.peripherals = [{ partId: 'oled', source: 'ceoloide/display_ssd1306', kind: 'display-i2c', gpioTerminals: [['SDA', 'i2c/SDA']], fixedTerminals: [] }];
    expect(() => firmwareRequest(demoProject(), value)).toThrow(/display-i2c/);
  });
});

it('carries Rust connection findings into the firmware request for authoritative rejection', () => {
  const value = plan();
  value.diagnostics = [{ code: 'module/left/host-role', severity: 'error', message: 'Choose a host connector', keyId: 'left' }];
  const request = firmwareRequest(demoProject(), value).request;
  expect(request.hardware?.moduleFindings).toEqual([{ id: 'module/left/host-role', message: 'Choose a host connector' }]);
});

it('exports the configured rotary-only VIK carrier under its mounted module identity', () => {
  const document = demoProject();
  const rotaryProfile = { a: 'gpio1', b: 'gpio2', common: 'gnd', driver: 'ec11', steps: 24, triggersPerRotation: 4 };
  document.moduleDefinitions = [{
    id: 'vik-ec11', catalogueRow: 'ec11-evqwgd001', name: 'VIK EC11', family: 'pointing-input', variant: 'standard',
    source: { repository: 'https://github.com/sadekbaroudi/vik', revision: 'cd5d16e4cd9137a229fc673412a89d75f4e64553', path: 'pcb/ec11-evqwgd001/ec11-evqwgd001.kicad_pcb', license: 'CERN-OHL-S-2.0', sha256: '2b22c0f0b2b99206ac25029b8acd05c75146d9246cb8e3d9ceec5deacae4e033' },
    board: { contours: [], holes: [], thickness: 1.6 }, mounts: [], volumes: [], openings: [], models: [], candidateModels: [], gates: [], interfaces: [],
    electrical: { protocol: 'gpio', requiredSignals: ['gnd', 'gpio1', 'gpio2'], rotaryProfile }, constituents: [],
  } as NonNullable<ProjectDoc['moduleDefinitions']>[number]];
  document.modules = [{
    id: 'module/left-encoder', definitionId: 'vik-ec11', hostBoardId: 'b', hostInstanceId: 'left', hostFace: 'front', facingFace: 'back',
    at: { x: 0, y: 0 }, rotation: 0, gap: 0, attachment: 'board', detached: false, serviceClearance: 0,
    connection: { hostConnectorPartId: 'vik-host', modulePortId: 'vik-port', busId: 'bus', assignments: { gnd: 'GND', gpio1: 'P21', gpio2: 'P20' }, cableType: 'type-a', railVoltages: {} },
  } as NonNullable<ProjectDoc['modules']>[number]];
  const value = plan(); value.instanceId = 'left';
  value.peripherals = [{ partId: 'vik-host', source: 'vik-host', kind: 'vik', gpioTerminals: [['5', 'vik/b/vik-host/gpio1'], ['3', 'vik/b/vik-host/gpio2']], fixedTerminals: [] }];
  value.peripheralPins = { 'vik/b/vik-host/gpio1': 'P0.02', 'vik/b/vik-host/gpio2': 'P0.03' };
  const request = firmwareRequest(document, value).request;
  expect(request.encoder_ids).toEqual(['module/left-encoder']);
  expect(request.encoders).toEqual([{ id: 'module/left-encoder', profile: rotaryProfile, aGpio: 'P0.02', bGpio: 'P0.03' }]);
  expect(request.hardware?.modules[0]).toMatchObject({ id: 'module/left-encoder', catalogueRow: 'ec11-evqwgd001', rotaryProfile });
});

it('registers split encoders in a shared global order with remote nodes disabled', () => {
  const document = demoProject(); document.hardware = { topology: 'split', transport: 'wireless', boards: [], instances: [], sharedConstruction: null };
  const left = plan(); const right = plan(); right.boardId = 'right';
  for (const [value, id] of [[left, 'left-knob'], [right, 'right-knob']] as const) {
    value.peripherals = [{ partId: id, source: 'ceoloide/rotary_encoder_ec11_ec12', kind: 'encoder', gpioTerminals: [['A', 'encoder-a'], ['C', 'encoder-b']], fixedTerminals: [] }];
    value.peripheralPins = { 'encoder-a': 'P0.02', 'encoder-b': 'P0.03' };
  }
  const request = firmwareRequest(document, left, right).request;
  const sensors = (overlays: string[]) => overlays.find(value => value.includes('zmk,keymap-sensors'));
  expect(sensors(request.peripheral_overlays)).toBe(sensors(request.peripheral!.peripheral_overlays));
  expect(request.peripheral_overlays.join('\n')).toContain('status = "disabled"');
  expect(request.peripheral!.peripheral_overlays.join('\n')).toContain('status = "disabled"');
  expect(request.encoder_ids).toEqual(['left-knob']); expect(request.peripheral!.encoder_ids).toEqual(['right-knob']);
});

it('keeps separate physical instance scopes and metadata in split firmware hardware snapshots', () => {
  const document = demoProject();
  document.hardware = {topology:'split',transport:'wireless',boards:[],sharedConstruction:null,instances:[{id:'left',name:'Left',boardId:'b',half:'left',role:'central',flipped:false,constructionLinked:false,controllerPartId:null,mechanical:null},{id:'right',name:'Right',boardId:'b',half:'right',role:'peripheral',flipped:true,constructionLinked:false,controllerPartId:null,mechanical:null}]};
  document.moduleDefinitions = [{
    id:'sensor',name:'Sensor',family:'trackball',variant:'standard',source:{repository:'example',revision:'abc',path:'sensor.kicad_pcb',license:'MIT'},
    board:{contours:[],holes:[],thickness:1.6},mounts:[],volumes:[],openings:[],models:[],candidateModels:[],
    gates:[{output:'firmware',code:'driver',message:'Driver requires qualification'}],interfaces:[],
    electrical:{protocol:'spi',requiredSignals:[]},constituents:[],
  } as NonNullable<ProjectDoc['moduleDefinitions']>[number]];
  document.modules = ['left-module','right-module'].map((id,index) => ({
    id,definitionId:'sensor',hostBoardId:'b',hostInstanceId:index === 0 ? 'left' : 'right',hostFace:'front',facingFace:'back',
    at:{x:0,y:0},rotation:0,gap:0,attachment:'board',detached:false,serviceClearance:0,
  } as NonNullable<ProjectDoc['modules']>[number]));
  const left = {...plan(),instanceId:'left'};
  const right = {...plan(),instanceId:'right'};
  const result = firmwareRequest(document,left,right).request;
  expect(result.hardware?.physicalInstanceId).toBe('left');
  expect(result.peripheral?.hardware?.physicalInstanceId).toBe('right');
  expect(result.hardware?.modules.map(module => module.id)).toEqual(['left-module','right-module']);
  expect(result.peripheral?.hardware?.modules.map(module => module.id)).toEqual(['left-module','right-module']);
  expect(result.hardware?.physicalInstances).toEqual([{id:'left',boardId:'b'},{id:'right',boardId:'b'}]);
  expect(result.hardware).not.toHaveProperty('document');
  expect(document.physicalInstanceId).toBeUndefined();
});
