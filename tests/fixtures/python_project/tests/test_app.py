from src.app import greet, compute

def test_greet():
    assert greet("Alice") == "Hello, Alice!"

def test_compute():
    assert compute(2, 3) == 5
