import { MockBridge } from './mock';
import { TauriBridge } from './tauri';

export type Bridge = TauriBridge | MockBridge;

let bridgeInstance: Bridge | null = null;

function isTauri(): boolean {
  return !!((window as unknown as Record<string, unknown>).__TAURI_INTERNALS__);
}

export async function createBridge(): Promise<Bridge> {
  if (bridgeInstance) return bridgeInstance;

  if (isTauri()) {
    const bridge = new TauriBridge();
    await bridge.init();
    bridgeInstance = bridge;
  } else {
    const bridge = new MockBridge();
    await bridge.init();
    bridgeInstance = bridge;
  }

  return bridgeInstance;
}

export async function getBridge(): Promise<Bridge> {
  if (bridgeInstance) {
    return bridgeInstance;
  }

  return createBridge();
}

export { isTauri };
