// Shared KiCad form reader for source-layout maintenance commands.
function tokens(source) {
  const result = [];
  const pattern = /\s*(\(|\)|"(?:\\.|[^"\\])*"|[^\s()]+)/guy;
  let offset = 0;
  while (offset < source.length) {
    pattern.lastIndex = offset;
    const match = pattern.exec(source);
    if (!match) {
      if (/^\s*$/u.test(source.slice(offset))) break;
      throw new Error(`Invalid KiCad source at byte ${offset}`);
    }
    result.push(match[1]);
    offset = pattern.lastIndex;
  }
  return result;
}

export function parseForms(source) {
  const input = tokens(source);
  let cursor = 0;
  const read = () => {
    const token = input[cursor++];
    if (token === undefined || token === ')') throw new Error('Unbalanced KiCad source');
    if (token !== '(') return token.startsWith('"') ? `\u0000${JSON.parse(token)}` : token;
    const result = [];
    while (input[cursor] !== ')') {
      if (cursor >= input.length) throw new Error('Unbalanced KiCad source');
      result.push(read());
    }
    cursor++;
    return result;
  };
  const forms = [];
  while (cursor < input.length) {
    const node = read();
    if (!Array.isArray(node)) throw new Error('KiCad source must contain KiCad forms');
    forms.push(node);
  }
  return forms;
}

export function value(node) {
  if (typeof node !== 'string') throw new Error('Expected KiCad scalar');
  return node.startsWith('\u0000') ? node.slice(1) : node;
}

export function child(node, name) {
  return node.find((item) => Array.isArray(item) && item[0] === name);
}
