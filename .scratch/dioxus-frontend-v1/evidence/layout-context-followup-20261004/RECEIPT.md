# Layout context and attachment-choice follow-up

Candidate `frontend-preset-customization-20261004`, release source4e716156, port34822; current test/evidence HEAD7f3165b4. Reference recovered exact5a472a9 build on5175, DOM script main-CYxchQWA.js. Both use saved `PCB grouped qualification20261004`, originally identical imported input recorded in ../pcb-host-themes-20261004/input.json; subsequent paired recipe imports are recorded separately. No document edit was made in this journey; candidate remained Revision10 · Saved.

Public Objects tree route: Left PCB → keys → Column1 → Key1.1 → automatic diode companion → Key1.1 → Left PCB.

- Initially both show board-context Left PCB Inspector with no component/key selection.
- Selecting keys shows 4rows/6columns, pitches19.05, originX9.179999999999978/Y-78.048, rotation0, MX Hotswap and South-facing orientation in both.
- Column1 contains4 logical keys. Both UIs report “8 keys selected” in this fixture (retained wording, not a claim of8 logical keys); both show “keys · Column1”, Splay0 and matching origins.
- Column Relations text matches: independent layout with separately editable geometry/components, shared matrix pitch/stagger/splay edited in Properties.
- Moving to Key1.1 resets both from column Relations to Properties. Both show “1 keys selected”, Key enabled checked, Local X/Y0, rotation0, Key Assembly and Attached components.
- Expanding Key1.1 exposes the same “diode tht sod123 Automatic companion” row. Selecting it opens left-diode-24-keys with matching numeric position (candidate fullprecision16.579999999999977/-79.548, reference display16.58/-79.55). No lock/driven unavailability is proved by this independent fixture.
- Returning to Left PCB removes key/component Inspector and shows board Inspector in both. Arbitrary two-key selection, all empty-selection triggers and all nested keyboard routes are not claimed.

## Reproduced F3.5-C05 gap, before source repair

Key1.1 Properties → Replace diode exposes59 options in the candidate and52 in reference. Both retain the assigned `assembly-preset-mx-hotswap-south-left-keys-0/definition/diode`. The reference excludes all seven other preset snapshots; the candidate offers them. There are no options missing from the candidate relative to reference. The extra candidate values are:

- assembly-preset-mx-hotswap-south-left-keys-0/definition/switch
- assembly-preset-mx-hotswap-south-left-thumbs-0/definition/switch
- assembly-preset-mx-hotswap-south-left-thumbs-0/definition/diode
- assembly-preset-mx-hotswap-south-right-keys-0/definition/switch
- assembly-preset-mx-hotswap-south-right-keys-0/definition/diode
- assembly-preset-mx-hotswap-south-right-thumbs-0/definition/switch
- assembly-preset-mx-hotswap-south-right-thumbs-0/definition/diode

This is an actual mounted option-set discrepancy, corroborating the independent source reconciliation in ../layout-criterion-reconciliation-20261004.md. No leaked option was selected and no replacement edit was submitted. Native regression must fail on the actual choice policy before changing production code. Existing successful replacement/history/mirror-locality evidence remains valid; the option-set clause stays open until repaired and packaged replayed. This adds no catalogue fault-injection requirement.

## Desktop nested outline continuation

On the same paired saved fixture and unchanged candidate script `boardstudio-web-dxh7a84b355b3d74c.js`, Objects → Outline → Edit perimeter points mounted Perimeter in both frontends. Both showed Point1 of34, X=-3.82 and Y=-65.048. The Inspector Done action returned both to Board outline. No coordinate or topology edit was submitted. This is a pointer-driven nested route/return observation only; it does not claim keyboard focus restoration or Escape coverage. Mobile remains deferred.
