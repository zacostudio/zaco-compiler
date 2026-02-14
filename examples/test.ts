import * as fs from "fs";
import * as path from "path";

const filePath = path.join(__dirname, "example.txt");

fs.readFile(filePath, "utf-8", (err, data) => {
	if (err) {
		console.error("Error reading file:", err);
		return;
	}
	if (data) {
		console.log("==> Test Data:", data);
	}
});

console.log("File read successfully");
