import type { ElectricalPlan, PeripheralRequirement } from '@boardstudio/v2-contracts';

type PeripheralFirmware = { overlays: string[]; config: string[] };

const pinFor = (plan: ElectricalPlan, name: string): string | undefined => {
  const bare = name.split('/').slice(1).join('/');
  return plan.peripheralPins[name]
    ?? plan.peripheralPins[bare]
    ?? Object.entries(plan.peripheralPins).find(([key]) => key.endsWith(`/${bare}`))?.[1];
};
const phandle = (gpio: string): string => { const [port, pin] = parseGpio(gpio); return `&gpio${port} ${pin} GPIO_ACTIVE_HIGH`; };
const required = (plan: ElectricalPlan, name: string): string => { const gpio = pinFor(plan, name); if (!gpio) throw new Error(`No resolved GPIO for peripheral function ${name}`); return phandle(gpio); };
const requiredRole = (plan: ElectricalPlan, peripheral: PeripheralRequirement, role: string, fallback: string): string => {
  const fn = peripheral.gpioTerminals.find(([terminal]) => terminal.toLowerCase() === role.toLowerCase())?.[1]
    ?? peripheral.gpioTerminals.find(([, functionName]) => functionName === fallback)?.[1];
  return required(plan, fn && plan.peripheralPins[fn] ? fn : fn ? `${peripheral.partId}/${fn}` : `${peripheral.partId}/${fallback}`);
};

export function peripheralFirmware(plan: ElectricalPlan, sensorIds = plan.peripherals.filter(item => item.kind === 'encoder').map(item => item.partId)): PeripheralFirmware {
  const displays = plan.peripherals.filter(p => p.kind === 'display-i2c' || p.kind === 'display-spi');
  if (displays.length > 1) {
    const buses = new Set(displays.map(display => display.kind));
    if (buses.size > 1) throw new Error('I2C0 and SPI1 displays require separate hardware peripherals');
    throw new Error('Only one firmware display is supported per controller');
  }
  const overlays: string[] = [];
  const config: string[] = [];
  for (const peripheral of plan.peripherals) {
    if (peripheral.kind === 'display-i2c') { overlays.push(i2cOverlay(plan, peripheral)); config.push('CONFIG_ZMK_DISPLAY=y', 'CONFIG_I2C=y', 'CONFIG_SSD1306=y', 'CONFIG_LVGL=y'); }
    else if (peripheral.kind === 'display-spi') { overlays.push(niceViewOverlay(plan, peripheral)); config.push('CONFIG_ZMK_DISPLAY=y', 'CONFIG_SPI=y', 'CONFIG_LS0XX=y', 'CONFIG_LVGL=y'); }
    else if (peripheral.kind === 'encoder') { overlays.push(encoderOverlay(plan, peripheral)); config.push('CONFIG_EC11=y', 'CONFIG_EC11_TRIGGER_GLOBAL_THREAD=y'); }
    else if (peripheral.kind === 'rgb') { if (peripheral === plan.peripherals.find(item => item.kind === 'rgb')) overlays.push(rgbOverlay(plan, peripheral)); config.push('CONFIG_ZMK_RGB_UNDERGLOW=y', 'CONFIG_SPI=y', 'CONFIG_WS2812_STRIP=y'); }
    else if (!['split', 'power-switch', 'reset', 'battery'].includes(peripheral.kind)) throw new Error(`No source-verified firmware profile for ${peripheral.kind}`);
  }
  const encoders = plan.peripherals.filter(item => item.kind === 'encoder');
  for (const id of sensorIds.filter(id => !encoders.some(item => item.partId === id))) overlays.push(`/ { ${encoderLabel(id)}: ${encoderLabel(id)} { compatible = "alps,ec11"; status = "disabled"; }; };`);
  if (sensorIds.length) overlays.push(`/ { sensors { compatible = "zmk,keymap-sensors"; sensors = <${sensorIds.map(id => `&${encoderLabel(id)}`).join(' ')}>; triggers-per-rotation = <20>; }; };`);
  return { overlays, config: [...new Set(config)] };
}

