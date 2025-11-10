# CRDT Counter Benefits in Automerge

## What is a CRDT Counter?

A CRDT (Conflict-free Replicated Data Type) counter is a data structure designed for distributed systems where multiple replicas can concurrently increment/decrement a value, and all changes merge correctly without conflicts.

## The Problem with Regular Integers

### Scenario: Concurrent Increments

Imagine two clients editing the same document offline:

```rust
// Initial state: counter = 5

// Client A (offline)
counter = 6  // User clicks increment

// Client B (offline, same time)
counter = 6  // User clicks increment

// When they sync: counter = 6 ❌
// Expected: counter = 7 (5 + 1 + 1)
```

With a regular integer, the last-write-wins. One increment is lost!

## The Solution: CRDT Counter

### How It Works

Instead of storing just a value, a CRDT counter stores:
- A base value
- A set of increments/decrements from each actor

When merging:
- All increments are summed
- No updates are lost
- Order doesn't matter (commutative)

```rust
// Initial state: counter = Counter(5)

// Client A (offline)
counter.increment(1)  // Stores: +1 from Actor A

// Client B (offline, same time)  
counter.increment(1)  // Stores: +1 from Actor B

// When they sync: counter.value() = 7 ✅
// Correctly: 5 + 1 + 1 = 7
```

## In Our Code

### Before: Plain i64

```rust
struct Doc {
    counter: i64,
}

// Incrementing
let current = get_counter(doc);
doc.transact(|tx| {
    tx.put(ROOT, "counter", current + 1)?;
})?;
```

**Problem**: This is a read-modify-write pattern. Concurrent modifications conflict.

### After: autosurgeon::Counter

```rust
struct Doc {
    counter: autosurgeon::Counter,
}

// Incrementing
let mut state: Doc = hydrate(doc).unwrap_or_default();
state.counter.increment(1);
doc.transact(|tx| reconcile(tx, &state))?;
```

**Benefit**: The increment operation is stored as a CRDT operation, not a value replacement.

## API Examples

### Creating Counters

```rust
// Start at zero
let counter = Counter::default();

// Start at specific value
let counter = Counter::with_value(42);
```

### Modifying Counters

```rust
// Increment by positive amount
counter.increment(5);

// Decrement (negative increment)
counter.increment(-3);
```

### Reading Values

```rust
let value: i64 = counter.value();
println!("Counter is at: {}", value);
```

## Real-World Scenario

### Without CRDT Counter

```
Browser 1: Clicks +1 (counter: 0 → 1)
Browser 2: Clicks +1 (counter: 0 → 1) [concurrent]
CLI:       Clicks +1 (counter: 0 → 1) [concurrent]

After merge: counter = 1 ❌
Lost: 2 increments
```

### With CRDT Counter

```
Browser 1: Clicks +1 (stores: Actor1: +1)
Browser 2: Clicks +1 (stores: Actor2: +1) [concurrent]
CLI:       Clicks +1 (stores: Actor3: +1) [concurrent]

After merge: counter = 3 ✅
All increments preserved!
```

## Key Properties

### 1. Commutativity
Operations can be applied in any order:
```
A: +2, B: +3  → Result: 5
B: +3, A: +2  → Result: 5 (same!)
```

### 2. Associativity
Grouping doesn't matter:
```
(+1 + +2) + +3 = +1 + (+2 + +3) = 6
```

### 3. Idempotency (per actor)
Same operation from same actor applied multiple times = applied once:
```
Actor A: +1 (applied twice due to network) → Result: +1 (not +2)
```

### 4. Convergence
All replicas eventually reach the same value when they've seen all operations.

## When to Use CRDT Counter

✅ **Use CRDT Counter when:**
- Multiple users can modify the same counter
- Offline editing is required
- Increments/decrements must not be lost
- Examples: vote counts, likes, view counts, inventory

❌ **Don't need CRDT Counter when:**
- Only one writer
- Always online, sequential updates
- Last-write-wins is acceptable

## In This Project

The counter demonstrates CRDT principles:
- Browser can increment while offline
- CLI can increment concurrently
- All increments merge correctly
- Educational example of CRDTs in action

## Further Reading

- [Automerge Counter docs](https://docs.rs/autosurgeon/latest/autosurgeon/struct.Counter.html)
- [CRDT basics](https://crdt.tech/)
- [Automerge paper](https://arxiv.org/abs/1608.03960)
- [Conflict-free Replicated Data Types](https://hal.inria.fr/inria-00609399v1/document)