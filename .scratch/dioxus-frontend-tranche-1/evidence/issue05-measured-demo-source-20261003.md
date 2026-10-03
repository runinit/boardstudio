# Issue 05 measured-demo source packet

## Scope

The Dioxus Library and Project menu now declare the same fifteen measured layouts, in the order and with the display names exported by the React demo catalogue. Each card preview reads its prepared fixture JSON; opening a card continues through `Runtime::open_fixture`, which creates a fresh ID and uses the existing archive import/Session path.

Fixture preparation now invokes the existing `openKeyboardDemo` path for every `keyboardDemos` entry. That path constructs the measured project from the pinned layout source, runs the existing Core electrical resolver and applies the accepted plans before the fixture is packed with the existing `packProject` service. The preparer writes JSON/archive pairs under `measured-<id>` and records the exact source inputs, board identities, key counts, archive hashes and referenced asset hashes in the generated provenance manifest.

The existing REVIUNG41 archive fixture is still emitted unchanged for prior Issue 20 evidence; the visible REVIUNG41 measured-layout card points to `measured-reviung41`. Sofle v2/RGB/Choc and VIK module-review cards/fixtures remain in place.

## Source pins

The authoritative reference catalogue and generation path are `app/src/demos/keyboards.ts`, `app/src/demos/keyboard-layouts.json`, and `app/src/demos/physicalLayout.ts`. The fixture adapter is `scripts/prepare-m1-fixtures.mjs`; card wiring is `web/src/presentation/library.rs`. The generated provenance manifest records the SHA-256 values of all inputs when the coordinator refreshes the packaged fixtures.

This packet is based on integrated source `8bb958ee1c5f662f17b1df861ea26f9d12ca4eaf`. SHA-256 pins at source freeze:

| Input | SHA-256 |
| --- | --- |
| `app/src/demos/keyboards.ts` | `8139ad903b81868a4a2f12f441473d53f8baa6f34a6f652b1b2d690fe2092157` |
| `app/src/demos/keyboard-layouts.json` | `c292a5bdef43db4c8fcefce56224b55a3ea0329602e3c646fdb004783ed0cacc` |
| `app/src/demos/physicalLayout.ts` | `bf9b57d12af57bb3ea3709f28a4ec01c28f592be3ed9e9a8a66a61e85f284e9c` |
| `app/src/demo.ts` | `6c244766d08ef4102a28ff28e8700d5ce13b0f16fc4184dd7f05be9c008f818f` |
| `app/src/storage.ts` | `15664582c6770f0002d88a19be680fec5dd32ab003bd2250b3cadcb36216e282` |
| `scripts/prepare-m1-fixtures.mjs` | `2c031e015f3b6a6d0a590f61002797fc646525a17e8c748b3bda39f0dd82e96e` |
| `web/src/presentation/library.rs` | `aeab4988a5ed66d12ca90514b51515825d9e11075e6a68c35152d6222a4a2942` |

## Verification boundary

This receipt records source wiring only. Fixture generation, combined package checks, and public browser journeys were not run in this author packet; they remain open for the coordinator's integrated candidate. No Issue 05 acceptance checkbox or F2.1 parent criterion is claimed complete by source presence.
