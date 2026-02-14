// Test import tracking for fs and path modules

import { readFileSync, writeFileSync } from "fs";
import { join, dirname } from "path";

// Use imported fs functions
let content = readFileSync("test.txt", "utf-8");
console.log("File content:", content);

writeFileSync("output.txt", "Hello from Zaco!");

// Use imported path functions
let fullPath = join("/home", "user");
let dir = dirname("/home/user/file.txt");

console.log("Joined path:", fullPath);
console.log("Directory:", dir);
