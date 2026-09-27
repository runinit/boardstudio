import type {
  MechanicalBuiltinProfile, MechanicalConfiguration, MechanicalExtraction,
  MechanicalPartProfile, MechanicalSwitchFamily,
} from '@boardstudio/v2-contracts';
import { defaultPlateFoamThickness, defaultPlateThickness, plateToPcbGap, profileSwitchFamily } from '../mechanicalPresets';

type LoadProfile = (definitionId: string, source: MechanicalBuiltinProfile, gap: number) => Promise<MechanicalPartProfile>;
type ProfileFeedback = {
  setPending: (pending: boolean) => void;
  setError: (error: string) => void;
  update: (patch: Partial<MechanicalConfiguration>) => void;
};

/** A delayed profile must never overwrite another board or a newer mechanical edit. */
export class MechanicalProfileController {
  private configuration?: MechanicalConfiguration;
  private scope = '';
  private version = 0;
  private requestId = 0;

  receive(configuration: MechanicalConfiguration | undefined, scope: string): void {
    if (configuration !== this.configuration || scope !== this.scope) this.version += 1;
    this.configuration = configuration;
    this.scope = scope;
  }

  invalidate(): void {
    this.version += 1;
    this.requestId += 1;
  }

  private async request(load: () => Promise<MechanicalPartProfile>, commit: (profile: MechanicalPartProfile) => void, feedback: ProfileFeedback): Promise<void> {
    const version = this.version;
    const requestId = ++this.requestId;
    const current = () => version === this.version && requestId === this.requestId;
    feedback.setPending(true);
    feedback.setError('');
    try {
      const profile = await load();
      if (current()) commit(profile);
    } catch (cause) {
      if (current()) feedback.setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      // An earlier request must not clear a newer request's busy state.
      if (requestId === this.requestId) feedback.setPending(false);
    }
  }

  async assign(definitionId: string, source: MechanicalBuiltinProfile, load: LoadProfile, feedback: ProfileFeedback): Promise<void> {
    const config = this.configuration;
    if (!config || config.profiles.some(profile => profile.definitionId === definitionId)) return;
    const family: MechanicalSwitchFamily | undefined = source === 'mx-switch' ? 'mx'
      : source === 'choc-v1-switch' ? 'choc-v1' : source === 'choc-v2-switch' ? 'choc-v2' : undefined;
    const familyChanged = family !== undefined && family !== profileSwitchFamily(config);
    const plateThickness = familyChanged ? defaultPlateThickness(family) : config.plateThickness;
    const plateToPcb = family ? plateToPcbGap(family, plateThickness) : config.plateToPcb;
    await this.request(() => load(definitionId, source, plateToPcb), loaded => {
      const profile = family ? { ...loaded, switchFamily: family, plateToPcb } : loaded;
      feedback.update({
        ...(familyChanged ? { plateThickness, plateToPcb, plateFoamThickness: defaultPlateFoamThickness(plateToPcb) } : {}),
        profiles: [...config.profiles, profile],
      });
    }, feedback);
  }

  async assignStabilizer(partId: string, definitionId: string, source: MechanicalBuiltinProfile, load: LoadProfile, feedback: ProfileFeedback): Promise<void> {
    const config = this.configuration;
    if (!config) return;
    const stabilizer = config.stabilizers?.find(entry => entry.partId === partId) ?? { partId, kind: 'none' as const, units: 2 };
    await this.request(() => load(definitionId, source, config.plateToPcb), profile => {
      feedback.update({ stabilizers: [...(config.stabilizers ?? []).filter(entry => entry.partId !== partId), { ...stabilizer, profile }] });
    }, feedback);
  }
}

export function applyMechanicalExtraction(profile: MechanicalPartProfile, extraction: MechanicalExtraction, name?: string): MechanicalPartProfile {
  return {
    ...profile,
    source: name ? `KiCad ${name}` : profile.source,
    sourceGeometry: extraction.sourceGeometry,
    pcbHoles: extraction.pcbHoles,
    clearances: extraction.clearanceEnvelopes,
    cutouts: extraction.plateCutouts,
  };
}
