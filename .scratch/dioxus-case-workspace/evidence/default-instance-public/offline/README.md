# ed242c02 offline bundle verification

Candidate source/build identity comes from the retained `default-instance-source-reviewed/build-provenance.json`: source commit `ed242c0258f59af6fde65d8ffe1580334e2836c4`, build ID `frontend-instance-defaults-20261002`, provenance status `complete`. The same artifact was served at `http://127.0.0.1:34665/` and `http://127.0.0.1:34665/boardstudio/`.

Each mount was tested in a separate fresh disk-backed browser profile under `/var/tmp/frontend-run/profiles/ed242-offline-{root,subpath}` and a separate named agent-browser session. The workflow used ordinary navigation, allowed the service worker to install/activate, recorded the controller/registration/cache state, set browser network offline, and reloaded the landing page.

| Mount | Online proof | Offline reload proof | Service worker scope | Cache entries |
|---|---|---|---|---:|
| `/` | `root-online-state.json`, `root-online.png` | `root-offline-state.json`, `root-offline-reload.png` | `http://127.0.0.1:34665/` | 51 |
| `/boardstudio/` | `subpath-online-state.json`, `subpath-online.png` | `subpath-offline-state.json`, `subpath-offline-reload.png` | `http://127.0.0.1:34665/boardstudio/` | 51 |

Both online snapshots report `navigator.onLine=true`, an activated registration and a non-null page controller. Both post-reload offline snapshots report `navigator.onLine=false`, the same activated controller, and the full BoardStudio landing page (`Open a keyboard`, saved demo list, import action). Each isolated mount has one cache with 51 entries. Root cache URLs are listed in `root-offline-state.json`; the corresponding subpath URLs are in `subpath-offline-state.json`. The browser error logs `root-errors.txt` and `subpath-errors.txt` are empty.

Screenshots are agent-browser outputs copied from the absolute screenshot paths it returned. No storage was cleared or injected, no app project/profile was changed, and no server/build/source process was modified. This verifies the tested root and subpath mounts with this exact build; it does not establish offline behavior for other origins, browser engines, or cache eviction conditions.
