import { renderToStaticMarkup } from 'react-dom/server';
import { expect, it } from 'vitest';
import { catalogue } from '@boardstudio/v2-ergogen';
import { Ergogen2DPreview } from './Ergogen2DPreview';
it('renders the actual pin label rather than the KiCad text category', () => {
  const definition = catalogue().find(item => item.generator?.source === 'ceoloide/mcu_supermini_nrf52840')!;
  definition.generator!.parameters.P1 = 'ROW1';
  const markup = renderToStaticMarkup(<svg><Ergogen2DPreview definition={definition}/></svg>);
  expect(markup).toContain('>ROW1</text>');
  expect(markup).not.toContain('>user</text>');
});
