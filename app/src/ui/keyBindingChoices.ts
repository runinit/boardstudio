export const keyBindingChoices: readonly (readonly [string, string])[] = [
  ['&none', 'Unassigned'], ['&trans', 'Transparent'],
  ...Array.from({ length: 26 }, (_, i) => { const key = String.fromCharCode(65 + i); return [`&kp ${key}`, key] as const; }),
  ...Array.from({ length: 10 }, (_, i) => [`&kp N${i}`, `${i}`] as const),
  ...Array.from({ length: 12 }, (_, i) => [`&kp F${i + 1}`, `F${i + 1}`] as const),
  ['&kp SPACE', 'Space'], ['&kp ENTER', 'Enter'], ['&kp ESC', 'Esc'], ['&kp TAB', 'Tab'], ['&kp BSPC', 'Backspace'],
  ['&kp LSHFT', 'Shift'], ['&kp LCTRL', 'Ctrl'], ['&kp LALT', 'Alt'], ['&kp LGUI', 'Super'],
  ['&kp UP', 'Up'], ['&kp DOWN', 'Down'], ['&kp LEFT', 'Left'], ['&kp RIGHT', 'Right'],
  ['&kp MINUS', '-'], ['&kp EQUAL', '='], ['&kp LBKT', '['], ['&kp RBKT', ']'], ['&kp BSLH', '\\'], ['&kp SEMI', ';'], ['&kp SQT', "'"], ['&kp COMMA', ','], ['&kp DOT', '.'], ['&kp FSLH', '/'], ['&kp GRAVE', '`'],
];
export function bindingLabel(binding: string): string {
  return binding === '&none' || binding === '&trans' ? '' : keyBindingChoices.find(([value]) => value === binding)?.[1] ?? binding.replace(/^&kp /, '');
}
