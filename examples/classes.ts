// Class inheritance and generics example

interface Printable {
    toString(): string;
}

class Animal implements Printable {
    name: string;
    age: number;

    constructor(name: string, age: number) {
        this.name = name;
        this.age = age;
    }

    speak(): string {
        return this.name + " makes a sound";
    }

    toString(): string {
        return this.name + " (age: " + this.age + ")";
    }
}

class Dog extends Animal {
    breed: string;

    constructor(name: string, age: number, breed: string) {
        super(name, age);
        this.breed = breed;
    }

    speak(): string {
        return this.name + " barks!";
    }
}

class Cat extends Animal {
    indoor: boolean;

    constructor(name: string, age: number, indoor: boolean) {
        super(name, age);
        this.indoor = indoor;
    }

    speak(): string {
        return this.name + " meows!";
    }
}

// Generic container
class Container<T> {
    private value: T;

    constructor(value: T) {
        this.value = value;
    }

    get(): T {
        return this.value;
    }

    set(newValue: T): void {
        this.value = newValue;
    }
}

function main(): void {
    let dog: Dog = new Dog("Buddy", 3, "Golden Retriever");
    let cat: Cat = new Cat("Whiskers", 5, true);

    console.log(dog.speak());
    console.log(cat.speak());

    let box: Container<number> = new Container<number>(42);
    console.log(box.get());
    box.set(100);
    console.log(box.get());
}

main();