function i2cOverlay(plan: ElectricalPlan, peripheral: PeripheralRequirement): string {
  const sda = requiredRole(plan, peripheral, 'SDA', 'i2c/SDA'); const scl = requiredRole(plan, peripheral, 'SCL', 'i2c/SCL');
  return `&i2c0 { status = "okay"; pinctrl-0 = <&boardstudio_i2c>; pinctrl-names = "default"; oled: ssd1306@3c { compatible = "solomon,ssd1306fb"; reg = <0x3c>; width = <128>; height = <32>; segment-offset = <0>; page-offset = <0>; display-offset = <0>; multiplex-ratio = <31>; segment-remap; com-invdir; com-sequential; inversion-on; prechargep = <0x22>; }; }; &pinctrl { boardstudio_i2c: boardstudio_i2c { group1 { psels = <NRF_PSEL(TWIM_SDA, ${port(sda)}, ${number(sda)})>, <NRF_PSEL(TWIM_SCL, ${port(scl)}, ${number(scl)})>; }; }; }; / { chosen { zephyr,display = &oled; }; };`;
}

function niceViewOverlay(plan: ElectricalPlan, peripheral: PeripheralRequirement): string {
  const mosi = requiredRole(plan, peripheral, 'MOSI', 'spi/MOSI'); const sck = requiredRole(plan, peripheral, 'SCK', 'spi/SCK'); const cs = requiredRole(plan, peripheral, 'CS', 'CS');
  return `&spi1 { status = "okay"; pinctrl-0 = <&boardstudio_spi>; pinctrl-names = "default"; cs-gpios = <${cs}>; nice_view: ls0xx@0 { compatible = "sharp,ls0xx"; spi-max-frequency = <1000000>; reg = <0>; width = <160>; height = <68>; }; }; &pinctrl { boardstudio_spi: boardstudio_spi { group1 { psels = <NRF_PSEL(SPIM_MOSI, ${port(mosi)}, ${number(mosi)})>, <NRF_PSEL(SPIM_SCK, ${port(sck)}, ${number(sck)})>; }; }; }; / { chosen { zephyr,display = &nice_view; }; };`;
}

const encoderLabel = (id: string) => `encoder_${Array.from(new TextEncoder().encode(id), byte => byte.toString(16).padStart(2, '0')).join('')}`;

function encoderOverlay(plan: ElectricalPlan, peripheral: PeripheralRequirement): string {
  const a = requiredRole(plan, peripheral, 'A', 'encoder/A');
  const b = requiredRole(plan, peripheral, 'C', 'encoder/C');
  return `/ { ${encoderLabel(peripheral.partId)}: ${encoderLabel(peripheral.partId)} { compatible = "alps,ec11"; status = "okay"; a-gpios = <${a.replace('GPIO_ACTIVE_HIGH', '(GPIO_ACTIVE_HIGH | GPIO_PULL_UP)')}>; b-gpios = <${b.replace('GPIO_ACTIVE_HIGH', '(GPIO_ACTIVE_HIGH | GPIO_PULL_UP)')}>; steps = <80>; }; };`;
}

function rgbOverlay(plan: ElectricalPlan, peripheral: PeripheralRequirement): string {
  const din = requiredRole(plan, peripheral, 'P4', 'rgb-in');
  return `#include <zephyr/dt-bindings/led/led.h>\n&spi3 { status = "okay"; pinctrl-0 = <&boardstudio_spi3>; pinctrl-names = "default"; led_strip: ws2812@0 { compatible = "worldsemi,ws2812-spi"; reg = <0>; spi-max-frequency = <4000000>; chain-length = <${plan.peripherals.filter(item => item.kind === 'rgb').length}>; spi-one-frame = <0x70>; spi-zero-frame = <0x40>; color-mapping = <LED_COLOR_ID_GREEN LED_COLOR_ID_RED LED_COLOR_ID_BLUE>; }; }; &pinctrl { boardstudio_spi3: boardstudio_spi3 { group1 { psels = <NRF_PSEL(SPIM_MOSI, ${port(din)}, ${number(din)})>; }; }; }; / { chosen { zmk,underglow = &led_strip; }; };`;
}

function parseGpio(value: string): [string, string] {
  const match = /^P([01])\.(\d{1,2})$/.exec(value) ?? /^&gpio([01])\s+(\d{1,2})/.exec(value);
  if (!match) throw new Error(`Invalid allocated GPIO ${value}`);
  if (Number(match[2]) > 31) throw new Error(`GPIO pin out of range ${value}`);
  return [match[1], String(Number(match[2]))];
}
function port(value: string): string { return parseGpio(value)[0]; }
function number(value: string): string { return parseGpio(value)[1]; }
