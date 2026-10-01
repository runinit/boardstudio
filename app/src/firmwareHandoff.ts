import { peripheralFirmware } from './firmwarePeripherals';
import type { ElectricalPlan, FirmwareKey, FirmwareRequest, FirmwareScanMode, PeripheralRequirement, ProjectDoc, ScanPin } from '@boardstudio/v2-contracts';

type FirmwareHandoffResult = { request: FirmwareRequest; warnings: string[] };
type PlanExtensions = ElectricalPlan & { moduleAliases?: Record<string, string>; peripheralTerminals?: Record<string, string>; jumpers?: unknown[]; instanceId?: string | null };

function errors(plan: ElectricalPlan): string[] { return plan.diagnostics.filter(d => d.severity === 'error').map(d => d.message); }

function pin(terminal: string, gpio: string): ScanPin { return { terminal, gpio }; }

function peripheralWarnings(plan: ElectricalPlan): string[] {
  return plan.peripherals.flatMap((item: PeripheralRequirement) => {
    const unresolved = item.gpioTerminals.some(([_, fn]) => !findPeripheralPin(plan, fn));
    const unsupported = !['split', 'power-switch', 'reset', 'battery', 'display-i2c', 'display-spi', 'encoder', 'press', 'rgb'].includes(item.kind);
    return unresolved ? [`No resolved GPIO for ${item.kind} ${item.partId}`] : unsupported ? [`Firmware profile for ${item.kind} is not implemented; export is blocked`] : [];
  });
}

export function firmwareRequest(document: ProjectDoc, plan: ElectricalPlan, peripheralPlan?: ElectricalPlan): FirmwareHandoffResult {
  const extended = plan as PlanExtensions;
  const failures = [...errors(plan), ...peripheralWarnings(plan)];
  const controllerProfile = plan.controllerProfile;
  if (!controllerProfile) failures.push('No reviewed controller profile is selected');
  if (failures.length) throw new Error(failures.join('; '));
  const transport = document.hardware?.transport ?? 'none';
  if (transport !== 'none' && !peripheralPlan) throw new Error('A split firmware handoff requires a distinct peripheral electrical plan');
  const assignments = [...plan.assignments];
  const mode: FirmwareScanMode = plan.mode === 'direct' ? 'direct' : 'matrix';
  const rows = (mode === 'matrix' ? plan.rowPins : []).map((terminal, index) => pin(extended.moduleAliases?.[terminal] ?? terminal, assignments.find(a => a.row === index)?.rowFirmwareGpio ?? '')).filter(p => p.gpio);
  const columns = (mode === 'matrix' ? plan.columnPins : []).map((terminal, index) => pin(extended.moduleAliases?.[terminal] ?? terminal, assignments.find(a => a.column === index)?.columnFirmwareGpio ?? '')).filter(p => p.gpio);
  const keys: FirmwareKey[] = assignments.map(a => ({ id: a.keyId, row: a.row, column: a.column }));
  const directPins = assignments.map(a => a.directGpio ? pin(plan.moduleAliases[a.columnPin] ?? a.columnPin, a.directGpio) : null).filter((p): p is ScanPin => Boolean(p));
  const auxiliary = plan.peripherals.flatMap(peripheral => peripheral.gpioTerminals.filter(([terminal,fn]) => (peripheral.kind === 'encoder' || peripheral.kind === 'press') && (fn.endsWith('/encoder-push') || fn.endsWith('/input-push') || terminal === 'S1')).map(([,fn]) => ({ id: peripheral.pressKeyId ?? `${peripheral.partId}/push`, function: fn })));
  const auxiliaryPins = auxiliary.map(item => pin(plan.peripheralTerminals[item.function], plan.peripheralPins[item.function]));
  auxiliary.forEach((item, index) => keys.push({ id: item.id, row: mode === 'direct' ? 1 : rows.length, column: index }));
  const configuredBindings = document.hardware?.boards.find(board => board.boardId === plan.boardId)?.keyBindings ?? {};
  const sensorIds = [...plan.peripherals, ...(peripheralPlan?.peripherals ?? [])].filter(item => item.kind === 'encoder').map(item => item.partId);
  const profiled = [...plan.peripherals, ...(peripheralPlan?.peripherals ?? [])].filter(item => item.kind === 'encoder').every(item => Boolean(item.rotary));
  const legacyPeripherals = (current: ElectricalPlan) => peripheralFirmware(profiled ? { ...current, peripherals: current.peripherals.filter(item => item.kind !== 'encoder') } : current, profiled ? [] : sensorIds);
  const peripherals = legacyPeripherals(plan);
  const request: FirmwareRequest = {
    encoders: profiled ? plan.peripherals.flatMap(item => {
      if (item.kind !== 'encoder' || !item.rotary) return [];
      const gpio = (terminal: string) => findPeripheralPin(plan, item.gpioTerminals.find(([name]) => name === terminal)?.[1] ?? '') ?? '';
      return [{ id: item.partId, profile: item.rotary, aGpio: gpio(item.rotary.a), bGpio: gpio(item.rotary.b) }];
    }) : undefined,
    keymap: document.keymap ?? undefined,
    encoder_ids: plan.peripherals.filter(item => item.kind === 'encoder').map(item => item.partId),
    controller_profile: controllerProfile!,
    board_name: plan.boardId ?? document.name,
    rows, columns, keys, diode_direction: plan.diodeDirection,
    mode, direct_pins: directPins, auxiliary_pins: auxiliaryPins, key_bindings: keys.map(key => configuredBindings[key.id] ?? '&none'), peripheral_config: [...peripherals.config, ...(auxiliaryPins.length ? ['CONFIG_ZMK_KSCAN_COMPOSITE_DRIVER=y'] : [])],
    transport: transport === 'wireless' ? 'wireless' : transport === 'wired' ? 'wired-uart' : null,
    uart_tx: transport === 'wired' ? uartPin(plan, 'split-tx') : null,
    uart_rx: transport === 'wired' ? uartPin(plan, 'split-rx') : null,
    matrix_row_offset: 0,
    peripheral_overlays: peripherals.overlays,
    peripheral: peripheralPlan ? { ...firmwareRequest({ ...document, hardware: { topology: 'unibody', transport: 'none', boards: document.hardware?.boards ?? [], instances: [], sharedConstruction: null } }, peripheralPlan).request, peripheral_overlays: legacyPeripherals(peripheralPlan).overlays, transport: transport === 'wireless' ? 'wireless' : transport === 'wired' ? 'wired-uart' : null, matrix_row_offset: plan.rowPins.length, uart_tx: transport === 'wired' ? uartPin(peripheralPlan, 'split-rx') : null, uart_rx: transport === 'wired' ? uartPin(peripheralPlan, 'split-tx') : null } : null,
  };
  return { request, warnings: plan.diagnostics.filter(d => d.severity !== 'error').map(d => d.message) };
}

function findPeripheralPin(plan: ElectricalPlan, functionName: string): string | undefined {
  return plan.peripheralPins[functionName] ?? Object.entries(plan.peripheralPins).find(([key]) => key.endsWith(`/${functionName}`))?.[1];
}

function uartPin(plan: ElectricalPlan, functionName: string): ScanPin | null {
  const gpio = findPeripheralPin(plan, functionName);
  const terminal = plan.peripheralTerminals[functionName] ?? Object.entries(plan.peripheralTerminals).find(([key]) => key.endsWith(`/${functionName}`))?.[1];
  return gpio && terminal ? pin(terminal, gpio) : null;
}
