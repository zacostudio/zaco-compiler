// Test NPM package import resolution
import { chunk } from "lodash";

const numbers = [1, 2, 3, 4, 5, 6];
const chunked = chunk(numbers, 2);

console.log("Chunked array:");
console.log(chunked);
