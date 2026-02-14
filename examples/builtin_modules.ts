// Test built-in module recognition and import/export lowering

// Math module tests
let pi = Math.PI;
let e = Math.E;
let floor_result = Math.floor(3.7);
let ceil_result = Math.ceil(2.3);
let sqrt_result = Math.sqrt(16);
let pow_result = Math.pow(2, 3);
let random_result = Math.random();

console.log("Math.PI:", pi);
console.log("Math.floor(3.7):", floor_result);
console.log("Math.sqrt(16):", sqrt_result);

// JSON module tests
let json_str = JSON.stringify({ name: "test" });
console.log("JSON.stringify:", json_str);

// Console methods
console.log("This is a log message");
console.error("This is an error message");
console.warn("This is a warning message");

// Export a function
export function greet(name: string): string {
  return "Hello " + name;
}

// Process module tests
let cwd = process.cwd();
let platform = process.platform;
console.log("Current directory:", cwd);
console.log("Platform:", platform);
