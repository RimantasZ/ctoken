def greet(name: str) -> str:
    return f"Hello, {name}!"

def compute(x: int, y: int) -> int:
    return x + y

if __name__ == "__main__":
    print(greet("world"))
    print(compute(3, 4))
