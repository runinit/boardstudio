// Generated initialization entry: all core worker and error policy lives in Rust.
import init, { start_core_worker } from './boardstudio_web.js';
await init();
start_core_worker();
