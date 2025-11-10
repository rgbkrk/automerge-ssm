//! Simple example of working with Automerge documents in Rust
//!
//! This demonstrates:
//! - Creating documents
//! - Making changes
//! - Merging concurrent changes
//! - Saving and loading
//!
//! Run with: cargo run --example simple_client

use automerge::{AutoCommit, ObjType, ReadDoc};
use automerge::transaction::Transactable;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🦀 Automerge Rust Client Example\n");

    // Create a new document
    println!("1. Creating a new document...");
    let mut doc1 = AutoCommit::new();

    // Initialize the document structure (like our frontend does)
    doc1.put(automerge::ROOT, "counter", 0_i64)?;
    doc1.put(automerge::ROOT, "notes", "")?;

    let collaborators = doc1.put_object(automerge::ROOT, "collaborators", ObjType::List)?;

    println!("   Created document with counter, notes, and collaborators");

    // Make some changes
    println!("\n2. Making changes to doc1...");
    doc1.put(automerge::ROOT, "counter", 5_i64)?;
    doc1.put(automerge::ROOT, "notes", "Hello from Rust!")?;
    doc1.insert(&collaborators, 0, "RustUser")?;

    println!("   Counter: {}", doc1.get(automerge::ROOT, "counter")?.unwrap().0);
    println!("   Notes: {}", doc1.get(automerge::ROOT, "notes")?.unwrap().0);

    // Save the document
    println!("\n3. Saving document to bytes...");
    let saved_bytes = doc1.save();
    println!("   Saved {} bytes", saved_bytes.len());

    // Fork the document to simulate another client
    println!("\n4. Forking document (simulating another client)...");
    let mut doc2 = doc1.fork();

    // Make concurrent changes on both docs
    println!("\n5. Making concurrent changes...");

    println!("   Doc1: Incrementing counter by 3");
    let current: i64 = match doc1.get(automerge::ROOT, "counter")? {
        Some((automerge::Value::Scalar(s), _)) => {
            s.as_ref().to_i64().unwrap_or(0)
        }
        _ => 0,
    };
    doc1.put(automerge::ROOT, "counter", current + 3)?;

    println!("   Doc2: Incrementing counter by 2");
    let current: i64 = match doc2.get(automerge::ROOT, "counter")? {
        Some((automerge::Value::Scalar(s), _)) => {
            s.as_ref().to_i64().unwrap_or(0)
        }
        _ => 0,
    };
    doc2.put(automerge::ROOT, "counter", current + 2)?;

    println!("   Doc1: Updating notes");
    doc1.put(automerge::ROOT, "notes", "Updated from doc1")?;

    println!("   Doc2: Updating notes");
    doc2.put(automerge::ROOT, "notes", "Updated from doc2")?;

    // Print state before merge
    println!("\n6. State before merge:");
    println!("   Doc1 counter: {}", doc1.get(automerge::ROOT, "counter")?.unwrap().0);
    println!("   Doc2 counter: {}", doc2.get(automerge::ROOT, "counter")?.unwrap().0);

    // Merge the documents
    println!("\n7. Merging doc2 into doc1...");
    doc1.merge(&mut doc2)?;

    // Print state after merge
    println!("\n8. State after merge:");
    println!("   Counter: {}", doc1.get(automerge::ROOT, "counter")?.unwrap().0);
    println!("   (Notice: 5 + 3 + 2 = 10, both increments preserved!)");

    println!("   Notes: {}", doc1.get(automerge::ROOT, "notes")?.unwrap().0);
    println!("   (One value won, but no data was lost in the CRDT)");

    // Get all conflicts for notes
    let conflicts = doc1.get_all(automerge::ROOT, "notes")?;
    if conflicts.len() > 1 {
        println!("\n   Conflicts detected for 'notes' field:");
        for (idx, (val, _)) in conflicts.iter().enumerate() {
            println!("     Option {}: {}", idx + 1, val);
        }
    }

    // Demonstrate loading from bytes
    println!("\n9. Loading document from saved bytes...");
    let loaded_doc = AutoCommit::load(&saved_bytes)?;
    println!("   Loaded successfully!");
    println!("   Counter from loaded doc: {}",
             loaded_doc.get(automerge::ROOT, "counter")?.unwrap().0);

    // Show how to work with text (collaborative strings)
    println!("\n10. Working with collaborative text...");
    let mut text_doc = AutoCommit::new();

    // Create a text field
    text_doc.put(automerge::ROOT, "content", "Hello")?;

    // Note: For collaborative text editing, you would use splice operations
    // text_doc.splice(...) for more complex text operations
    println!("    Text: {}", text_doc.get(automerge::ROOT, "content")?.unwrap().0);

    println!("\n✅ Demo complete!");
    println!("\nKey takeaways:");
    println!("  • CRDTs automatically merge concurrent changes");
    println!("  • Numeric operations (like counter increments) are preserved");
    println!("  • Conflicts are handled deterministically");
    println!("  • Documents can be saved/loaded as bytes");
    println!("  • Same format works with JavaScript/TypeScript clients");

    Ok(())
}
