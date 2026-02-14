// Fibonacci example — demonstrates functions and loops

function fibonacci(n: number): number {
    if (n <= 1) {
        return n;
    }

    let a: number = 0;
    let b: number = 1;

    for (let i: number = 2; i <= n; i = i + 1) {
        let temp: number = b;
        b = a + b;
        a = temp;
    }

    return b;
}

function main(): void {
    for (let i: number = 0; i < 20; i = i + 1) {
        console.log(fibonacci(i));
    }
}

main();
