# HANDOFF: Proper TypeScript Types for Automerge ImmutableString

## Problem Statement

We've been using a `getString()` helper function to paper over type mismatches between Rust's `String` fields (which serialize as `ImmutableString` in Automerge) and JavaScript's expectations. This works, but it's not using Automerge's type system as intended.

## Current State

### What Works

The application successfully syncs data between Rust CLI and React frontend. All todo operations (create, toggle, delete) work across both clients.

### The `getString()` Workaround

We currently use this helper everywhere:

```typescript
const getString = (value: unknown): string => {
  if (typeof value === "string") return value;
  if (typeof value === "object" && value !== null && "val" in value) {
    return (value as { val: string }).val;
  }
  return "";
};
```

This handles both:
- Plain JavaScript `string` values (collaborative text)
- `ImmutableString` objects with structure `{val: "string"}` (atomic strings from Rust)

### Where We Use `getString()`

Currently applied to:
- `todo.id` - for matching/comparing IDs
- `todo.text` - when rendering todo text
- `tag` - when rendering/comparing tags
- `metadata.title` - when rendering document title
- `name` (collaborators - removed) - when rendering names

## Investigation Goals

### Primary Question

**Does Automerge-Repo provide proper TypeScript types for `ImmutableString`?**

According to the docs, JavaScript should represent non-collaborative strings using `ImmutableString`:

```typescript
import * as A from "@automerge/automerge";
doc.atomicStringValue = new A.ImmutableString("immutable");
```

But our TypeScript interfaces define everything as plain `string`:

```typescript
interface TodoItem {
  id: string;
  text: string;
  completed: boolean;
}
```

### Questions to Answer

1. **Type Definition**: Is there a proper `ImmutableString` type exported from `@automerge/automerge-repo` or `@automerge/react`?

2. **Type Guards**: Should we use type guards instead of `getString()`? Something like:
   ```typescript
   if (value instanceof ImmutableString) {
     return value.val;
   }
   ```

3. **Interface Definition**: Should our TypeScript interfaces distinguish between collaborative and non-collaborative strings?
   ```typescript
   interface TodoItem {
     id: ImmutableString;  // Non-collaborative
     text: string;         // Collaborative
     completed: boolean;
   }
   ```

4. **useDocument Hook**: Does `useDocument` provide proper typing for documents containing `ImmutableString` values?

5. **Round-tripping**: When we read an `ImmutableString` from Rust, modify it in JavaScript, what type should we use?

## Recommended Investigation Steps

### 1. Check Type Exports

```typescript
import * as Automerge from "@automerge/automerge";
import * as AutomergeRepo from "@automerge/automerge-repo";

// What's exported?
console.log("ImmutableString" in Automerge);
console.log(typeof Automerge.ImmutableString);
```

### 2. Test Runtime Behavior

Create a test document with both string types:

```typescript
const handle = repo.create({
  collaborativeText: "hello",
  atomicString: new Automerge.ImmutableString("world"),
});

console.log(handle.docSync());
// What do we actually see at runtime?
```

### 3. Check TypeScript Definitions

Look at `node_modules/@automerge/automerge/index.d.ts` and related files:
- Is `ImmutableString` exported as a class/type?
- Are there utility types for handling it?
- What does the `change` callback expect?

### 4. Test Cross-Platform Round-Trip

1. Rust creates document with `String` field (becomes `ImmutableString`)
2. JavaScript reads it - what type is it?
3. JavaScript modifies it - should we wrap in `new ImmutableString()`?
4. Does modification work without `getString()` if we use proper types?

## Known Issues to Document

### Issue 1: Type Mismatch on `todo.id`

**Problem**: Rust `String` fields serialize as `{val: "string"}` objects, but TypeScript expects `string`.

**Current Solution**: Use `getString()` wrapper everywhere.

**Better Solution**: Use proper Automerge types (if they exist).

### Issue 2: Collaborative vs Non-Collaborative Strings

**Problem**: Our schema doesn't distinguish between:
- `todo.text` - should be collaborative (character-by-character merge)
- `todo.id` - should be atomic (replace on conflict)

**Current State**: Both typed as `string`, relies on Rust-side serialization.

**Question**: Should we explicitly use `ImmutableString` in TypeScript for atomic strings?

## Success Criteria

A successful investigation will:

1. ✅ Eliminate all `getString()` calls
2. ✅ Use proper Automerge TypeScript types throughout
3. ✅ Maintain cross-platform compatibility (Rust ↔ JavaScript)
4. ✅ Clearly document which fields are collaborative vs atomic
5. ✅ Provide simpler, more maintainable code

## If Types Don't Exist...

If Automerge doesn't provide proper TypeScript types for `ImmutableString`, we should:

1. **Document the gap** - Create minimal reproducible example
2. **File issue upstream** - Ask Automerge maintainers about intended usage
3. **Propose solution** - Either:
   - Add types to Automerge
   - Document the `getString()` pattern as canonical
   - Create a community package with proper types

## Files to Review

- `frontend/src/App.tsx` - All `getString()` usage
- `cli/src/bin/repo_client.rs` - Rust type definitions
- `node_modules/@automerge/automerge/index.d.ts` - TypeScript definitions
- Automerge docs on ImmutableString (already fetched)

## References

From Automerge docs:

> There are two representations for strings. Plain old javascript `string`s represent collaborative text. This means that you should modify these strings using `Automerge.splice` or `Automerge.updateText`, this will ensure that your changes merge well with concurrent changes. On the other hand, non-collaborative text is represented using `ImmutableString`, which you create using `new Automerge.ImmutableString`.

This clearly states the intended usage. Our task is to implement it properly in TypeScript.

## Next Actions

1. Create test file to explore runtime behavior
2. Examine TypeScript definitions in detail
3. Test proper type usage patterns
4. Document findings (update this file)
5. Either: Implement proper types OR create reproducible issue for Automerge team

## Open Questions

- Does `@automerge/automerge-repo-react-hooks` handle `ImmutableString` transparently?
- Should `useDocument` automatically unwrap `ImmutableString` to plain strings?
- Is our current approach (Rust String → ImmutableString → getString()) the intended pattern?
- Would using `Text` everywhere (as we did earlier) be simpler/better?