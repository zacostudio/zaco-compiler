// Simpler test - just export and call locally defined functions

function add(a: number, b: number): number {
    return a + b;
}

const x = 10;
const y = 5;

const sum = add(x, y);

console.log("Sum:");
console.log(sum);

export { add };
