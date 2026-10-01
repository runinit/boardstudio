// Test-only entry that compiles the unmodified React storage adapter.
import * as storage from '../../../../app/src/storage';

(globalThis as any).referenceStorage = storage;
