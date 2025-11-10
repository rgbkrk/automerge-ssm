# Autosurgeon Migration Guide

## Overview

The CLI has been refactored to use `autosurgeon` instead of raw `automerge` types. This provides type-safe, ergonomic document handling with automatic serialization/deserialization.

## What Changed

### Before: Raw Automerge API

Previously, we manually used `automerge` APIs like:
- `doc.get()` with pattern matching on `Value` enums
- `doc.transact()` with manual `tx.put()` calls
- Manual type conversion for `Text` objects vs scalar strings
- Manual iteration over lists with type checking

### After: Autosurgeon Derive Macros

Now we use:
- `#[derive(Reconcile, Hydrate)]` on our document struct
- `autosurgeon::hydrate()` to read documents into Rust structs
- `autosurgeon::reconcile()` to write Rust structs back to documents
- Automatic handling of CRDT types like `Counter`

## Code Comparison

### Before: Manual Automerge API (~80 lines)

```rust
fn get_counter(doc: &automerge::Automerge) -> i64 {
    match doc.get(automerge::ROOT, "counter") {
        Ok(Some((automerge::Value::Scalar(s), _))) => s.to_i64().unwrap_or(0),
        _ => 0,
    }
}

fn get_notes(doc: &automerge::Automerge) -> Result<String> {
    match doc.get(automerge::ROOT, "notes") {
        Ok(Some((automerge::Value::Scalar(s), _))) => {
            Ok(s.to_str().map(|s| s.to_string()).unwrap_or_default())
        }
        Ok(Some((automerge::Value::Object(ObjType::Text), obj_id))) => {
            doc.text(&obj_id).context("Failed to read text object")
        }
        Ok(None) => Ok(String::new()),
        Ok(Some((val, _))) => {
            anyhow::bail!("Unexpected type for notes field: {:?}", val)
        }
        Err(e) => Err(e).context("Failed to get notes field"),
    }
}

fn increment_counter(doc_handle: &DocHandle) {
    doc_handle.with_document(|doc| {
        let current = get_counter(doc);
        doc.transact(|tx| {
            tx.put(automerge::ROOT, "counter", current + 1)?;
            Ok::<_, automerge::AutomergeError>(())
        }).expect("Failed to increment counter");
    });
}
```

### After: Autosurgeon (~10 lines)

```rust
#[derive(Debug, Clone, Default, Reconcile, Hydrate)]
struct Doc {
    counter: autosurgeon::Counter,
    notes: String,
    collaborators: Vec<String>,
}

fn increment_counter(doc_handle: &DocHandle) {
    doc_handle.with_document(|doc| {
        let mut state: Doc = hydrate(doc).unwrap_or_default();
        state.counter.increment(1);
        doc.transact(|tx| reconcile(tx, &state)).unwrap();
    });
}
```

## Benefits

1. **Type Safety**: Compile-time guarantees about document structure
2. **Less Boilerplate**: ~150 lines of manual serialization code eliminated
3. **Ergonomic**: Work with native Rust types, not `Value` enums
4. **Maintainable**: Schema is self-documenting through the struct definition
5. **Smart Merging**: Autosurgeon automatically generates optimal diffs

## Document Schema

```rust
#[derive(Debug, Clone, Default, Reconcile, Hydrate)]
struct Doc {
    counter: autosurgeon::Counter,  // CRDT counter type
    notes: String,                   // Automatically uses Text for JS compatibility
    collaborators: Vec<String>,      // Automatically uses List
}
```

## Usage Pattern

### Reading from Document

```rust
// Hydrate Rust struct from automerge document
let state: Doc = hydrate(doc).unwrap_or_default();
println!("Counter: {}", state.counter.value());
```

### Writing to Document

```rust
// Modify Rust struct
state.counter.increment(1);
state.notes = "Updated notes".to_string();

// Reconcile changes back to document
doc.transact(|tx| {
    reconcile(tx, &state)
})?;
```

## Counter Type

The `autosurgeon::Counter` type wraps `i64` and provides CRDT counter semantics:

```rust
// Create counter
let counter = Counter::with_value(42);

// Increment (can be negative)
counter.increment(5);
counter.increment(-2);

// Read value
let value: i64 = counter.value();
```

Counters merge correctly across concurrent edits - increments from different replicas sum up rather than conflicting.

## Cross-Platform Compatibility

Autosurgeon handles JS/Rust type compatibility automatically:

- Rust `String` ↔ JS `Automerge.Text`
- Rust `Vec<T>` ↔ JS arrays
- Rust `Counter` ↔ JS counters
- Rust structs ↔ JS objects

## Advanced: Custom Serialization

For types not handled by autosurgeon, use attributes:

```rust
#[derive(Reconcile, Hydrate)]
struct File {
    // Use custom hydrate/reconcile functions
    #[autosurgeon(with = "path_handlers")]
    path: std::path::PathBuf,
}

mod path_handlers {
    pub fn hydrate<D: ReadDoc>(
        doc: &D, obj: &ObjId, prop: Prop
    ) -> Result<PathBuf, HydrateError> {
        let s = String::hydrate(doc, obj, prop)?;
        Ok(PathBuf::from(s))
    }
    
    pub fn reconcile<R: Reconciler>(
        path: &PathBuf, reconciler: R
    ) -> Result<(), R::Error> {
        reconciler.str(path.display().to_string())
    }
}
```

## Migration Checklist

If migrating your own code:

1. Add `autosurgeon` to `Cargo.toml`
2. Define document schema with `#[derive(Reconcile, Hydrate)]`
3. Replace `doc.get()` calls with `hydrate(doc)`
4. Replace manual `tx.put()` calls with `reconcile(tx, &value)`
5. Use `Counter` type for counter fields
6. Remove manual type checking code

## References

- [autosurgeon docs](https://docs.rs/autosurgeon/)
- [automerge docs](https://docs.rs/automerge/)
- [CRDT concepts](https://crdt.tech/)