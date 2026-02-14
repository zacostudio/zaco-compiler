// Ownership demo — Zaco hybrid memory management

class Point {
    x: number;
    y: number;

    constructor(x: number, y: number) {
        this.x = x;
        this.y = y;
    }

    distanceTo(ref other: Point): number {
        let dx: number = this.x - other.x;
        let dy: number = this.y - other.y;
        return Math.sqrt(dx * dx + dy * dy);
    }
}

function createPoint(x: number, y: number): Point {
    return new Point(x, y);
}

function main(): void {
    let a: Point = createPoint(0, 0);
    let b: Point = createPoint(3, 4);

    // 'a' and 'b' are owned values
    let dist: number = a.distanceTo(b);  // b is borrowed (ref) here
    console.log(dist);  // 5

    // Ownership transfer (move)
    let c: Point = a;  // a is moved to c, a is no longer usable

    // Explicit clone
    let d: Point = clone b;  // d is a deep copy of b

    console.log(c.x);  // 0
    console.log(d.y);  // 4
}

main();
