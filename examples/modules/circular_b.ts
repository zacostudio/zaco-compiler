// Test circular dependency detection
import { a } from "./circular_a";

export function b(): number {
    return a() + 2;
}
