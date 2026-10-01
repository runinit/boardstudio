// Test-only entry: compile the actual reference adapter without changing its source.
import * as storage from '../../../app/src/storage';
(globalThis as any).referenceStorage = storage;
