// Test circular dependency detection
import { b } from "./circular_b";

export function a(): number {
    return b() + 1;
}
